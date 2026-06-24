"""B1+B3 · cli.py in-process — drive main() directly (capsys + return codes)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from cairnkit.cli import main
from tests.conftest import CAIRNKIT_YAML, write_artifact

RUN = "2026-06-24-demo"


def _init(tmp_path: Path) -> Path:
    (tmp_path / "cairnkit.yaml").write_text(CAIRNKIT_YAML, encoding="utf-8")
    assert main(["--root", str(tmp_path), "state", "init", "--run-id", RUN]) == 0
    return tmp_path


def _run(root: Path, *args: str) -> int:
    return main(["--root", str(root), *args])


def test_show_and_first_advance(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    capsys.readouterr()
    assert _run(root, "state", "show") == 0
    assert json.loads(capsys.readouterr().out)["stage"] == "INIT"
    assert _run(root, "state", "advance") == 0
    assert json.loads(capsys.readouterr().out)["stage"] == "INTENT_GATE"


def test_advance_gate_refusal_returns_3(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    _run(root, "state", "set-stage", "ANALYSE_PRODUCT")  # producing stage, no artifact yet
    capsys.readouterr()
    assert _run(root, "state", "advance") == 3
    assert "missing" in capsys.readouterr().err.lower()


def test_clarify_flow(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    _run(root, "state", "set-stage", "ANALYSE_PRODUCT")
    write_artifact(root, RUN, "01-product.md")
    _run(root, "state", "advance")               # -> CLARIFY_PRODUCT (pending)
    capsys.readouterr()
    assert _run(root, "state", "resume") == 0
    assert json.loads(capsys.readouterr().out)["paused"] is True
    assert _run(root, "state", "advance") == 3   # not approved
    assert _run(root, "state", "approve-clarify") == 0
    capsys.readouterr()
    assert _run(root, "state", "advance") == 0    # -> ANALYSE_TECH
    assert json.loads(capsys.readouterr().out)["stage"] == "ANALYSE_TECH"


def test_set_path_mode_and_intent(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    capsys.readouterr()
    assert _run(root, "state", "set-path-mode", "lite") == 0
    assert json.loads(capsys.readouterr().out)["path_mode"] == "lite"
    assert _run(root, "state", "set-path-mode", "turbo") == 2  # invalid
    assert _run(root, "intent", "classify", "--text", "fix a typo in the readme") == 0
    assert json.loads(capsys.readouterr().out)["path_mode"] == "single"


def test_fail_and_unblock(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    for _ in range(5):
        _run(root, "state", "fail", "--stage", "BUILD_VERIFY")
    capsys.readouterr()
    _run(root, "state", "show")
    assert json.loads(capsys.readouterr().out)["blocked_reason"] is not None
    assert _run(root, "state", "fail", "--stage", "IMPLEMENT") == 2  # not a verify stage
    assert _run(root, "state", "unblock") == 0


def test_set_stage_and_corrupt(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    capsys.readouterr()
    assert _run(root, "state", "set-stage", "DONE") == 0
    assert json.loads(capsys.readouterr().out)["stage"] == "DONE"
    (root / ".cairnkit" / "STATE.yaml").write_text("stage: INIT\n", encoding="utf-8")
    assert _run(root, "state", "show") == 4


def test_set_stage_illegal_returns_2(tmp_path: Path) -> None:
    root = _init(tmp_path)
    assert _run(root, "state", "set-stage", "NONSENSE") == 2


def test_missing_config_returns_2(tmp_path: Path) -> None:
    assert _run(tmp_path, "state", "show") == 2


def test_config_show(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    (tmp_path / "cairnkit.yaml").write_text(CAIRNKIT_YAML, encoding="utf-8")
    capsys.readouterr()
    assert _run(tmp_path, "config", "show") == 0
    data = json.loads(capsys.readouterr().out)
    assert data["project"] == "demo-task"
    assert data["has_run"] is False
    _run(tmp_path, "state", "init", "--run-id", RUN)
    capsys.readouterr()
    _run(tmp_path, "config", "show")
    assert json.loads(capsys.readouterr().out)["has_run"] is True


def test_config_show_missing_returns_2(tmp_path: Path) -> None:
    assert _run(tmp_path, "config", "show") == 2


def test_state_init_refuses_to_overwrite_existing_run(tmp_path: Path) -> None:
    root = _init(tmp_path)
    assert _run(root, "state", "init", "--run-id", "another") == 2
