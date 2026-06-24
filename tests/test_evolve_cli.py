"""B7 · evolve CLI (in-process)."""
from __future__ import annotations
import json
from pathlib import Path
import pytest
from cairnkit.cli import main

YAML = "project: demo\nrepos:\n  - name: demo\n    path: .\n"

def _proj(tmp_path: Path) -> Path:
    (tmp_path / "cairnkit.yaml").write_text(YAML, encoding="utf-8")
    return tmp_path

def test_evolve_propose_list_apply(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    capsys.readouterr()
    assert main(["--root", str(root), "evolve", "propose", "--id", "fix-1", "--content", "x"]) == 0
    assert json.loads(capsys.readouterr().out)["proposed"] == "fix-1"
    assert main(["--root", str(root), "evolve", "list", "--state", "pending"]) == 0
    assert "fix-1" in json.loads(capsys.readouterr().out)["proposals"]
    assert main(["--root", str(root), "evolve", "apply", "--id", "fix-1"]) == 0
    capsys.readouterr()
    assert main(["--root", str(root), "evolve", "list", "--state", "applied"]) == 0
    assert "fix-1" in json.loads(capsys.readouterr().out)["proposals"]

def test_evolve_apply_unknown_errors(tmp_path: Path) -> None:
    root = _proj(tmp_path)
    assert main(["--root", str(root), "evolve", "apply", "--id", "nope"]) == 2


def test_evolve_reject_and_defer(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    main(["--root", str(root), "evolve", "propose", "--id", "a", "--content", "x"])
    main(["--root", str(root), "evolve", "propose", "--id", "b", "--content", "y"])
    capsys.readouterr()
    assert main(["--root", str(root), "evolve", "reject", "--id", "a"]) == 0
    assert json.loads(capsys.readouterr().out)["a"] == "rejected"
    assert main(["--root", str(root), "evolve", "defer", "--id", "b"]) == 0
    assert json.loads(capsys.readouterr().out)["b"] == "deferred"
