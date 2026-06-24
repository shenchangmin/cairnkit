"""Data models + file persistence for the cairnkit state machine.

This module owns the *data layer*: the immutable ``Config`` / ``State`` models and the
YAML read/write for ``cairnkit.yaml`` and ``.cairnkit/STATE.yaml``. The state-machine
*logic* lives in ``state.py``; the admission gate in ``gate.py``.

The file system is the single source of truth (see CLAUDE.md). All writes are atomic
(temp + os.replace) and order-preserving (ruamel.yaml round-trip).
"""

from __future__ import annotations

import os
from dataclasses import dataclass, replace
from datetime import datetime
from pathlib import Path
from types import MappingProxyType
from typing import Any, Mapping

from ruamel.yaml import YAML
from ruamel.yaml.error import YAMLError
from ruamel.yaml.representer import RoundTripRepresenter

from cairnkit.errors import ConfigError, StateCorruptError

# The valid stages (the enum). Transition logic (NEXT) lives in state.py.
STAGES = ("INIT", "ANALYSE_PRODUCT", "CLARIFY_PRODUCT", "ARCHITECT_BACKEND", "DONE")

# Canonical STATE.yaml field order (kept stable for readable diffs).
_STATE_FIELDS = (
    "run_id", "stage", "path_mode", "history",
    "artifacts", "retries", "pending_clarify", "updated_at",
)

_yaml = YAML()  # round-trip mode: preserves order
_yaml.default_flow_style = False
# Serialize None as an explicit `null` (more readable in STATE.yaml diffs).
_yaml.Representer = RoundTripRepresenter
_yaml.representer.add_representer(
    type(None),
    lambda r, d: r.represent_scalar("tag:yaml.org,2002:null", "null"),
)


def _now() -> str:
    return datetime.now().isoformat(timespec="seconds")


# --- models ----------------------------------------------------------------

@dataclass(frozen=True)
class Repo:
    name: str
    path: str


@dataclass(frozen=True)
class Config:
    project: str
    domain: str | None
    repos: tuple[Repo, ...]
    root: Path

    @property
    def state_path(self) -> Path:
        return self.root / ".cairnkit" / "STATE.yaml"

    def run_dir(self, run_id: str) -> Path:
        return self.root / "docs" / "workflows" / run_id


@dataclass(frozen=True)
class State:
    run_id: str
    stage: str
    path_mode: str
    history: tuple[str, ...]
    artifacts: Mapping[str, str]
    retries: Mapping[str, int]
    pending_clarify: str | None
    updated_at: str

    def __post_init__(self) -> None:
        # Make the mappings genuinely read-only: frozen=True stops field rebinding
        # but not in-place mutation of a dict. Wrap so `state.artifacts[k] = v` raises.
        object.__setattr__(self, "artifacts", MappingProxyType(dict(self.artifacts)))
        object.__setattr__(self, "retries", MappingProxyType(dict(self.retries)))

    def with_(self, **changes: Any) -> "State":
        """Return a new State with the given fields changed (immutability)."""
        return replace(self, **changes)


# --- config IO -------------------------------------------------------------

def load_config(root: Path) -> Config:
    """Read ``<root>/cairnkit.yaml``. Missing/invalid → ConfigError (run /team-init)."""
    path = root / "cairnkit.yaml"
    if not path.exists():
        raise ConfigError(
            f"cairnkit.yaml not found in {root}. Run /team-init to initialise the project."
        )
    try:
        data = _yaml.load(path.read_text(encoding="utf-8")) or {}
    except YAMLError as exc:
        raise ConfigError(f"cairnkit.yaml is not valid YAML: {exc}") from exc
    if "project" not in data:
        raise ConfigError("cairnkit.yaml missing required field: project")
    try:
        repos = tuple(
            Repo(name=str(r["name"]), path=str(r.get("path", ".")))
            for r in (data.get("repos") or [])
        )
    except (TypeError, KeyError, AttributeError) as exc:
        raise ConfigError(
            f"cairnkit.yaml has an invalid repos entry (each needs a 'name'): {exc}"
        ) from exc
    if not repos:  # single-repo default
        repos = (Repo(name=str(data["project"]), path="."),)
    return Config(
        project=str(data["project"]),
        domain=data.get("domain"),
        repos=repos,
        root=root,
    )


# --- state IO --------------------------------------------------------------

def init_state(config: Config, run_id: str) -> State:
    """Create a fresh STATE at stage INIT and persist it."""
    state = State(
        run_id=run_id,
        stage="INIT",
        path_mode="full",
        history=(),
        artifacts={},
        retries={},
        pending_clarify=None,
        updated_at=_now(),
    )
    save_state(config.state_path, state)
    return state


def load_state(state_path: Path) -> State:
    """Read STATE.yaml. Missing file / bad YAML / missing fields → StateCorruptError."""
    if not state_path.exists():
        raise StateCorruptError(
            f"STATE.yaml not found at {state_path}. Start a run with /flow-run."
        )
    try:
        data = _yaml.load(state_path.read_text(encoding="utf-8")) or {}
    except YAMLError as exc:
        raise StateCorruptError(f"STATE.yaml is not valid YAML: {exc}") from exc
    missing = [f for f in _STATE_FIELDS if f not in data]
    if missing:
        raise StateCorruptError(
            "STATE.yaml is missing required fields: "
            + ", ".join(missing)
            + f". Expected fields: {', '.join(_STATE_FIELDS)}."
        )
    if str(data["stage"]) not in STAGES:
        raise StateCorruptError(
            f"STATE.yaml has an unknown stage {data['stage']!r}. "
            f"Valid stages: {', '.join(STAGES)}."
        )
    return State(
        run_id=str(data["run_id"]),
        stage=str(data["stage"]),
        path_mode=str(data["path_mode"]),
        history=tuple(data["history"] or ()),
        artifacts=dict(data["artifacts"] or {}),
        retries=dict(data["retries"] or {}),
        pending_clarify=data["pending_clarify"],
        updated_at=str(data["updated_at"]),
    )


def save_state(state_path: Path, state: State) -> None:
    """Persist STATE atomically, re-stamping updated_at; fields in canonical order."""
    state_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "run_id": state.run_id,
        "stage": state.stage,
        "path_mode": state.path_mode,
        "history": list(state.history),
        "artifacts": dict(state.artifacts),
        "retries": dict(state.retries),
        "pending_clarify": state.pending_clarify,
        "updated_at": _now(),  # Python owns the timestamp
    }
    tmp = state_path.with_suffix(state_path.suffix + ".tmp")
    with tmp.open("w", encoding="utf-8") as fh:
        _yaml.dump(payload, fh)
    os.replace(tmp, state_path)  # atomic
