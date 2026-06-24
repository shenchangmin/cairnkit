"""Three-level progressive index generation (M5).

  A  knowledge-catalog.md         — the ~50-line panorama (counts + per-stage map)
  B  <wiki>/catalog.md            — one line per entry: ID | title | maturity | class | tags
  C  the entry .md files          — the full content (already on disk)

Agents drill A → B → C on demand, so knowing the whole base costs ~50 lines and locating a
relevant entry ~a few hundred — never the whole corpus.
"""

from __future__ import annotations

from collections import Counter
from pathlib import Path

from cairnkit.knowledge.model import Entry, KnowledgeError, load_entry

CATALOG_A = "knowledge-catalog.md"
CATALOG_B = "catalog.md"


def iter_entries(kb_root: Path) -> list[Entry]:
    """Load every entry under tech-wiki/ and biz-wiki/, skipping generated catalogs."""
    entries: list[Entry] = []
    for wiki in ("tech-wiki", "biz-wiki"):
        base = kb_root / wiki
        if not base.exists():
            continue
        for path in sorted(base.rglob("*.md")):
            if path.name == CATALOG_B:
                continue
            try:
                entries.append(load_entry(path))
            except KnowledgeError:
                continue  # a stray non-entry .md must not crash every scan
    return entries


def _line(entry: Entry) -> str:
    tags = ",".join(entry.tags)
    return f"{entry.id} | {entry.title} | {entry.maturity} | {entry.knowledge_class} | {tags}"


def build_index(kb_root: Path) -> dict[str, int]:
    """(Re)generate A and B catalogs from the entries on disk. Returns a small stats dict."""
    kb_root.mkdir(parents=True, exist_ok=True)  # fresh project: knowledge_root may not exist yet
    entries = iter_entries(kb_root)

    # --- B: per-wiki / per-domain catalogs ---
    tech = [e for e in entries if e.category == "tech"]
    if tech:
        _write_catalog(kb_root / "tech-wiki" / CATALOG_B, "Tech knowledge (L1)", tech)
    biz_by_domain: dict[str, list[Entry]] = {}
    for e in entries:
        if e.category == "biz":
            biz_by_domain.setdefault(e.domain or "_", []).append(e)
    for domain, items in biz_by_domain.items():
        _write_catalog(
            kb_root / "biz-wiki" / domain / CATALOG_B, f"Business knowledge — {domain} (L2)", items
        )

    # --- A: panorama ---
    _write_panorama(kb_root / CATALOG_A, entries)

    return {
        "total": len(entries),
        "tech": len(tech),
        "biz": sum(len(v) for v in biz_by_domain.values()),
        "domains": len(biz_by_domain),
    }


def _write_catalog(path: Path, title: str, entries: list[Entry]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [f"# {title}", "", "ID | title | maturity | class | tags", "--- | --- | --- | --- | ---"]
    lines += [_line(e) for e in sorted(entries, key=lambda e: e.id)]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _write_panorama(path: Path, entries: list[Entry]) -> None:
    by_cat = Counter(e.category for e in entries)
    by_mat = Counter(e.maturity for e in entries)
    by_class = Counter(e.knowledge_class for e in entries)
    stage_counts: Counter = Counter()
    for e in entries:
        for ph in e.applicable_phases:
            stage_counts[ph] += 1

    lines = [
        "# Knowledge catalog (panorama)",
        "",
        f"- total: {len(entries)}  ·  tech: {by_cat.get('tech', 0)}  ·  biz: {by_cat.get('biz', 0)}",
        "- maturity: " + ", ".join(f"{k}={by_mat.get(k, 0)}" for k in ("draft", "verified", "proven")),
        "- class: " + ", ".join(f"{k}={by_class.get(k, 0)}" for k in ("point", "causal", "spatiotemporal")),
        "",
        "## Entries applicable per stage",
        "",
        "stage | count",
        "--- | ---",
    ]
    lines += [f"{stage} | {n}" for stage, n in sorted(stage_counts.items())]
    lines += [
        "",
        "> Drill down: read a wiki `catalog.md` (B) for one-line entries, then the entry `.md` (C).",
    ]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
