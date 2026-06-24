"""Knowledge maturity lifecycle (M6): promotion, decay, and layer judging.

Maturity is driven by real usage, not time alone:
  - promote: draft→verified on first successful reference; verified→proven on ≥2 projects.
  - decay:   proven 12mo unreferenced → verified; verified 6mo → draft (event-triggered, no cron).
  - judge_layer: where a promoted entry belongs (L3 project / L1 tech / L2 biz).
"""

from __future__ import annotations

from datetime import date

from cairnkit.knowledge.model import Entry

_PROVEN_DECAY_MONTHS = 12
_VERIFIED_DECAY_MONTHS = 6


def months_between(earlier_iso: str, now: date) -> float:
    """Approximate months from an ISO date to ``now`` (public; used by lint)."""
    y, m, d = (int(x) for x in earlier_iso.split("-")[:3])
    return (now.year - y) * 12 + (now.month - m) + (now.day - d) / 30.0


def promote(entry: Entry, now: date | None = None) -> Entry:
    """Apply the usage-driven promotion rule (one level). Returns a new Entry (or the same)."""
    now = now or date.today()
    if entry.maturity == "draft" and entry.evidence.ref_count >= 1:
        return _with_maturity(entry, "verified", now, "promote")
    if entry.maturity == "verified" and len(entry.evidence.projects) >= 2:
        return _with_maturity(entry, "proven", now, "promote")
    return entry


def decay(entry: Entry, now: date | None = None) -> Entry:
    """Demote one level if the entry has gone unreferenced past its half-life window.

    Implements proven→verified (12mo) and verified→draft (6mo). The further
    draft→archived step (02-requirements §6) is intentionally deferred: archival is a
    destructive move best left to an explicit Lint/maintainer action, not auto-decay.
    """
    now = now or date.today()
    last = entry.evidence.last_referenced
    if not last:
        return entry  # never referenced yet — promotion/extraction handles, not decay
    age = months_between(last, now)
    if entry.maturity == "proven" and age >= _PROVEN_DECAY_MONTHS:
        return _with_maturity(entry, "verified", now, "decay")
    if entry.maturity == "verified" and age >= _VERIFIED_DECAY_MONTHS:
        return _with_maturity(entry, "draft", now, "decay")
    return entry


def judge_layer(entry: Entry) -> str:
    """Suggest where a promoted entry belongs (the model/maintainer confirms)."""
    if len(entry.evidence.projects) <= 1:
        return "L3"                      # project-specific until proven across projects
    return "L1" if entry.category == "tech" else "L2"


def _with_maturity(entry: Entry, maturity: str, now: date, why: str) -> Entry:
    hist = entry.history + ({"date": now.isoformat(), "update_type": why, "by": "system"},)
    return entry.with_(maturity=maturity, history=hist)


def promote_repo(kb_root, now: date | None = None) -> list[str]:
    """Promote every eligible entry on disk; return the ids that changed."""
    return _apply_repo(kb_root, lambda e: promote(e, now))


def decay_repo(kb_root, now: date | None = None) -> list[str]:
    """Decay every stale entry on disk; return the ids that changed."""
    return _apply_repo(kb_root, lambda e: decay(e, now))


def _apply_repo(kb_root, fn) -> list[str]:
    from cairnkit.knowledge.index import iter_entries
    from cairnkit.knowledge.model import save_entry

    changed: list[str] = []
    for entry in iter_entries(kb_root):
        new = fn(entry)
        if new.maturity != entry.maturity and entry.path is not None:
            save_entry(entry.path, new)
            changed.append(entry.id)
    return changed
