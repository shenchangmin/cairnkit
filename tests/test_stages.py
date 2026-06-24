"""B3 · stages.py — sequence & path-mode helpers."""

from __future__ import annotations

import json
from pathlib import Path

from cairnkit import stages
from cairnkit.cli import main


def test_next_stage_full_chain() -> None:
    assert stages.next_stage("INIT", "full") == "INTENT_GATE"
    assert stages.next_stage("ARCHIVE", "full") == "DONE"
    assert stages.next_stage("DONE", "full") is None


def test_next_stage_skips_excluded_in_lite() -> None:
    # ARCHITECT_BACKEND's next in lite skips frontend stages straight to IMPLEMENT
    assert stages.next_stage("CLARIFY_ARCH_BACKEND", "lite") == "IMPLEMENT"


def test_next_stage_from_foreign_current_falls_back() -> None:
    # current was a stage excluded from the active mode -> resume at next active stage
    assert stages.next_stage("ARCHITECT_FRONTEND", "lite") == "IMPLEMENT"
    assert stages.next_stage("CLARIFY_TECH", "single") == "IMPLEMENT"


def test_stages_for_unknown_mode_raises() -> None:
    import pytest
    with pytest.raises(ValueError):
        stages.stages_for("turbo")


def test_stages_for_single_is_minimal() -> None:
    s = stages.stages_for("single")
    assert "ANALYSE_PRODUCT" not in s
    assert "IMPLEMENT" in s and "DONE" in s


def test_intent_classify_from_file(tmp_path: Path, capsys) -> None:
    f = tmp_path / "req.txt"
    f.write_text("rename a constant", encoding="utf-8")
    capsys.readouterr()
    assert main(["intent", "classify", "--input", str(f)]) == 0
    assert json.loads(capsys.readouterr().out)["path_mode"] == "single"
