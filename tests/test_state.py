"""T3 · state.py — state-machine transitions (RED first)."""

from __future__ import annotations

from pathlib import Path

import pytest

from cairnkit import config as cfg
from cairnkit import state as sm
from cairnkit.errors import GateError, StateError, UsageError
from tests.conftest import write_artifact

RUN = "2026-06-24-demo"


def _fresh(project: Path) -> tuple[cfg.Config, Path]:
    c = cfg.load_config(project)
    cfg.init_state(c, RUN)
    return c, c.state_path


def test_advance_from_init_to_analyse(project: Path) -> None:
    c, sp = _fresh(project)
    # Act
    new = sm.advance(sp, c)
    # Assert
    assert new.stage == "ANALYSE_PRODUCT"
    assert "INIT" in new.history


def test_advance_blocked_when_upstream_artifact_missing(project: Path) -> None:
    c, sp = _fresh(project)
    sm.advance(sp, c)  # INIT -> ANALYSE_PRODUCT
    # No 01-product.md written -> cannot enter CLARIFY_PRODUCT
    with pytest.raises(GateError):
        sm.advance(sp, c)
    # stage unchanged
    assert cfg.load_state(sp).stage == "ANALYSE_PRODUCT"


def test_advance_into_clarify_sets_pending(project: Path) -> None:
    c, sp = _fresh(project)
    sm.advance(sp, c)                       # -> ANALYSE_PRODUCT
    write_artifact(project, RUN, "01-product.md")
    new = sm.advance(sp, c)                 # -> CLARIFY_PRODUCT
    assert new.stage == "CLARIFY_PRODUCT"
    assert new.pending_clarify is not None  # paused for async approval


def test_advance_to_architect_blocked_until_approved(project: Path) -> None:
    c, sp = _fresh(project)
    sm.advance(sp, c)
    write_artifact(project, RUN, "01-product.md")
    sm.advance(sp, c)                       # -> CLARIFY_PRODUCT (pending set)
    with pytest.raises(GateError):
        sm.advance(sp, c)                   # blocked: not approved
    assert cfg.load_state(sp).stage == "CLARIFY_PRODUCT"


def test_advance_to_architect_after_approval(project: Path) -> None:
    c, sp = _fresh(project)
    sm.advance(sp, c)
    write_artifact(project, RUN, "01-product.md")
    sm.advance(sp, c)                       # -> CLARIFY_PRODUCT
    sm.approve_clarify(sp)
    new = sm.advance(sp, c)                 # -> ARCHITECT_BACKEND
    assert new.stage == "ARCHITECT_BACKEND"
    assert new.pending_clarify is None
    # product artifact recorded
    assert "ANALYSE_PRODUCT" in new.artifacts


def test_full_minimal_run_to_done(project: Path) -> None:
    c, sp = _fresh(project)
    sm.advance(sp, c)
    write_artifact(project, RUN, "01-product.md")
    sm.advance(sp, c)
    sm.approve_clarify(sp)
    sm.advance(sp, c)                       # -> ARCHITECT_BACKEND
    write_artifact(project, RUN, "03-arch.md")
    new = sm.advance(sp, c)                 # -> DONE
    assert new.stage == "DONE"
    assert new.history[-1] == "ARCHITECT_BACKEND"


def test_advance_past_done_raises(project: Path) -> None:
    c, sp = _fresh(project)
    sm.set_stage(sp, "DONE", c)
    with pytest.raises(StateError):
        sm.advance(sp, c)


def test_resume_reports_paused(project: Path) -> None:
    c, sp = _fresh(project)
    sm.advance(sp, c)
    write_artifact(project, RUN, "01-product.md")
    sm.advance(sp, c)                       # -> CLARIFY_PRODUCT (pending)
    st = sm.resume(sp)
    assert st.stage == "CLARIFY_PRODUCT"
    assert sm.is_paused(st) is True


def test_resume_not_paused(project: Path) -> None:
    c, sp = _fresh(project)
    sm.advance(sp, c)                       # -> ANALYSE_PRODUCT
    st = sm.resume(sp)
    assert sm.is_paused(st) is False


def test_set_stage_records_history(project: Path) -> None:
    c, sp = _fresh(project)
    new = sm.set_stage(sp, "ARCHITECT_BACKEND", c)
    assert new.stage == "ARCHITECT_BACKEND"
    assert "INIT" in new.history


def test_set_stage_illegal_enum_rejected(project: Path) -> None:
    c, sp = _fresh(project)
    with pytest.raises(UsageError):
        sm.set_stage(sp, "NONSENSE", c)
