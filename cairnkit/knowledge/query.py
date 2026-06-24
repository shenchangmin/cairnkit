"""Budget-bounded progressive knowledge injection (M5).

The core context-bloat defence: filter by stage + domain, rank causal/spatiotemporal and
proven first, then cap to a line budget — and **return what was dropped** so the truncation
is never silent (the article's central tension).

Budget policy: the cap is hard for every entry except the single highest-ranked one, which
is always included so the most relevant proven knowledge is never lost; when that one entry
alone exceeds the budget the result sets ``over_budget=True`` (reported, never silent).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from cairnkit.knowledge import CLASS_RANK, MATURITY_RANK
from cairnkit.knowledge.index import iter_entries
from cairnkit.knowledge.model import Entry, serialize_entry


@dataclass(frozen=True)
class QueryResult:
    stage: str
    budget_lines: int
    injected_ids: tuple[str, ...]
    dropped: tuple[dict, ...]
    text: str
    lines: int = field(default=0)
    over_budget: bool = field(default=False)  # True if the single top entry alone exceeds budget


def _rank(entry: Entry) -> tuple[int, int, str]:
    # higher maturity & higher knowledge-class first; id as a stable tiebreak
    return (
        -MATURITY_RANK.get(entry.maturity, 0),
        -CLASS_RANK.get(entry.knowledge_class, 0),
        entry.id,
    )


def _applies(entry: Entry, stage: str, domain: str | None) -> bool:
    if stage not in entry.applicable_phases:
        return False
    if entry.category == "tech":
        return True  # L1 tech knowledge is globally visible
    # biz: only within the current project's domain
    return domain is not None and entry.domain == domain


def query(kb_root: Path, stage: str, budget_lines: int, domain: str | None = None) -> QueryResult:
    """Select stage-relevant entries, rank, and inject up to budget_lines; report dropped."""
    candidates = sorted(
        (e for e in iter_entries(kb_root) if _applies(e, stage, domain)),
        key=_rank,
    )

    injected: list[str] = []
    injected_ids: list[str] = []
    dropped: list[dict] = []
    used = 0          # cumulative injected line count (serialized newlines)
    over_budget = False
    for entry in candidates:
        block = serialize_entry(entry)
        block_lines = block.count("\n")
        if not injected:
            # Always include the single highest-ranked entry so the most relevant proven
            # knowledge is never lost — but flag (never silently) when it alone exceeds budget.
            injected.append(block)
            injected_ids.append(entry.id)
            used += block_lines
            over_budget = block_lines > budget_lines
            continue
        if used + block_lines > budget_lines:
            dropped.append({"id": entry.id, "title": entry.title, "reason": "budget"})
            continue
        injected.append(block)
        injected_ids.append(entry.id)
        used += block_lines

    return QueryResult(
        stage=stage,
        budget_lines=budget_lines,
        injected_ids=tuple(injected_ids),
        dropped=tuple(dropped),
        text="\n".join(injected),
        lines=used,
        over_budget=over_budget,
    )
