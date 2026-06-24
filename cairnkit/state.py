"""The state machine — deterministic stage transitions over the file-as-state model.

The model never holds workflow state in its head. State lives in ``.cairnkit/STATE.yaml``
and is mutated only here, so re-running a flow = read STATE and continue (crash-resume is
automatic, zero memory dependency). Transitions are one-step-only (no stage skipping) and
gated by ``gate.check`` (CLAUDE.md §2).

B1 scope: the minimal stage set INIT → ANALYSE_PRODUCT → CLARIFY_PRODUCT →
ARCHITECT_BACKEND → DONE.
"""

from __future__ import annotations

from pathlib import Path

from cairnkit import gate
from cairnkit.config import STAGES, Config, State, load_state, save_state
from cairnkit.errors import GateError, StateError, UsageError

NEXT: dict[str, str | None] = {
    "INIT": "ANALYSE_PRODUCT",
    "ANALYSE_PRODUCT": "CLARIFY_PRODUCT",
    "CLARIFY_PRODUCT": "ARCHITECT_BACKEND",
    "ARCHITECT_BACKEND": "DONE",
    "DONE": None,
}


def _is_clarify(stage: str) -> bool:
    return stage.startswith("CLARIFY")


def advance(state_path: Path, config: Config) -> State:
    """Transition current → NEXT[current].

    Verifies the admission gate for the *next* stage, then appends the current stage to
    history, records any artifact the current stage produced, and persists. Refuses
    (GateError) if the gate fails; raises StateError at the terminal stage. Never skips.
    """
    state = load_state(state_path)
    current = state.stage
    nxt = NEXT.get(current)
    if nxt is None:
        raise StateError(f"{current} is the terminal stage; nothing to advance to.")

    result = gate.check(nxt, state, config)
    if not result.ok:
        raise GateError(result.message, missing=result.missing)

    artifacts = dict(state.artifacts)
    produced = gate.STAGE_ARTIFACT.get(current)
    if produced:
        rel = config.run_dir(state.run_id) / produced
        artifacts[current] = str(rel.relative_to(config.root))

    # Entering a CLARIFY stage pauses the run for async approval.
    pending = f"Awaiting approval for {NEXT[nxt]}" if _is_clarify(nxt) else None

    new = state.with_(
        stage=nxt,
        history=state.history + (current,),
        artifacts=artifacts,
        pending_clarify=pending,
    )
    save_state(state_path, new)
    return new


def approve_clarify(state_path: Path) -> State:
    """Clear the CLARIFY pause so the downstream stage may be entered."""
    state = load_state(state_path)
    new = state.with_(pending_clarify=None)
    save_state(state_path, new)
    return new


def set_stage(state_path: Path, stage: str, config: Config) -> State:
    """Manual repair back-door: force the current stage (records history)."""
    if stage not in STAGES:
        raise UsageError(
            f"Unknown stage {stage!r}. Valid stages: {', '.join(STAGES)}."
        )
    state = load_state(state_path)
    new = state.with_(stage=stage, history=state.history + (state.stage,))
    save_state(state_path, new)
    return new


def resume(state_path: Path) -> State:
    """Return the state to continue from (pure file read, zero memory dependency)."""
    return load_state(state_path)


def show(state_path: Path) -> State:
    """Return the current state (for /flow-status)."""
    return load_state(state_path)


def is_paused(state: State) -> bool:
    """True when the run is parked at a CLARIFY pause awaiting approval."""
    return state.pending_clarify is not None
