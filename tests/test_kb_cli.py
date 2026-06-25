"""B2 · kb CLI subcommands (in-process)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from cairnkit.cli import main
from tests.knowledge_fixtures import make_entry

YAML = "project: demo\ndomain: advertising\nknowledge_root: kb\nrepos:\n  - name: demo\n    path: .\n"


def _proj(tmp_path: Path) -> Path:
    (tmp_path / "cairnkit.yaml").write_text(YAML, encoding="utf-8")
    kb = tmp_path / "kb"
    make_entry(kb, id="TK-001", title="P", maturity="proven", kclass="causal",
               phases=["ARCHITECT_BACKEND"])
    make_entry(kb, id="BK-001", title="Flow", category="biz", domain="advertising",
               layer="L2", phases=["ANALYSE_PRODUCT"])
    return tmp_path


def test_kb_build_index(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    capsys.readouterr()
    assert main(["--root", str(root), "kb", "build-index"]) == 0
    assert json.loads(capsys.readouterr().out)["total"] == 2


def test_kb_query_uses_config_domain(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    capsys.readouterr()
    # config domain=advertising, so the biz entry is visible at its stage
    assert main(["--root", str(root), "kb", "query", "--stage", "ANALYSE_PRODUCT", "--budget", "500"]) == 0
    data = json.loads(capsys.readouterr().out)
    assert "BK-001" in data["injected_ids"]
    assert "over_budget" in data  # the never-silent budget flag is exposed at the CLI


def test_kb_validate_ok_and_fail(tmp_path: Path, capsys: pytest.CaptureFixture) -> None:
    root = _proj(tmp_path)
    good = root / "kb" / "tech-wiki" / "TK-001.md"
    assert main(["--root", str(root), "kb", "validate", str(good)]) == 0
    # a malformed entry (tech with a domain) -> schema error -> code 2
    bad = make_entry(root / "kb", id="TK-009", category="tech", domain="oops",
                     phases=["IMPLEMENT"])
    assert main(["--root", str(root), "kb", "validate", str(bad)]) == 2


def test_knowledge_root_defaults_to_docs_knowledge(tmp_path: Path) -> None:
    from cairnkit.config import load_config
    (tmp_path / "cairnkit.yaml").write_text("project: d\nrepos:\n  - name: d\n    path: .\n", encoding="utf-8")
    c = load_config(tmp_path)
    assert c.knowledge_root == tmp_path / "docs" / "knowledge"
