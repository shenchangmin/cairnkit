"""T5 · cli.py — subcommand dispatch + JSON output + return-code split (RED first).

Run through the real ``python -m cairnkit`` entry point via subprocess so exit codes
are asserted honestly (0 ok / 2 usage / 3 gate / 4 corrupt).
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

from tests.conftest import CAIRNKIT_YAML, write_artifact

RUN = "2026-06-24-demo"


def run(root: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, "-m", "cairnkit", "--root", str(root), *args],
        capture_output=True, text=True,
    )


def _init(tmp_path: Path) -> Path:
    (tmp_path / "cairnkit.yaml").write_text(CAIRNKIT_YAML, encoding="utf-8")
    # bootstrap a STATE via set-stage path: use show after init through a real command
    return tmp_path


def _bootstrap_state(root: Path) -> None:
    # init_state is internal; the CLI bootstraps via `state init`
    run(root, "state", "init", "--run-id", RUN)


def test_state_show_returns_json(tmp_path: Path) -> None:
    root = _init(tmp_path)
    _bootstrap_state(root)
    cp = run(root, "state", "show")
    assert cp.returncode == 0
    data = json.loads(cp.stdout)
    assert data["stage"] == "INIT"
    assert data["run_id"] == RUN


def test_state_advance_success_code_0(tmp_path: Path) -> None:
    root = _init(tmp_path)
    _bootstrap_state(root)
    cp = run(root, "state", "advance")
    assert cp.returncode == 0
    assert json.loads(cp.stdout)["stage"] == "ANALYSE_PRODUCT"


def test_state_advance_gate_refusal_code_3(tmp_path: Path) -> None:
    root = _init(tmp_path)
    _bootstrap_state(root)
    run(root, "state", "advance")  # -> ANALYSE_PRODUCT
    cp = run(root, "state", "advance")  # needs 01-product.md, missing
    assert cp.returncode == 3
    assert cp.stderr  # error reported to stderr


def test_gate_check_pass_and_fail_codes(tmp_path: Path) -> None:
    root = _init(tmp_path)
    _bootstrap_state(root)
    run(root, "state", "advance")  # -> ANALYSE_PRODUCT
    fail = run(root, "gate", "check", "--stage", "CLARIFY_PRODUCT")
    assert fail.returncode == 3
    assert json.loads(fail.stdout)["ok"] is False
    write_artifact(root, RUN, "01-product.md")
    ok = run(root, "gate", "check", "--stage", "CLARIFY_PRODUCT")
    assert ok.returncode == 0
    assert json.loads(ok.stdout)["ok"] is True


def test_resume_reports_paused_flag(tmp_path: Path) -> None:
    root = _init(tmp_path)
    _bootstrap_state(root)
    run(root, "state", "advance")
    write_artifact(root, RUN, "01-product.md")
    run(root, "state", "advance")  # -> CLARIFY_PRODUCT
    cp = run(root, "state", "resume")
    assert cp.returncode == 0
    data = json.loads(cp.stdout)
    assert data["stage"] == "CLARIFY_PRODUCT"
    assert data["paused"] is True


def test_corrupt_state_code_4(tmp_path: Path) -> None:
    root = _init(tmp_path)
    (root / ".cairnkit").mkdir()
    (root / ".cairnkit" / "STATE.yaml").write_text("stage: INIT\n", encoding="utf-8")
    cp = run(root, "state", "show")
    assert cp.returncode == 4


def test_unknown_subcommand_code_2(tmp_path: Path) -> None:
    root = _init(tmp_path)
    cp = run(root, "state", "nonsense")
    assert cp.returncode == 2


def test_set_stage_illegal_enum_code_2(tmp_path: Path) -> None:
    root = _init(tmp_path)
    _bootstrap_state(root)
    cp = run(root, "state", "set-stage", "NONSENSE")
    assert cp.returncode == 2
