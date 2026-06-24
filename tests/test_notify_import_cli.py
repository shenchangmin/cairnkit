"""B6 · notify + import CLI (in-process)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from cairnkit.cli import main

YAML = "project: demo\nrepos:\n  - name: demo\n    path: .\n"


def _proj(tmp_path: Path) -> Path:
    (tmp_path / "cairnkit.yaml").write_text(YAML, encoding="utf-8")
    return tmp_path


def test_notify_cli_degrades_local(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    capsys.readouterr()
    assert main(["--root", str(root), "notify", "--event", "done", "--detail", "ok"]) == 0
    assert json.loads(capsys.readouterr().out)["sent"] is False


def test_import_cli_flow(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    capsys.readouterr()
    assert main(["--root", str(root), "import", "init"]) == 0
    assert json.loads(capsys.readouterr().out)["step"] == "doc-collect"
    assert main(["--root", str(root), "import", "advance"]) == 0
    capsys.readouterr()
    assert main(["--root", str(root), "import", "show"]) == 0
    assert json.loads(capsys.readouterr().out)["step"] == "codebase-profile"
