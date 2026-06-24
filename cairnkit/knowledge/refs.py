"""Reference-tracking closed loop (M6) — the engine of maturity.

Each role agent records a ``knowledgeReferences`` block in its artifact. ARCHIVE batch-reads
them and writes back into each referenced entry's evidence (last_referenced / ref_count /
projects), which is what later drives promotion. Missing references are not an error — they
simply do not update anything.
"""

from __future__ import annotations

import json
from datetime import date
from pathlib import Path

from cairnkit.knowledge.index import iter_entries
from cairnkit.knowledge.model import Evidence, save_entry


def _top_level_json_objects(text: str):
    """Yield each balanced top-level ``{...}`` substring (robust to nested objects)."""
    depth = 0
    start = None
    for i, ch in enumerate(text):
        if ch == "{":
            if depth == 0:
                start = i
            depth += 1
        elif ch == "}":
            if depth > 0:
                depth -= 1
                if depth == 0 and start is not None:
                    yield text[start:i + 1]
                    start = None


def _ids_in(obj) -> list[str]:
    """Recursively collect ids under any 'knowledgeReferences' list (handles nesting)."""
    out: list[str] = []
    if isinstance(obj, dict):
        for key, val in obj.items():
            if key == "knowledgeReferences" and isinstance(val, list):
                out += [r["id"] for r in val if isinstance(r, dict) and r.get("id")]
            else:
                out += _ids_in(val)
    elif isinstance(obj, list):
        for item in obj:
            out += _ids_in(item)
    return out


def collect_references(run_dir: Path) -> list[str]:
    """Return entry ids referenced across all artifacts in a run directory (may contain dups)."""
    ids: list[str] = []
    if not run_dir.exists():
        return ids
    for path in sorted(run_dir.glob("*.md")):
        text = path.read_text(encoding="utf-8")
        for blob in _top_level_json_objects(text):
            try:
                obj = json.loads(blob)
            except json.JSONDecodeError:
                continue
            ids += _ids_in(obj)
    return ids


def touch(kb_root: Path, run_dir: Path, project: str, today: str | None = None) -> dict:
    """Write back references from a run into the knowledge base evidence. Returns a summary."""
    today = today or date.today().isoformat()
    referenced = collect_references(run_dir)
    counts: dict[str, int] = {}
    for rid in referenced:
        counts[rid] = counts.get(rid, 0) + 1

    updated: list[str] = []
    by_id = {e.id: e for e in iter_entries(kb_root)}
    for rid, n in counts.items():
        entry = by_id.get(rid)
        if entry is None or entry.path is None:
            continue  # unknown reference — not an error, just nothing to update
        ev = entry.evidence
        projects = tuple(dict.fromkeys((*ev.projects, project)))  # dedupe, keep order
        new_ev = Evidence(
            contributors=ev.contributors,
            sources=ev.sources,
            projects=projects,
            last_referenced=today,
            ref_count=ev.ref_count + n,
        )
        save_entry(entry.path, entry.with_(evidence=new_ev))
        updated.append(rid)

    return {"referenced": referenced, "updated": updated, "unknown": [r for r in counts if r not in updated]}
