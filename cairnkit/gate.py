"""Stage admission gate — the hard, Python-enforced precondition check.

``check(stage, ...)`` answers: *"are the preconditions to ENTER ``stage`` satisfied?"*
A transition is refused (by ``state.advance``) unless the upstream artifact a stage
depends on exists and is non-empty, and — for stages downstream of a CLARIFY pause —
the async approval has been granted. This is the discipline that does not rely on
model goodwill (CLAUDE.md §2).

B1 scope: the minimal stage set only. Retry/blocked logic (BUILD_VERIFY/E2E_VERIFY)
is a placeholder added in B3.
"""

from __future__ import annotations

from dataclasses import dataclass

from cairnkit.config import Config, State

# Artifact a stage *produces* (written by its role agent before advancing).
STAGE_ARTIFACT = {
    "ANALYSE_PRODUCT": "01-product.md",
    "ARCHITECT_BACKEND": "03-arch.md",
}

# Upstream artifacts required to ENTER a given stage.
ENTRY_ARTIFACTS: dict[str, tuple[str, ...]] = {
    "CLARIFY_PRODUCT": ("01-product.md",),
    "ARCHITECT_BACKEND": ("01-product.md",),
    "DONE": ("03-arch.md",),
}

# Stages that may only be entered once the preceding CLARIFY pause is approved.
CLARIFY_REQUIRED = {"ARCHITECT_BACKEND"}

# Stages that require a predecessor stage to be completed (it produces no file artifact,
# so completion is proven by history). Satisfied when the predecessor is in history OR is
# the current stage being left (advance appends to history only after the gate passes).
HISTORY_REQUIRED = {"ANALYSE_PRODUCT": "INIT"}


@dataclass(frozen=True)
class GateResult:
    ok: bool
    stage: str
    missing: tuple[str, ...]
    message: str


def check(stage: str, state: State, config: Config) -> GateResult:
    """Validate the preconditions to enter ``stage`` (see module docstring)."""
    if stage == "INIT":
        if (config.root / "cairnkit.yaml").exists():
            return GateResult(True, stage, (), "ok")
        return GateResult(
            False, stage, ("cairnkit.yaml",),
            "cairnkit.yaml missing — run /team-init first.",
        )

    # Predecessor stage must be completed (in history or the stage being left now).
    prev = HISTORY_REQUIRED.get(stage)
    if prev is not None and prev != state.stage and prev not in state.history:
        return GateResult(
            False, stage, (),
            f"Cannot enter {stage}: predecessor stage {prev} not completed.",
        )

    # CLARIFY approval must be granted before entering a clarify-gated stage.
    if stage in CLARIFY_REQUIRED and state.pending_clarify is not None:
        return GateResult(
            False, stage, (),
            f"CLARIFY not yet approved (pending: {state.pending_clarify}). "
            "Approve with `cairnkit state approve-clarify`.",
        )

    run_dir = config.run_dir(state.run_id)
    missing: list[str] = []
    for fname in ENTRY_ARTIFACTS.get(stage, ()):
        path = run_dir / fname
        if not path.exists() or path.stat().st_size == 0:
            missing.append(str(path.relative_to(config.root)))

    if missing:
        return GateResult(
            False, stage, tuple(missing),
            f"Cannot enter {stage}: missing/empty upstream artifact(s): "
            + ", ".join(missing),
        )
    return GateResult(True, stage, (), "ok")
