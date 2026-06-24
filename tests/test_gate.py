"""T4 · gate.py — admission gate (RED first)."""

from __future__ import annotations

from pathlib import Path

from cairnkit import config as cfg
from cairnkit import gate
from tests.conftest import write_artifact

RUN = "2026-06-24-demo"


def _state(project: Path, stage: str, **over) -> cfg.State:
    c = cfg.load_config(project)
    st = cfg.init_state(c, RUN)
    return st.with_(stage=stage, **over)


def test_enter_init_ok_when_config_present(project: Path) -> None:
    c = cfg.load_config(project)
    st = cfg.init_state(c, RUN)
    res = gate.check("INIT", st, c)
    assert res.ok


def test_enter_analyse_product_from_init_ok(project: Path) -> None:
    # advancing from INIT: INIT is the current stage being left → precondition met
    c = cfg.load_config(project)
    st = _state(project, "INIT")
    res = gate.check("ANALYSE_PRODUCT", st, c)
    assert res.ok


def test_enter_analyse_product_without_init_completed_fails(project: Path) -> None:
    # direct gate check on a state that never completed INIT → refused
    c = cfg.load_config(project)
    st = _state(project, "DONE", history=())
    res = gate.check("ANALYSE_PRODUCT", st, c)
    assert not res.ok
    assert "INIT" in res.message


def test_enter_clarify_missing_product_artifact_fails(project: Path) -> None:
    c = cfg.load_config(project)
    st = _state(project, "ANALYSE_PRODUCT")
    res = gate.check("CLARIFY_PRODUCT", st, c)
    assert not res.ok
    assert any("01-product.md" in m for m in res.missing)


def test_enter_clarify_with_product_artifact_ok(project: Path) -> None:
    c = cfg.load_config(project)
    write_artifact(project, RUN, "01-product.md")
    st = _state(project, "ANALYSE_PRODUCT")
    res = gate.check("CLARIFY_PRODUCT", st, c)
    assert res.ok


def test_empty_artifact_file_is_rejected(project: Path) -> None:
    c = cfg.load_config(project)
    write_artifact(project, RUN, "01-product.md", body="")  # 0 bytes
    st = _state(project, "ANALYSE_PRODUCT")
    res = gate.check("CLARIFY_PRODUCT", st, c)
    assert not res.ok


def test_enter_architect_blocked_until_clarify_approved(project: Path) -> None:
    c = cfg.load_config(project)
    write_artifact(project, RUN, "01-product.md")
    st = _state(project, "CLARIFY_PRODUCT", pending_clarify="Awaiting approval for ARCHITECT_BACKEND")
    res = gate.check("ARCHITECT_BACKEND", st, c)
    assert not res.ok
    assert "clarif" in res.message.lower()


def test_enter_architect_ok_when_approved_and_artifact_present(project: Path) -> None:
    c = cfg.load_config(project)
    write_artifact(project, RUN, "01-product.md")
    st = _state(project, "CLARIFY_PRODUCT", pending_clarify=None)
    res = gate.check("ARCHITECT_BACKEND", st, c)
    assert res.ok


def test_enter_done_needs_arch_artifact(project: Path) -> None:
    c = cfg.load_config(project)
    st = _state(project, "ARCHITECT_BACKEND")
    res = gate.check("DONE", st, c)
    assert not res.ok
    write_artifact(project, RUN, "03-arch.md")
    res2 = gate.check("DONE", st, c)
    assert res2.ok
