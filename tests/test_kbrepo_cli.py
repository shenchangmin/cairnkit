"""B5 · kbrepo / knowledge stats CLI (in-process)."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest

from cairnkit.cli import main
from cairnkit.knowledge import kbrepo
from tests.knowledge_fixtures import make_entry


def _run(root: Path, *args: str) -> int:
    return main(["--root", str(root), *args])


def _project_with_repo(tmp_path: Path) -> tuple[Path, Path]:
    repo = tmp_path / "team-kb"
    kbrepo.init_repo(repo)
    subprocess.run(["git", "-C", str(repo), "config", "user.email", "t@t.t"], check=True)
    subprocess.run(["git", "-C", str(repo), "config", "user.name", "t"], check=True)
    subprocess.run(["git", "-C", str(repo), "add", "-A"], check=True, capture_output=True)
    subprocess.run(["git", "-C", str(repo), "commit", "-qm", "init"], check=True, capture_output=True)
    proj = tmp_path / "proj"
    proj.mkdir()
    (proj / "cairnkit.yaml").write_text(
        f"project: demo\ndomain: null\nrepos:\n  - name: demo\n    path: .\n"
        f"knowledge_repo:\n  local: {repo}\n",
        encoding="utf-8",
    )
    return proj, repo


def test_kbrepo_push_and_stats_cli(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    proj, repo = _project_with_repo(tmp_path)
    make_entry(repo, id="TK-1", title="P", maturity="proven", phases=["IMPLEMENT"])
    capsys.readouterr()
    assert _run(proj, "kbrepo", "push", "--message", "add TK-1") == 0
    assert json.loads(capsys.readouterr().out)["committed"] is True
    assert _run(proj, "knowledge", "stats") == 0
    assert json.loads(capsys.readouterr().out)["total"] == 1


def test_kbrepo_promote_cli(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    proj, repo = _project_with_repo(tmp_path)
    make_entry(repo, id="TK-1", title="P", layer="L3", phases=["IMPLEMENT"])
    capsys.readouterr()
    assert _run(proj, "kbrepo", "promote", "--id", "TK-1", "--to", "L1") == 0
    assert json.loads(capsys.readouterr().out)["to"] == "L1"


def test_kbrepo_without_config_errors(tmp_path: Path) -> None:
    (tmp_path / "cairnkit.yaml").write_text(
        "project: d\nrepos:\n  - name: d\n    path: .\n", encoding="utf-8"
    )
    assert _run(tmp_path, "kbrepo", "pull") == 2  # no knowledge_repo.local configured
