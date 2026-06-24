"""T5 · cli.py in-process coverage — drive cli.main() directly (capsys + return codes).

The subprocess suite (test_cli.py) verifies exit codes end-to-end; these tests exercise
the same handlers in-process so coverage is measured and the JSON payloads are asserted.
"""

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


def test_show_and_advance(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    capsys.readouterr()
    assert _run(root, "state", "show") == 0
    assert json.loads(capsys.readouterr().out)["stage"] == "INIT"
    assert _run(root, "state", "advance") == 0
    assert json.loads(capsys.readouterr().out)["stage"] == "ANALYSE_PRODUCT"


def test_advance_gate_refusal_returns_3(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    _run(root, "state", "advance")  # -> ANALYSE_PRODUCT
    capsys.readouterr()
    assert _run(root, "state", "advance") == 3
    assert "missing" in capsys.readouterr().err.lower()


def test_full_run_through_clarify(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    _run(root, "state", "advance")              # ANALYSE_PRODUCT
    write_artifact(root, RUN, "01-product.md")
    _run(root, "state", "advance")              # CLARIFY_PRODUCT
    capsys.readouterr()
    assert _run(root, "state", "resume") == 0
    assert json.loads(capsys.readouterr().out)["paused"] is True
    assert _run(root, "state", "approve-clarify") == 0
    capsys.readouterr()
    assert _run(root, "state", "advance") == 0  # ARCHITECT_BACKEND
    write_artifact(root, RUN, "03-arch.md")
    capsys.readouterr()
    assert _run(root, "state", "advance") == 0  # DONE
    assert json.loads(capsys.readouterr().out)["stage"] == "DONE"


def test_gate_check_codes(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    _run(root, "state", "advance")
    capsys.readouterr()
    assert _run(root, "gate", "check", "--stage", "CLARIFY_PRODUCT") == 3
    assert json.loads(capsys.readouterr().out)["ok"] is False


def test_set_stage_and_corrupt(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _init(tmp_path)
    capsys.readouterr()
    assert _run(root, "state", "set-stage", "DONE") == 0
    assert json.loads(capsys.readouterr().out)["stage"] == "DONE"
    # corrupt STATE -> code 4
    (root / ".cairnkit" / "STATE.yaml").write_text("stage: INIT\n", encoding="utf-8")
    assert _run(root, "state", "show") == 4


def test_set_stage_illegal_returns_2(tmp_path: Path) -> None:
    root = _init(tmp_path)
    assert _run(root, "state", "set-stage", "NONSENSE") == 2


def test_missing_config_returns_2(tmp_path: Path) -> None:
    # no cairnkit.yaml -> ConfigError (code 2)
    assert _run(tmp_path, "state", "show") == 2


def test_config_show(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    (tmp_path / "cairnkit.yaml").write_text(CAIRNKIT_YAML, encoding="utf-8")
    capsys.readouterr()
    # valid config, no run yet
    assert _run(tmp_path, "config", "show") == 0
    data = json.loads(capsys.readouterr().out)
    assert data["project"] == "demo-task"
    assert data["has_run"] is False
    # after init, has_run flips true
    _run(tmp_path, "state", "init", "--run-id", RUN)
    capsys.readouterr()
    _run(tmp_path, "config", "show")
    assert json.loads(capsys.readouterr().out)["has_run"] is True


def test_config_show_missing_returns_2(tmp_path: Path) -> None:
    assert _run(tmp_path, "config", "show") == 2


def test_state_init_refuses_to_overwrite_existing_run(tmp_path: Path) -> None:
    root = _init(tmp_path)  # already inits a run
    # second init on an existing run must be refused (UsageError -> code 2)
    assert _run(root, "state", "init", "--run-id", "another") == 2


def test_usage_error_exits_2(tmp_path: Path) -> None:
    with pytest.raises(SystemExit) as exc:
        main(["--root", str(tmp_path), "state", "nonsense"])
    assert exc.value.code == 2
