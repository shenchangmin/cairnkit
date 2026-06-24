"""B3 · state.py — the full 16-stage machine, path modes, retry/block."""

from __future__ import annotations

from pathlib import Path

import pytest

from cairnkit import config as cfg
from cairnkit import stages
from cairnkit import state as sm
from cairnkit.errors import GateError, StateError, UsageError
from tests.conftest import advance_to, write_artifact

RUN = "2026-06-24-demo"


def _fresh(project: Path) -> tuple[cfg.Config, Path]:
    c = cfg.load_config(project)
    cfg.init_state(c, RUN)
    return c, c.state_path


def test_advance_from_init_to_intent_gate(project: Path) -> None:
    c, sp = _fresh(project)
    new = sm.advance(sp, c)
    assert new.stage == "INTENT_GATE"
    assert "INIT" in new.history


def test_advance_blocked_when_upstream_artifact_missing(project: Path) -> None:
    c, sp = _fresh(project)
    advance_to(c, "ANALYSE_PRODUCT")
    # ANALYSE_PRODUCT produces 01-product.md; not written -> cannot advance out
    with pytest.raises(GateError):
        sm.advance(sp, c)
    assert cfg.load_state(sp).stage == "ANALYSE_PRODUCT"


def test_advance_into_clarify_sets_pending(project: Path) -> None:
    c, sp = _fresh(project)
    advance_to(c, "ANALYSE_PRODUCT")
    write_artifact(project, RUN, "01-product.md")
    new = sm.advance(sp, c)  # -> CLARIFY_PRODUCT
    assert new.stage == "CLARIFY_PRODUCT"
    assert new.pending_clarify is not None


def test_clarify_blocks_until_approved(project: Path) -> None:
    c, sp = _fresh(project)
    advance_to(c, "CLARIFY_PRODUCT")
    with pytest.raises(GateError):
        sm.advance(sp, c)  # not approved
    sm.approve_clarify(sp)
    new = sm.advance(sp, c)
    assert new.stage == "ANALYSE_TECH"


def test_full_run_reaches_done(project: Path) -> None:
    c, _ = _fresh(project)
    st = advance_to(c, "DONE")
    assert st.stage == "DONE"
    assert st.history[-1] == "ARCHIVE"


def test_advance_past_done_raises(project: Path) -> None:
    c, sp = _fresh(project)
    sm.set_stage(sp, "DONE", c)
    with pytest.raises(StateError):
        sm.advance(sp, c)


def test_lite_mode_skips_frontend(project: Path) -> None:
    c, sp = _fresh(project)
    advance_to(c, "INTENT_GATE")
    sm.set_path_mode(sp, "lite")
    st = advance_to(c, "DONE")
    # frontend + visual stages never appear in history under lite
    assert "ARCHITECT_FRONTEND" not in st.history
    assert "VISUAL_REVIEW" not in st.history
    assert "ARCHITECT_BACKEND" in st.history


def test_single_mode_minimal_path(project: Path) -> None:
    c, sp = _fresh(project)
    advance_to(c, "INTENT_GATE")
    sm.set_path_mode(sp, "single")
    st = advance_to(c, "DONE")
    assert "ANALYSE_PRODUCT" not in st.history  # skipped in single
    assert "IMPLEMENT" in st.history
    assert "TEST" in st.history


def test_set_path_mode_rejects_unknown(project: Path) -> None:
    c, sp = _fresh(project)
    with pytest.raises(UsageError):
        sm.set_path_mode(sp, "turbo")


def test_record_failure_increments_then_blocks(project: Path) -> None:
    c, sp = _fresh(project)
    for i in range(stages.RETRY_CAP - 1):
        st = sm.record_failure(sp, "BUILD_VERIFY")
        assert st.blocked_reason is None
        assert st.retries["BUILD_VERIFY"] == i + 1
    st = sm.record_failure(sp, "BUILD_VERIFY")  # hits cap
    assert st.blocked_reason is not None


def test_blocked_run_cannot_advance(project: Path) -> None:
    c, sp = _fresh(project)
    advance_to(c, "INTENT_GATE")
    sm.set_path_mode(sp, "single")
    advance_to(c, "BUILD_VERIFY")
    for _ in range(stages.RETRY_CAP):
        sm.record_failure(sp, "BUILD_VERIFY")
    write_artifact(project, RUN, "06-build.md")
    with pytest.raises(GateError):
        sm.advance(sp, c)  # blocked
    unblocked = sm.unblock(sp)
    assert unblocked.retries == {}  # retries reset on unblock
    new = sm.advance(sp, c)  # unblocked -> proceeds
    assert new.stage == "TEST"


def test_record_failure_rejects_non_verify_stage(project: Path) -> None:
    c, sp = _fresh(project)
    with pytest.raises(UsageError):
        sm.record_failure(sp, "IMPLEMENT")


def test_resume_reports_paused(project: Path) -> None:
    c, sp = _fresh(project)
    advance_to(c, "CLARIFY_PRODUCT")
    st = sm.resume(sp)
    assert st.stage == "CLARIFY_PRODUCT"
    assert sm.is_paused(st) is True


def test_set_stage_illegal_enum_rejected(project: Path) -> None:
    c, sp = _fresh(project)
    with pytest.raises(UsageError):
        sm.set_stage(sp, "NONSENSE", c)
