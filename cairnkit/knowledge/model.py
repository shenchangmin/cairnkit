"""Knowledge entry model + Markdown-with-frontmatter (de)serialization (M4).

An entry is a Markdown file: a YAML frontmatter block (the classification metadata) then a
free-text body. Files are the single source of truth; this module just parses/serializes them.
"""

from __future__ import annotations

import io
from dataclasses import dataclass, field
from pathlib import Path

from ruamel.yaml import YAML

from cairnkit.errors import CairnkitError

_yaml = YAML()
_yaml.default_flow_style = False


class KnowledgeError(CairnkitError):
    """Malformed knowledge entry (bad frontmatter / unreadable)."""

    code = 2


@dataclass(frozen=True)
class Evidence:
    contributors: tuple[str, ...] = ()
    sources: tuple[str, ...] = ()
    projects: tuple[str, ...] = ()
    last_referenced: str | None = None
    ref_count: int = 0


@dataclass(frozen=True)
class Entry:
    id: str
    title: str
    category: str                       # tech | biz
    domain: str | None
    type: str                           # model|decision|guideline|pitfall|process
    guideline_polarity: str | None      # recommend|avoid (guideline only)
    maturity: str                       # draft|verified|proven
    knowledge_class: str                # point|causal|spatiotemporal
    layer: str                          # L0-P|L0-T|L1|L2|L3
    tags: tuple[str, ...]
    applicable_phases: tuple[str, ...]
    evidence: Evidence
    history: tuple[dict, ...]
    body: str
    path: Path | None = field(default=None, compare=False)


def _split_frontmatter(text: str) -> tuple[str, str]:
    """Return (frontmatter_yaml, body). Raises if the leading/closing --- fence is absent."""
    if not text.startswith("---"):
        raise KnowledgeError("entry missing leading '---' frontmatter block")
    rest = text[len("---"):]
    end = rest.find("\n---")           # closing fence on its own line
    if end == -1:
        raise KnowledgeError("entry frontmatter is not terminated by '---'")
    fm = rest[:end]
    after = rest[end + len("\n---"):]  # the remainder of the closing '---' line, then body
    newline = after.find("\n")
    body = after[newline + 1:] if newline != -1 else ""
    return fm, body.lstrip("\n")


def parse_entry(text: str, path: Path | None = None) -> Entry:
    """Parse Markdown-with-frontmatter into an Entry. Does not validate semantics (see schema)."""
    fm_text, body = _split_frontmatter(text)
    try:
        data = _yaml.load(fm_text) or {}
    except Exception as exc:  # ruamel raises various YAMLError subclasses
        raise KnowledgeError(f"entry frontmatter is not valid YAML: {exc}") from exc
    if not isinstance(data, dict):
        raise KnowledgeError("entry frontmatter must be a mapping")

    history = data.get("history") or []
    if not all(isinstance(h, dict) for h in history):
        raise KnowledgeError("each history item must be a mapping")

    ev = data.get("evidence") or {}
    evidence = Evidence(
        contributors=tuple(ev.get("contributors") or ()),
        sources=tuple(ev.get("sources") or ()),
        projects=tuple(ev.get("projects") or ()),
        last_referenced=ev.get("last_referenced"),
        ref_count=int(ev.get("ref_count") or 0),
    )
    return Entry(
        id=str(data.get("id", "")),
        title=str(data.get("title", "")),
        category=str(data.get("category", "")),
        domain=data.get("domain"),
        type=str(data.get("type", "")),
        guideline_polarity=data.get("guideline_polarity"),
        maturity=str(data.get("maturity", "")),
        knowledge_class=str(data.get("knowledge_class") or "point"),
        layer=str(data.get("layer", "")),
        tags=tuple(data.get("tags") or ()),
        applicable_phases=tuple(data.get("applicable_phases") or ()),
        evidence=evidence,
        history=tuple(history),
        body=body.strip("\n"),  # normalize: surrounding blank lines are not content
        path=path,
    )


def load_entry(path: Path) -> Entry:
    return parse_entry(path.read_text(encoding="utf-8"), path=path)


def serialize_entry(entry: Entry) -> str:
    """Render an Entry back to Markdown-with-frontmatter (canonical field order)."""
    data = {
        "id": entry.id,
        "title": entry.title,
        "category": entry.category,
        "domain": entry.domain,
        "type": entry.type,
        "guideline_polarity": entry.guideline_polarity,
        "maturity": entry.maturity,
        "knowledge_class": entry.knowledge_class,
        "layer": entry.layer,
        "tags": list(entry.tags),
        "applicable_phases": list(entry.applicable_phases),
        "evidence": {
            "contributors": list(entry.evidence.contributors),
            "sources": list(entry.evidence.sources),
            "projects": list(entry.evidence.projects),
            "last_referenced": entry.evidence.last_referenced,
            "ref_count": entry.evidence.ref_count,
        },
        "history": [dict(h) for h in entry.history],
    }
    buf = io.StringIO()
    buf.write("---\n")
    _yaml.dump(data, buf)
    buf.write("---\n\n")
    buf.write(entry.body.strip("\n") + "\n")  # matches parse_entry normalization (lossless round-trip)
    return buf.getvalue()


def save_entry(path: Path, entry: Entry) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(serialize_entry(entry), encoding="utf-8")
