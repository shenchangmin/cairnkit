"""B2 · M4 model + schema."""

from __future__ import annotations

from pathlib import Path

import pytest

from cairnkit.knowledge import model
from cairnkit.knowledge.model import Entry, Evidence, KnowledgeError
from cairnkit.knowledge.schema import SchemaError, validate
from tests.knowledge_fixtures import make_entry


def _entry(**over) -> Entry:
    base = dict(
        id="TK-001", title="t", category="tech", domain=None, type="decision",
        guideline_polarity=None, maturity="draft", knowledge_class="point",
        layer="L1", tags=(), applicable_phases=(), evidence=Evidence(), history=(), body="b",
    )
    base.update(over)
    return Entry(**base)


def test_parse_roundtrip(tmp_path: Path) -> None:
    p = make_entry(tmp_path, id="TK-001", title="Pagination", body="A causal note.\n- bullet")
    entry = model.load_entry(p)
    assert entry.id == "TK-001"
    assert entry.title == "Pagination"
    assert entry.category == "tech"
    assert "bullet" in entry.body  # body starting with '-' survives
    # serialize -> parse is stable
    again = model.parse_entry(model.serialize_entry(entry))
    assert again.id == entry.id
    assert again.body.strip().endswith("- bullet")


def test_parse_missing_frontmatter_raises() -> None:
    with pytest.raises(KnowledgeError):
        model.parse_entry("no frontmatter here")


def test_knowledge_class_defaults_to_point(tmp_path: Path) -> None:
    text = "---\nid: TK-9\ntitle: t\ncategory: tech\ntype: decision\nmaturity: draft\nlayer: L1\n---\nbody"
    entry = model.parse_entry(text)
    assert entry.knowledge_class == "point"


def test_schema_valid_tech_entry() -> None:
    validate(_entry())  # no raise


def test_schema_rejects_unknown_category() -> None:
    with pytest.raises(SchemaError):
        validate(_entry(category="bogus"))


def test_schema_biz_requires_domain() -> None:
    with pytest.raises(SchemaError):
        validate(_entry(id="BK-1", category="biz", domain=None, layer="L2"))


def test_schema_tech_must_not_have_domain() -> None:
    with pytest.raises(SchemaError):
        validate(_entry(domain="advertising"))


def test_schema_guideline_requires_polarity() -> None:
    with pytest.raises(SchemaError):
        validate(_entry(type="guideline", guideline_polarity=None))
    validate(_entry(type="guideline", guideline_polarity="avoid"))


def test_schema_polarity_only_for_guideline() -> None:
    with pytest.raises(SchemaError):
        validate(_entry(type="decision", guideline_polarity="recommend"))


def test_schema_id_prefix_matches_category() -> None:
    with pytest.raises(SchemaError):
        validate(_entry(id="BK-1"))  # tech entry with biz prefix
    with pytest.raises(SchemaError):
        validate(_entry(id="TK-1", category="biz", domain="d", layer="L2"))


def test_schema_l3_exempt_from_prefix() -> None:
    validate(_entry(id="local-1", layer="L3"))  # project-local id allowed at L3


@pytest.mark.parametrize("field,bad", [
    ("id", ""),
    ("title", ""),
    ("type", "bogus"),
    ("maturity", "bogus"),
    ("knowledge_class", "bogus"),
    ("layer", "bogus"),
])
def test_schema_rejects_each_bad_field(field: str, bad: str) -> None:
    with pytest.raises(SchemaError):
        validate(_entry(**{field: bad}))


def test_parse_rejects_non_mapping_history() -> None:
    text = (
        "---\nid: TK-1\ntitle: t\ncategory: tech\ntype: decision\n"
        "maturity: draft\nlayer: L1\nhistory:\n  - just a string\n---\nbody"
    )
    with pytest.raises(KnowledgeError):
        model.parse_entry(text)


def test_body_roundtrip_is_exact(tmp_path: Path) -> None:
    p = make_entry(tmp_path, id="TK-5", title="t", body="line1\n\nline2\n- item")
    e = model.load_entry(p)
    assert model.parse_entry(model.serialize_entry(e)).body == e.body
