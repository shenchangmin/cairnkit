"""B4 · M6 extract gate, reference writeback, lint, repo-level promote/decay (integration)."""

from __future__ import annotations

import json
from pathlib import Path

from cairnkit.knowledge.extract_gate import evaluate, extract_from_run
from cairnkit.knowledge.lifecycle import decay_repo, promote_repo
from cairnkit.knowledge.lint import lint
from cairnkit.knowledge.model import load_entry
from cairnkit.knowledge.refs import collect_references, touch
from tests.knowledge_fixtures import make_entry

GOOD_BODY = "A reproducible, transferable technical decision with enough depth to matter here."


# --- extract gate ----------------------------------------------------------

def test_extract_gate_rejects_shallow_candidate() -> None:
    assert not evaluate({"title": "x", "type": "decision", "applicable_phases": ["IMPLEMENT"],
                         "body": "too short"}).accepted


def test_extract_gate_accepts_deep_candidate() -> None:
    assert evaluate({"id": "TK-1", "title": "x", "type": "decision",
                     "applicable_phases": ["IMPLEMENT"], "body": GOOD_BODY,
                     "knowledge_class": "causal"}).accepted


def test_extract_from_run_writes_drafts_and_reports_rejects(tmp_path: Path) -> None:
    kb = tmp_path / "kb"
    run = tmp_path / "run"
    run.mkdir()
    (run / "knowledge-candidates.json").write_text(json.dumps([
        {"id": "TK-100", "title": "Good", "category": "tech", "type": "decision",
         "knowledge_class": "causal", "layer": "L1", "applicable_phases": ["IMPLEMENT"], "body": GOOD_BODY},
        {"id": "TK-101", "title": "Shallow", "category": "tech", "type": "decision",
         "applicable_phases": ["IMPLEMENT"], "body": "nope"},
    ]), encoding="utf-8")
    res = extract_from_run(run, kb)
    assert res["written"] == ["TK-100"]
    assert res["rejected"] and res["rejected"][0]["title"] == "Shallow"
    assert (kb / "tech-wiki" / "TK-100.md").exists()
    assert load_entry(kb / "tech-wiki" / "TK-100.md").maturity == "draft"


# --- reference writeback ---------------------------------------------------

def test_collect_and_touch_updates_evidence(tmp_path: Path) -> None:
    kb = tmp_path / "kb"
    make_entry(kb, id="TK-1", title="P", phases=["IMPLEMENT"])
    run = tmp_path / "run"
    run.mkdir()
    (run / "05-implement.md").write_text(
        'done.\n{"knowledgeReferences": [{"id": "TK-1", "title": "P", "usedIn": "step2"}]}\n',
        encoding="utf-8",
    )
    assert collect_references(run) == ["TK-1"]
    summary = touch(kb, run, project="proj-a", today="2026-06-24")
    assert "TK-1" in summary["updated"]
    e = load_entry(kb / "tech-wiki" / "TK-1.md")
    assert e.evidence.ref_count == 1
    assert e.evidence.last_referenced == "2026-06-24"
    assert "proj-a" in e.evidence.projects


def test_collect_references_handles_nested_json(tmp_path: Path) -> None:
    # a rich artifact object with a nested sibling object must not hide the references
    run = tmp_path / "run"
    run.mkdir()
    (run / "a.md").write_text(
        'prose\n{"stage": "IMPLEMENT", "context": {"phase": 1}, '
        '"knowledgeReferences": [{"id": "TK-7", "title": "x"}]}\nmore prose\n',
        encoding="utf-8",
    )
    assert collect_references(run) == ["TK-7"]


def test_extract_rejects_candidate_without_id(tmp_path: Path) -> None:
    assert not evaluate({"title": "x", "type": "decision",
                         "applicable_phases": ["IMPLEMENT"], "body": GOOD_BODY}).accepted


def test_extract_from_run_handles_malformed_candidates(tmp_path: Path) -> None:
    run = tmp_path / "run"
    run.mkdir()
    (run / "knowledge-candidates.json").write_text("{ not valid json", encoding="utf-8")
    res = extract_from_run(run, tmp_path / "kb")
    assert "error" in res and res["written"] == []


def test_touch_unknown_reference_is_not_an_error(tmp_path: Path) -> None:
    kb = tmp_path / "kb"
    make_entry(kb, id="TK-1", title="P", phases=["IMPLEMENT"])
    run = tmp_path / "run"
    run.mkdir()
    (run / "a.md").write_text('{"knowledgeReferences": [{"id": "TK-999"}]}', encoding="utf-8")
    summary = touch(kb, run, project="p", today="2026-06-24")
    assert summary["unknown"] == ["TK-999"]
    assert summary["updated"] == []


# --- the full closed loop: reference -> promote ----------------------------

def test_reference_then_promote_repo(tmp_path: Path) -> None:
    kb = tmp_path / "kb"
    make_entry(kb, id="TK-1", title="P", maturity="draft", phases=["IMPLEMENT"])
    run = tmp_path / "run"
    run.mkdir()
    (run / "x.md").write_text('{"knowledgeReferences": [{"id": "TK-1"}]}', encoding="utf-8")
    touch(kb, run, project="p", today="2026-06-24")
    changed = promote_repo(kb)
    assert "TK-1" in changed
    assert load_entry(kb / "tech-wiki" / "TK-1.md").maturity == "verified"


def test_decay_repo(tmp_path: Path) -> None:
    from datetime import date
    kb = tmp_path / "kb"
    make_entry(kb, id="TK-1", title="P", maturity="proven", phases=["IMPLEMENT"])
    # set last_referenced far in the past via touch then manual? simpler: write directly
    e = load_entry(kb / "tech-wiki" / "TK-1.md")
    from cairnkit.knowledge.model import Evidence, save_entry
    save_entry(e.path, e.with_(evidence=Evidence(last_referenced="2024-01-01", ref_count=1)))
    changed = decay_repo(kb, now=date(2026, 6, 1))
    assert "TK-1" in changed


# --- lint ------------------------------------------------------------------

def test_lint_detects_orphans_duplicates_conflicts(tmp_path: Path) -> None:
    kb = tmp_path / "kb"
    make_entry(kb, id="TK-1", title="Dup", maturity="verified", phases=["IMPLEMENT"])  # orphan (ref_count 0)
    make_entry(kb, id="TK-2", title="Dup", maturity="draft", phases=["IMPLEMENT"])      # duplicate title
    make_entry(kb, id="TK-3", title="R", type="guideline", polarity="recommend",
               tags=["x"], phases=["IMPLEMENT"])
    make_entry(kb, id="TK-4", title="A", type="guideline", polarity="avoid",
               tags=["x"], phases=["IMPLEMENT"])
    report = lint(kb)
    assert "TK-1" in report.orphans
    assert any(set(g) == {"TK-1", "TK-2"} for g in report.duplicates)
    assert any(set(g) == {"TK-3", "TK-4"} for g in report.conflicts)
    assert not report.clean


def test_lint_fix_rebuilds_index(tmp_path: Path) -> None:
    kb = tmp_path / "kb"
    make_entry(kb, id="TK-1", title="P", maturity="draft", phases=["IMPLEMENT"])
    report = lint(kb, fix=True)
    assert "rebuilt index" in report.fixed
    assert (kb / "knowledge-catalog.md").exists()
