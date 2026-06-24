"""B4 · lifecycle/lint/extract/touch CLI (in-process)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from cairnkit.cli import main
from tests.knowledge_fixtures import make_entry

YAML = "project: demo\ndomain: null\nknowledge_root: kb\nrepos:\n  - name: demo\n    path: .\n"


def _proj(tmp_path: Path) -> Path:
    (tmp_path / "cairnkit.yaml").write_text(YAML, encoding="utf-8")
    return tmp_path


def _run(root: Path, *args: str) -> int:
    return main(["--root", str(root), *args])


def test_lint_cli(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    make_entry(root / "kb", id="TK-1", title="P", maturity="verified", phases=["IMPLEMENT"])
    capsys.readouterr()
    assert _run(root, "lint") == 0
    data = json.loads(capsys.readouterr().out)
    assert "TK-1" in data["orphans"]


def test_extract_touch_promote_cli(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    run_dir = root / "docs" / "workflows" / "r1"
    run_dir.mkdir(parents=True)
    (run_dir / "knowledge-candidates.json").write_text(json.dumps([
        {"id": "TK-1", "title": "Deep", "category": "tech", "type": "decision",
         "knowledge_class": "causal", "layer": "L1", "applicable_phases": ["IMPLEMENT"],
         "body": "A sufficiently deep, reproducible, transferable technical decision body here, "
                 "explaining the causal mechanism and when it applies."},
    ]), encoding="utf-8")
    capsys.readouterr()
    assert _run(root, "kb", "extract", "--from", str(run_dir)) == 0
    assert json.loads(capsys.readouterr().out)["written"] == ["TK-1"]
    # reference it then promote
    (run_dir / "05.md").write_text('{"knowledgeReferences":[{"id":"TK-1"}]}', encoding="utf-8")
    assert _run(root, "kb", "touch", "--from", str(run_dir)) == 0
    capsys.readouterr()
    assert _run(root, "lifecycle", "promote") == 0
    assert "TK-1" in json.loads(capsys.readouterr().out)["promoted"]


def test_decay_cli_runs(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    make_entry(root / "kb", id="TK-1", title="P", maturity="draft", phases=["IMPLEMENT"])
    capsys.readouterr()
    assert _run(root, "lifecycle", "decay") == 0
    assert "decayed" in json.loads(capsys.readouterr().out)
