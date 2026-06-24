"""The state machine — deterministic stage transitions over the file-as-state model (M2).

State lives in ``.cairnkit/STATE.yaml`` and is mutated only here, so re-running a flow =
read STATE and continue (crash-resume is automatic, zero memory dependency). Transitions
are one-step-only along the active path mode (IntentGate routes small work onto a shorter
path), gated by ``gate.check``. Verify stages loop on failure with a retry cap, then block.
"""

from __future__ import annotations

from pathlib import Path

from cairnkit import gate, stages
from cairnkit.config import STAGES, Config, State, load_state, save_state
from cairnkit.errors import GateError, StateError, UsageError


def advance(state_path: Path, config: Config) -> State:
    """Transition current → next active stage for the path mode.

    Verifies the admission gate, appends the current stage to history, records any artifact
    the current stage produced, auto-pauses on entering a CLARIFY stage, and persists.
    Refuses (GateError) on a failed gate; raises StateError at the terminal stage.
    """
    state = load_state(state_path)
    current = state.stage
    nxt = stages.next_stage(current, state.path_mode)
    if nxt is None:
        raise StateError(f"{current} is the terminal stage; nothing to advance to.")

    result = gate.check(nxt, state, config)
    if not result.ok:
        raise GateError(result.message, missing=result.missing)

    artifacts = dict(state.artifacts)
    produced = stages.STAGE_ARTIFACT.get(current)
    if produced:
        rel = config.run_dir(state.run_id) / produced
        artifacts[current] = str(rel.relative_to(config.root))

    # Entering a CLARIFY stage pauses the run for async approval.
    after = stages.next_stage(nxt, state.path_mode)
    pending = f"Awaiting approval before {after}" if stages.is_clarify(nxt) else None

    new = state.with_(
        stage=nxt,
        history=state.history + (current,),
        artifacts=artifacts,
        pending_clarify=pending,
    )
    save_state(state_path, new)
    return new


def record_failure(state_path: Path, stage: str) -> State:
    """A verify stage failed: bump its retry counter; block the run if the cap is exceeded."""
    if stage not in stages.RETRY_STAGES:
        raise UsageError(
            f"{stage} is not a retryable verify stage {tuple(stages.RETRY_STAGES)}."
        )
    state = load_state(state_path)
    retries = dict(state.retries)
    retries[stage] = retries.get(stage, 0) + 1
    blocked = (
        f"{stage} failed {retries[stage]} times (cap {stages.RETRY_CAP})"
        if retries[stage] >= stages.RETRY_CAP
        else None
    )
    new = state.with_(retries=retries, blocked_reason=blocked)
    save_state(state_path, new)
    return new


def set_path_mode(state_path: Path, path_mode: str) -> State:
    """Record the IntentGate routing decision (full | lite | single)."""
    if path_mode not in stages.PATH_MODES:
        raise UsageError(
            f"Unknown path mode {path_mode!r}. Valid: {', '.join(stages.PATH_MODES)}."
        )
    state = load_state(state_path)
    new = state.with_(path_mode=path_mode)
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
        raise UsageError(f"Unknown stage {stage!r}. Valid stages: {', '.join(STAGES)}.")
    state = load_state(state_path)
    new = state.with_(stage=stage, history=state.history + (state.stage,))
    save_state(state_path, new)
    return new


def unblock(state_path: Path) -> State:
    """Clear a blocked run (human intervened); retry counters are reset for a fresh attempt."""
    state = load_state(state_path)
    new = state.with_(blocked_reason=None, retries={})
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
