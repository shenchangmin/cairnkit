"""B3 · gate.py — path-agnostic admission gate (verify current artifact + clarify + blocked)."""

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


def test_init_to_intent_gate_ok_when_config_present(project: Path) -> None:
    c = cfg.load_config(project)
    st = _state(project, "INIT")
    assert gate.check("INTENT_GATE", st, c).ok


def test_leaving_producing_stage_requires_its_artifact(project: Path) -> None:
    c = cfg.load_config(project)
    st = _state(project, "ANALYSE_PRODUCT")
    res = gate.check("CLARIFY_PRODUCT", st, c)
    assert not res.ok
    assert any("01-product.md" in m for m in res.missing)
    write_artifact(project, RUN, "01-product.md")
    assert gate.check("CLARIFY_PRODUCT", st, c).ok


def test_empty_artifact_rejected(project: Path) -> None:
    c = cfg.load_config(project)
    write_artifact(project, RUN, "01-product.md", body="")
    st = _state(project, "ANALYSE_PRODUCT")
    assert not gate.check("CLARIFY_PRODUCT", st, c).ok


def test_clarify_not_approved_blocks(project: Path) -> None:
    c = cfg.load_config(project)
    st = _state(project, "CLARIFY_PRODUCT", pending_clarify="awaiting")
    res = gate.check("ANALYSE_TECH", st, c)
    assert not res.ok
    assert "clarif" in res.message.lower()


def test_clarify_approved_passes(project: Path) -> None:
    c = cfg.load_config(project)
    st = _state(project, "CLARIFY_PRODUCT", pending_clarify=None)
    assert gate.check("ANALYSE_TECH", st, c).ok


def test_blocked_run_refused(project: Path) -> None:
    c = cfg.load_config(project)
    st = _state(project, "BUILD_VERIFY", blocked_reason="too many failures")
    res = gate.check("TEST", st, c)
    assert not res.ok
    assert "blocked" in res.message.lower()


def test_done_needs_archive_artifact(project: Path) -> None:
    c = cfg.load_config(project)
    st = _state(project, "ARCHIVE")
    assert not gate.check("DONE", st, c).ok
    write_artifact(project, RUN, "10-archive.md")
    assert gate.check("DONE", st, c).ok
