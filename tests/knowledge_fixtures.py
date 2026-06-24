"""Helpers to build a temporary knowledge base for M5 tests."""

from __future__ import annotations

from pathlib import Path

ENTRY_TMPL = """\
---
id: {id}
title: {title}
category: {category}
domain: {domain}
type: {type}
guideline_polarity: {polarity}
maturity: {maturity}
knowledge_class: {kclass}
layer: {layer}
tags: {tags}
applicable_phases: {phases}
evidence:
  contributors: [tester]
  sources: []
  projects: {projects}
  last_referenced: null
  ref_count: 0
history: []
---

{body}
"""


def make_entry(
    kb_root: Path,
    *,
    id: str,
    title: str = "Title",
    category: str = "tech",
    domain: str | None = None,
    type: str = "decision",
    polarity: str | None = None,
    maturity: str = "draft",
    kclass: str = "point",
    layer: str = "L1",
    tags: list[str] | None = None,
    phases: list[str] | None = None,
    projects: list[str] | None = None,
    body: str = "Body text.",
) -> Path:
    if category == "tech":
        rel = Path("tech-wiki") / f"{id}.md"
    else:
        rel = Path("biz-wiki") / (domain or "_") / f"{id}.md"
    path = kb_root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        ENTRY_TMPL.format(
            id=id, title=title, category=category,
            domain=("null" if domain is None else domain),
            type=type, polarity=("null" if polarity is None else polarity),
            maturity=maturity, kclass=kclass, layer=layer,
            tags=(tags or []), phases=(phases or []),
            projects=(projects or []), body=body,
        ),
        encoding="utf-8",
    )
    return path
