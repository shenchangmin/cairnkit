"""B2 · M5 index + query."""

from __future__ import annotations

from pathlib import Path

from cairnkit.knowledge.index import build_index, iter_entries
from cairnkit.knowledge.query import query
from tests.knowledge_fixtures import make_entry


def _kb(tmp_path: Path) -> Path:
    make_entry(tmp_path, id="TK-001", title="Pagination", maturity="proven",
               kclass="causal", phases=["ARCHITECT_BACKEND"], tags=["mysql"])
    make_entry(tmp_path, id="TK-002", title="Interceptor", maturity="draft",
               kclass="point", phases=["ARCHITECT_BACKEND", "ANALYSE_TECH"])
    make_entry(tmp_path, id="BK-001", title="Ad review flow", category="biz",
               domain="advertising", layer="L2", maturity="verified",
               kclass="spatiotemporal", phases=["ANALYSE_PRODUCT"])
    return tmp_path


def test_iter_entries_skips_catalogs(tmp_path: Path) -> None:
    _kb(tmp_path)
    build_index(tmp_path)  # writes catalog.md files
    ids = {e.id for e in iter_entries(tmp_path)}
    assert ids == {"TK-001", "TK-002", "BK-001"}  # catalogs not loaded as entries


def test_build_index_writes_three_levels(tmp_path: Path) -> None:
    _kb(tmp_path)
    stats = build_index(tmp_path)
    assert stats["total"] == 3
    assert (tmp_path / "knowledge-catalog.md").exists()
    assert (tmp_path / "tech-wiki" / "catalog.md").exists()
    assert (tmp_path / "biz-wiki" / "advertising" / "catalog.md").exists()
    panorama = (tmp_path / "knowledge-catalog.md").read_text(encoding="utf-8")
    assert "total: 3" in panorama


def test_query_filters_by_stage(tmp_path: Path) -> None:
    _kb(tmp_path)
    res = query(tmp_path, stage="ARCHITECT_BACKEND", budget_lines=1000)
    assert set(res.injected_ids) == {"TK-001", "TK-002"}  # BK-001 is ANALYSE_PRODUCT only


def test_query_biz_requires_matching_domain(tmp_path: Path) -> None:
    _kb(tmp_path)
    # without domain, the biz entry is invisible
    res_none = query(tmp_path, stage="ANALYSE_PRODUCT", budget_lines=1000)
    assert "BK-001" not in res_none.injected_ids
    # with matching domain, it shows
    res_dom = query(tmp_path, stage="ANALYSE_PRODUCT", budget_lines=1000, domain="advertising")
    assert "BK-001" in res_dom.injected_ids


def test_query_ranks_proven_and_higher_class_first(tmp_path: Path) -> None:
    _kb(tmp_path)
    res = query(tmp_path, stage="ARCHITECT_BACKEND", budget_lines=1000)
    # TK-001 (proven, causal) ranks before TK-002 (draft, point)
    assert res.injected_ids[0] == "TK-001"


def test_query_budget_truncates_and_reports_dropped(tmp_path: Path) -> None:
    _kb(tmp_path)
    # tiny budget: only the top-ranked entry is force-included; the rest are dropped (not silent)
    res = query(tmp_path, stage="ARCHITECT_BACKEND", budget_lines=5)
    assert res.injected_ids == ("TK-001",)
    assert any(d["id"] == "TK-002" for d in res.dropped)
    assert res.over_budget is True  # the single top entry alone exceeds 5 lines — flagged, not silent


def test_query_respects_budget_for_non_top_entries(tmp_path: Path) -> None:
    # three small entries; a budget big enough for two but not three
    for i in range(3):
        make_entry(tmp_path, id=f"TK-10{i}", title=f"e{i}", maturity="proven",
                   phases=["IMPLEMENT"], body="one line body")
    one = query(tmp_path, stage="IMPLEMENT", budget_lines=10_000)
    per_entry = one.lines // 3
    res = query(tmp_path, stage="IMPLEMENT", budget_lines=per_entry * 2 + 1)
    # top entry always in; budget then hard-caps the rest
    assert len(res.injected_ids) == 2
    assert res.over_budget is False
    assert res.lines <= res.budget_lines
    assert len(res.dropped) == 1
