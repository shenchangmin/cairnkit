"""Strict extraction gate (M6) — keep the knowledge base small and precise.

The archiver agent proposes candidate knowledge (semantic work it does by reading the run's
artifacts); this gate is the deterministic filter that decides which candidates are worth
storing. Borrowed from MemOS CREATE_EVAL: a candidate must be reproducible, transferable, and
carry technical depth, or it is dropped. Noise is worse than nothing.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import date
from pathlib import Path

from cairnkit.knowledge import KNOWLEDGE_CLASSES, TYPES
from cairnkit.knowledge.model import Entry, Evidence, save_entry
from cairnkit.knowledge.schema import iter_errors

_MIN_BODY_CHARS = 80          # depth: a one-liner is not durable knowledge
CANDIDATES_FILE = "knowledge-candidates.json"


@dataclass(frozen=True)
class GateVerdict:
    accepted: bool
    reasons: tuple[str, ...]


def evaluate(candidate: dict) -> GateVerdict:
    """Decide whether a candidate knowledge dict passes the strict gate."""
    reasons: list[str] = []
    if not str(candidate.get("id", "")).strip():
        reasons.append("missing id")
    body = str(candidate.get("body", "")).strip()
    if len(body) < _MIN_BODY_CHARS:
        reasons.append("insufficient depth (body too short)")
    if not candidate.get("applicable_phases"):
        reasons.append("not transferable (no applicable_phases)")
    if candidate.get("type") not in TYPES:
        reasons.append("missing/invalid type")
    kc = candidate.get("knowledge_class", "point")
    if kc not in KNOWLEDGE_CLASSES:
        reasons.append("invalid knowledge_class")
    if not str(candidate.get("title", "")).strip():
        reasons.append("missing title")
    return GateVerdict(accepted=not reasons, reasons=tuple(reasons))


def filter_candidates(candidates: list[dict]) -> tuple[list[dict], list[dict]]:
    """Split candidates into (accepted, rejected-with-reasons)."""
    accepted: list[dict] = []
    rejected: list[dict] = []
    for c in candidates:
        verdict = evaluate(c)
        if verdict.accepted:
            accepted.append(c)
        else:
            rejected.append({"title": c.get("title", "<no title>"), "reasons": list(verdict.reasons)})
    return accepted, rejected


def _candidate_to_entry(c: dict, now: str) -> Entry:
    return Entry(
        id=str(c["id"]),
        title=str(c.get("title", "")),
        category=str(c.get("category", "")),
        domain=c.get("domain"),
        type=str(c.get("type", "")),
        guideline_polarity=c.get("guideline_polarity"),
        maturity="draft",                       # extraction always lands as draft
        knowledge_class=str(c.get("knowledge_class") or "point"),
        layer=str(c.get("layer", "L3")),
        tags=tuple(c.get("tags") or ()),
        applicable_phases=tuple(c.get("applicable_phases") or ()),
        evidence=Evidence(contributors=tuple(c.get("contributors") or ())),
        history=({"date": now, "update_type": "extract", "by": "archiver"},),
        body=str(c.get("body", "")),
    )


def _entry_path(kb_root: Path, entry: Entry) -> Path:
    if entry.category == "biz":
        return kb_root / "biz-wiki" / (entry.domain or "_") / f"{entry.id}.md"
    return kb_root / "tech-wiki" / f"{entry.id}.md"


def extract_from_run(run_dir: Path, kb_root: Path, now: str | None = None) -> dict:
    """Read the archiver's candidate file, apply the gate + schema, write accepted drafts."""
    now = now or date.today().isoformat()
    candidates_file = run_dir / CANDIDATES_FILE
    if not candidates_file.exists():
        return {"written": [], "rejected": [], "note": f"no {CANDIDATES_FILE} in {run_dir}"}
    try:
        candidates = json.loads(candidates_file.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        return {"written": [], "rejected": [], "error": f"malformed {CANDIDATES_FILE}: {exc}"}
    if not isinstance(candidates, list):
        return {"written": [], "rejected": [], "error": f"{CANDIDATES_FILE} must be a JSON list"}
    accepted, rejected = filter_candidates(candidates)

    written: list[str] = []
    for c in accepted:
        entry = _candidate_to_entry(c, now)
        errs = list(iter_errors(entry))
        if errs:
            rejected.append({"title": entry.title, "reasons": [f"schema: {'; '.join(errs)}"]})
            continue
        save_entry(_entry_path(kb_root, entry), entry)
        written.append(entry.id)
    return {"written": written, "rejected": rejected}
