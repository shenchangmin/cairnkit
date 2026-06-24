"""Self-evolution proposal lifecycle (M9) — improve the harness, but never automatically.

`/evolve` analyses a bug by replaying the workflow log and writes an improvement *proposal*;
`/evolve:apply` lets a human review and approve it before the harness's own agents/rules are
edited. This module manages only the proposal lifecycle (pending → applied/rejected/deferred)
and the audit log. It deliberately has NO code path that writes to agents/ or rules/ — the
"never auto-apply" guarantee is structural, not a matter of discipline (CLAUDE.md §2).
"""

from __future__ import annotations

import re
from datetime import datetime
from pathlib import Path

from cairnkit.errors import UsageError

_SAFE_ID = re.compile(r"[A-Za-z0-9._-]+")

STATES = ("pending", "applied", "rejected", "deferred")
_ROOT = Path("docs") / "workflows" / "evolve-log"
_LOG = "log.md"


def evolve_dir(root: Path, state: str) -> Path:
    return root / _ROOT / state


def _log(root: Path, line: str) -> None:
    log = root / _ROOT / _LOG
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("a", encoding="utf-8") as fh:
        fh.write(f"{datetime.now().isoformat(timespec='seconds')} {line}\n")


def _safe_id(proposal_id: str) -> str:
    # whitelist slug chars: blocks path traversal AND control-char log injection
    if not proposal_id or not _SAFE_ID.fullmatch(proposal_id) or ".." in proposal_id:
        raise UsageError(f"invalid proposal id {proposal_id!r} (allowed: letters, digits, . _ -)")
    return proposal_id


def propose(root: Path, proposal_id: str, content: str) -> Path:
    """Record a new improvement proposal under pending/ (analysis only — never applied)."""
    proposal_id = _safe_id(proposal_id)
    pending = evolve_dir(root, "pending")
    pending.mkdir(parents=True, exist_ok=True)
    path = pending / f"{proposal_id}.md"
    if path.exists():
        raise UsageError(f"proposal {proposal_id} already exists")
    path.write_text(content, encoding="utf-8")
    _log(root, f"PROPOSE {proposal_id}")
    return path


def list_proposals(root: Path, state: str) -> list[str]:
    if state not in STATES:
        raise UsageError(f"unknown state {state!r}; valid: {STATES}")
    d = evolve_dir(root, state)
    return sorted(p.stem for p in d.glob("*.md")) if d.exists() else []


def transition(root: Path, proposal_id: str, to_state: str) -> Path:
    """Move a proposal from pending to applied/rejected/deferred. Only pending may transition.

    NOTE: this records the human's decision; it does NOT edit any agents/rules file. Applying
    the actual change is a separate, human-driven edit performed before this call.
    """
    proposal_id = _safe_id(proposal_id)
    if to_state not in ("applied", "rejected", "deferred"):
        raise UsageError(f"cannot transition to {to_state!r}")
    src = evolve_dir(root, "pending") / f"{proposal_id}.md"
    if not src.exists():
        raise UsageError(f"no pending proposal {proposal_id}")
    dest_dir = evolve_dir(root, to_state)
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / f"{proposal_id}.md"
    src.rename(dest)
    _log(root, f"{to_state.upper()} {proposal_id}")
    return dest
