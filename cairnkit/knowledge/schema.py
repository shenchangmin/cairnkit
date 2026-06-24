"""Frontmatter schema validation (M4) — entries that fail are rejected, not stored.

Enforces the classification vocabulary and cross-field rules (biz needs a domain, the
guideline polarity only applies to guidelines, the id prefix matches the category, etc.).
"""

from __future__ import annotations

from collections.abc import Iterator

from cairnkit.errors import CairnkitError
from cairnkit.knowledge import (
    CATEGORIES,
    KNOWLEDGE_CLASSES,
    LAYERS,
    MATURITIES,
    POLARITIES,
    TYPES,
)
from cairnkit.knowledge.model import Entry


class SchemaError(CairnkitError):
    """An entry violates the knowledge schema."""

    code = 2


def validate(entry: Entry) -> None:
    """Raise SchemaError on the first violation; return None when valid."""
    errors = list(iter_errors(entry))
    if errors:
        raise SchemaError(f"{entry.id or '<no id>'}: " + "; ".join(errors))


def iter_errors(entry: Entry) -> Iterator[str]:
    """Yield human-readable messages for every schema violation (for Lint/CLI)."""
    if not entry.id:
        yield "missing id"
    if not entry.title:
        yield "missing title"

    if entry.category not in CATEGORIES:
        yield f"category must be one of {CATEGORIES}, got {entry.category!r}"
    if entry.type not in TYPES:
        yield f"type must be one of {TYPES}, got {entry.type!r}"
    if entry.maturity not in MATURITIES:
        yield f"maturity must be one of {MATURITIES}, got {entry.maturity!r}"
    if entry.knowledge_class not in KNOWLEDGE_CLASSES:
        yield f"knowledge_class must be one of {KNOWLEDGE_CLASSES}, got {entry.knowledge_class!r}"
    if entry.layer not in LAYERS:
        yield f"layer must be one of {LAYERS}, got {entry.layer!r}"

    # cross-field rules
    if entry.category == "biz" and not entry.domain:
        yield "biz knowledge requires a domain"
    if entry.category == "tech" and entry.domain:
        yield "tech knowledge must not set a domain"

    if entry.type == "guideline":
        if entry.guideline_polarity not in POLARITIES:
            yield f"guideline requires guideline_polarity in {POLARITIES}"
    elif entry.guideline_polarity is not None:
        yield "guideline_polarity is only valid for type=guideline"

    # id prefix convention: TK- for tech, BK- for biz (L3 project-local entries exempt)
    if entry.id and entry.layer != "L3":
        if entry.category == "tech" and not entry.id.startswith("TK-"):
            yield "tech entry id should start with 'TK-'"
        if entry.category == "biz" and not entry.id.startswith("BK-"):
            yield "biz entry id should start with 'BK-'"
