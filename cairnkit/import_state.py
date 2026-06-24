"""Cold-start import progress (M8) — resumable `/flow-import` pipeline state.

`/flow-import` runs a 3-agent pipeline (doc-collector → codebase-profiler → knowledge-builder)
to make an existing project's implicit knowledge explicit. Like the main workflow, its progress
is a file (`docs/knowledge-import/import-state.json`) so a long import can resume after a crash.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from cairnkit.errors import StateError, UsageError

IMPORT_STEPS = ("doc-collect", "codebase-profile", "knowledge-build", "done")


@dataclass(frozen=True)
class ImportState:
    step: str
    done: tuple[str, ...]
    updated_at: str


def import_path(root: Path) -> Path:
    return root / "docs" / "knowledge-import" / "import-state.json"


def _now() -> str:
    return datetime.now().isoformat(timespec="seconds")


def init_import(root: Path) -> ImportState:
    path = import_path(root)
    if path.exists():
        raise UsageError("an import is already in progress; use `import show`/`import advance`.")
    state = ImportState(step=IMPORT_STEPS[0], done=(), updated_at=_now())
    _save(path, state)
    return state


def load_import(root: Path) -> ImportState:
    path = import_path(root)
    if not path.exists():
        raise StateError("no import in progress; start one with `import init`.")
    data = json.loads(path.read_text(encoding="utf-8"))
    return ImportState(step=data["step"], done=tuple(data["done"]), updated_at=data["updated_at"])


def advance_import(root: Path) -> ImportState:
    state = load_import(root)
    if state.step == "done":
        raise StateError("import already complete.")
    idx = IMPORT_STEPS.index(state.step)
    new = ImportState(
        step=IMPORT_STEPS[idx + 1],
        done=state.done + (state.step,),
        updated_at=_now(),
    )
    _save(import_path(root), new)
    return new


def _save(path: Path, state: ImportState) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps({"step": state.step, "done": list(state.done), "updated_at": state.updated_at}, indent=2),
        encoding="utf-8",
    )
