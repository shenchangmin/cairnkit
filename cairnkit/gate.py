"""Stage admission gate — the hard, Python-enforced transition check (M2).

``check(next_stage, state, config)`` answers: *"may the run move from its current stage
into ``next_stage``?"* It is path-mode agnostic: a transition is allowed only when

  - the run is not blocked (a retry cap was not exceeded),
  - if the current stage is a CLARIFY pause, it has been approved, and
  - the artifact the current stage was supposed to produce exists and is non-empty.

This is the discipline that does not rely on model goodwill (CLAUDE.md §2).
"""

from __future__ import annotations

from dataclasses import dataclass

from cairnkit import stages
from cairnkit.config import Config, State

# Re-exported for callers that referenced gate.STAGE_ARTIFACT historically.
STAGE_ARTIFACT = stages.STAGE_ARTIFACT


@dataclass(frozen=True)
class GateResult:
    ok: bool
    stage: str
    missing: tuple[str, ...]
    message: str


def check(next_stage: str, state: State, config: Config) -> GateResult:
    """Validate the transition current → next_stage (see module docstring)."""
    current = state.stage

    if state.blocked_reason:
        return GateResult(False, next_stage, (), f"run is blocked: {state.blocked_reason}")

    # Entering the very first stage only requires the project to be initialised.
    if current == "INIT" and next_stage == "INTENT_GATE":
        if (config.root / "cairnkit.yaml").exists():
            return GateResult(True, next_stage, (), "ok")
        return GateResult(
            False, next_stage, ("cairnkit.yaml",),
            "cairnkit.yaml missing — run /team-init first.",
        )

    # Leaving a CLARIFY pause requires approval.
    if stages.is_clarify(current) and state.pending_clarify is not None:
        return GateResult(
            False, next_stage, (),
            f"CLARIFY not yet approved (pending: {state.pending_clarify}). "
            "Approve with `cairnkit state approve-clarify`.",
        )

    # The current stage's produced artifact must exist and be non-empty.
    produced = stages.STAGE_ARTIFACT.get(current)
    if produced:
        path = config.run_dir(state.run_id) / produced
        if not path.exists() or path.stat().st_size == 0:
            rel = str(path.relative_to(config.root))
            return GateResult(
                False, next_stage, (rel,),
                f"Cannot leave {current}: its artifact is missing/empty: {rel}",
            )

    return GateResult(True, next_stage, (), "ok")
