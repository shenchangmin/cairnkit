"""B4 · M6 lifecycle: promote / decay / judge_layer."""

from __future__ import annotations

from datetime import date

from cairnkit.knowledge.lifecycle import decay, judge_layer, promote
from cairnkit.knowledge.model import Entry, Evidence


def _entry(**over) -> Entry:
    base = dict(
        id="TK-1", title="t", category="tech", domain=None, type="decision",
        guideline_polarity=None, maturity="draft", knowledge_class="point",
        layer="L3", tags=(), applicable_phases=(), evidence=Evidence(), history=(), body="b",
    )
    base.update(over)
    return Entry(**base)


def test_promote_draft_to_verified_on_first_reference() -> None:
    e = _entry(maturity="draft", evidence=Evidence(ref_count=1))
    assert promote(e).maturity == "verified"


def test_promote_verified_to_proven_on_two_projects() -> None:
    e = _entry(maturity="verified", evidence=Evidence(ref_count=3, projects=("a", "b")))
    assert promote(e).maturity == "proven"


def test_promote_noop_when_criteria_unmet() -> None:
    e = _entry(maturity="draft", evidence=Evidence(ref_count=0))
    assert promote(e).maturity == "draft"
    e2 = _entry(maturity="verified", evidence=Evidence(projects=("only",)))
    assert promote(e2).maturity == "verified"


def test_decay_proven_to_verified_after_12_months() -> None:
    e = _entry(maturity="proven", evidence=Evidence(last_referenced="2025-01-01", ref_count=5))
    assert decay(e, now=date(2026, 6, 1)).maturity == "verified"


def test_decay_verified_to_draft_after_6_months() -> None:
    e = _entry(maturity="verified", evidence=Evidence(last_referenced="2025-10-01", ref_count=1))
    assert decay(e, now=date(2026, 6, 1)).maturity == "draft"


def test_no_decay_when_recent_or_never_referenced() -> None:
    fresh = _entry(maturity="proven", evidence=Evidence(last_referenced="2026-05-01"))
    assert decay(fresh, now=date(2026, 6, 1)).maturity == "proven"
    never = _entry(maturity="proven", evidence=Evidence(last_referenced=None))
    assert decay(never, now=date(2026, 6, 1)).maturity == "proven"


def test_judge_layer() -> None:
    assert judge_layer(_entry(evidence=Evidence(projects=("only",)))) == "L3"
    assert judge_layer(_entry(category="tech", evidence=Evidence(projects=("a", "b")))) == "L1"
    assert judge_layer(_entry(category="biz", domain="ads", evidence=Evidence(projects=("a", "b")))) == "L2"


def test_promote_records_history() -> None:
    e = _entry(maturity="draft", evidence=Evidence(ref_count=1))
    promoted = promote(e, now=date(2026, 6, 1))
    assert promoted.history[-1]["update_type"] == "promote"
