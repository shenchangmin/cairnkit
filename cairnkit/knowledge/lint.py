"""Knowledge base Lint (M6) — keep the corpus healthy.

Borrowing Karpathy's LLM-Wiki idea: detect contradictions, orphans, stale entries, duplicates,
schema violations, and index drift. Mechanical fixes (index rebuild) can be auto-applied;
content contradictions are never auto-resolved — they are surfaced for a maintainer.
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
from datetime import date
from pathlib import Path

from cairnkit.knowledge.index import build_index, iter_entries
from cairnkit.knowledge.lifecycle import months_between
from cairnkit.knowledge.schema import iter_errors

_STALE_MONTHS = 12


@dataclass(frozen=True)
class LintReport:
    orphans: tuple[str, ...] = ()           # referenced-never, low credibility
    stale: tuple[str, ...] = ()             # past the staleness window
    duplicates: tuple[tuple[str, ...], ...] = ()  # groups of likely-duplicate ids
    invalid: tuple[str, ...] = ()           # schema violations: "id: msg"
    conflicts: tuple[tuple[str, ...], ...] = ()   # contradiction candidates (never auto-fixed)
    fixed: tuple[str, ...] = ()             # mechanical fixes applied

    @property
    def clean(self) -> bool:
        return not (self.orphans or self.stale or self.duplicates or self.invalid or self.conflicts)


def lint(kb_root: Path, fix: bool = False, now: date | None = None) -> LintReport:
    now = now or date.today()
    entries = iter_entries(kb_root)

    orphans = tuple(
        e.id for e in entries if e.evidence.ref_count == 0 and e.maturity != "draft"
    )
    stale = tuple(
        e.id for e in entries
        if e.evidence.last_referenced
        and months_between(e.evidence.last_referenced, now) >= _STALE_MONTHS
    )

    invalid: list[str] = []
    for e in entries:
        errs = list(iter_errors(e))
        if errs:
            invalid.append(f"{e.id}: {'; '.join(errs)}")

    # duplicate candidates: same normalized title
    by_title: dict[str, list[str]] = defaultdict(list)
    for e in entries:
        by_title[e.title.strip().lower()].append(e.id)
    duplicates = tuple(tuple(ids) for ids in by_title.values() if len(ids) > 1)

    # contradiction candidates: a recommend and an avoid guideline sharing a tag set
    conflicts = _find_conflicts(entries)

    fixed: list[str] = []
    if fix:
        build_index(kb_root)  # mechanical: index always safe to regenerate
        fixed.append("rebuilt index")

    return LintReport(
        orphans=orphans, stale=stale, duplicates=duplicates,
        invalid=tuple(invalid), conflicts=conflicts, fixed=tuple(fixed),
    )


def _find_conflicts(entries) -> tuple[tuple[str, ...], ...]:
    recs = [e for e in entries if e.type == "guideline" and e.guideline_polarity == "recommend"]
    avos = [e for e in entries if e.type == "guideline" and e.guideline_polarity == "avoid"]
    out: list[tuple[str, ...]] = []
    for r in recs:
        for a in avos:
            if set(r.tags) and set(r.tags) == set(a.tags):
                out.append((r.id, a.id))
    return tuple(out)
