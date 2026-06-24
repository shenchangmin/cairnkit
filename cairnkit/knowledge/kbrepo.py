"""Cross-project knowledge Git repository (M7) — the moat itself.

Knowledge precipitates into an *independent* Git repo shared across projects. No DB, no
central service — Git is the sync protocol. This module wraps the deterministic operations
(pull/push, promotion L3→L1/L2, conflict staging, append-only log, health stats) over a
local working copy. Git runs via subprocess (no gitpython dependency).

Contribution flow is hybrid (02-requirements §7): additive/evidence/draft→verified auto-merge
by direct push; contradictions / proven-promotion / L0-T changes go through review (PR).
"""

from __future__ import annotations

import re
import subprocess
from collections import Counter
from datetime import date
from pathlib import Path

from cairnkit.errors import CairnkitError
from cairnkit.knowledge.index import iter_entries

LOG_FILE = "log.md"
CONFLICTS_DIR = "contributions/conflicts"
_SAFE_ID = re.compile(r"^[A-Za-z0-9._-]+$")


def _safe_id(entry_id: str) -> str:
    """Reject ids that could escape a directory or inject into the log (path traversal / newlines)."""
    if ".." in entry_id or not _SAFE_ID.match(entry_id):
        raise KbRepoError(f"invalid entry id {entry_id!r} (allowed: letters, digits, . _ -)")
    return entry_id

# Contribution kinds that may auto-merge vs. must go through review.
_AUTO_MERGE = {"add", "evidence", "promote_verified"}
_NEEDS_REVIEW = {"conflict", "promote_proven", "team_convention"}


class KbRepoError(CairnkitError):
    code = 2


def git(repo: Path, *args: str) -> str:
    """Run a git command in ``repo``; return stdout. Raises KbRepoError on failure."""
    proc = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True, text=True,
    )
    if proc.returncode != 0:
        raise KbRepoError(f"git {' '.join(args)} failed: {proc.stderr.strip()}")
    return proc.stdout.strip()


def is_git_repo(path: Path) -> bool:
    return (path / ".git").exists()


def has_remote(repo: Path) -> bool:
    try:
        return bool(git(repo, "remote"))
    except KbRepoError:
        return False


def init_repo(repo: Path) -> None:
    """Initialise a fresh knowledge repo skeleton (idempotent)."""
    repo.mkdir(parents=True, exist_ok=True)
    if not is_git_repo(repo):
        git(repo, "init", "-q")
    for sub in ("tech-wiki", "biz-wiki", "team-conventions", CONFLICTS_DIR):
        (repo / sub).mkdir(parents=True, exist_ok=True)
    log = repo / LOG_FILE
    if not log.exists():
        log.write_text("# Knowledge contribution log (append-only)\n", encoding="utf-8")


def pull(repo: Path) -> dict:
    """Pull from the remote; degrade to local-only when there is no remote/network."""
    if not has_remote(repo):
        return {"pulled": False, "reason": "no remote configured (local-only mode)"}
    try:
        out = git(repo, "pull", "--ff-only")
        return {"pulled": True, "detail": out}
    except KbRepoError as exc:
        return {"pulled": False, "reason": str(exc)}


def push(repo: Path, message: str) -> dict:
    """Commit all changes; push if a remote exists, else commit locally (degraded)."""
    git(repo, "add", "-A")
    status = git(repo, "status", "--porcelain")
    if not status:
        return {"committed": False, "reason": "nothing to commit"}
    git(repo, "commit", "-q", "-m", message)
    if not has_remote(repo):
        return {"committed": True, "pushed": False, "reason": "no remote (committed locally)"}
    try:
        git(repo, "push")
        return {"committed": True, "pushed": True}
    except KbRepoError as exc:
        return {"committed": True, "pushed": False, "reason": str(exc)}


def append_log(repo: Path, line: str) -> None:
    """Append one line to the append-only contribution log (open in append mode, O(1))."""
    with (repo / LOG_FILE).open("a", encoding="utf-8") as fh:
        fh.write(line.replace("\n", " ").rstrip() + "\n")


def classify_contribution(kind: str) -> str:
    """Return 'auto' (direct push + auto-merge) or 'review' (PR + maintainer approval)."""
    if kind in _AUTO_MERGE:
        return "auto"
    if kind in _NEEDS_REVIEW:
        return "review"
    raise KbRepoError(f"unknown contribution kind {kind!r}")


def stage_conflict(repo: Path, entry_id: str, body: str, today: str | None = None) -> Path:
    """Park a contradicting contribution under contributions/conflicts/ (never overwrite)."""
    entry_id = _safe_id(entry_id)  # block path traversal / log injection
    today = today or date.today().isoformat()
    target = repo / CONFLICTS_DIR
    target.mkdir(parents=True, exist_ok=True)
    path = target / f"{entry_id}-{today}.md"
    n = 1
    while path.exists():  # never clobber an existing conflict record
        path = target / f"{entry_id}-{today}-{n}.md"
        n += 1
    path.write_text(body, encoding="utf-8")
    append_log(repo, f"- {today} CONFLICT staged for {entry_id} -> {path.name}")
    return path


def promote_entry(repo: Path, entry_id: str, target_layer: str) -> Path:
    """Promote an L3 entry to L1 (tech-wiki) or L2 (biz-wiki/<domain>). Returns the new path.

    Only L3 entries may be promoted, and an existing entry at the destination is never
    overwritten — both guards protect cross-project assets from silent data loss.
    """
    _safe_id(entry_id)
    if target_layer not in ("L1", "L2"):
        raise KbRepoError("promotion target must be L1 or L2")
    # only L3 entries are promotable — never select/move an already-promoted entry
    match = next((e for e in iter_entries(repo) if e.id == entry_id and e.layer == "L3"), None)
    if match is None or match.path is None:
        raise KbRepoError(f"no L3 entry with id {entry_id} found in {repo}")

    if target_layer == "L1":
        dest = repo / "tech-wiki" / match.path.name
    else:
        dest = repo / "biz-wiki" / (match.domain or "_") / match.path.name
    if dest.exists() and dest != match.path:
        raise KbRepoError(f"destination {dest.name} already exists — refusing to overwrite")
    dest.parent.mkdir(parents=True, exist_ok=True)

    from cairnkit.knowledge.model import save_entry
    save_entry(dest, match.with_(layer=target_layer))
    if match.path != dest:
        match.path.unlink()
    append_log(repo, f"- {date.today().isoformat()} PROMOTE {entry_id} -> {target_layer}")
    return dest


def stats(repo: Path) -> dict:
    """Scan the repo and compute a health report (zero DB, offline)."""
    entries = list(iter_entries(repo))
    by_mat = Counter(e.maturity for e in entries)
    by_cat = Counter(e.category for e in entries)
    referenced = sum(1 for e in entries if e.evidence.ref_count > 0)
    orphans = [e.id for e in entries if e.evidence.ref_count == 0 and e.maturity != "draft"]
    return {
        "total": len(entries),
        "by_maturity": dict(by_mat),
        "by_category": dict(by_cat),
        "referenced": referenced,
        "reference_rate": round(referenced / len(entries), 2) if entries else 0.0,
        "orphans": orphans,
    }


def stats_markdown(repo: Path) -> str:
    s = stats(repo)
    lines = [
        "# Knowledge base health",
        "",
        f"- total entries: {s['total']}",
        f"- by maturity: {s['by_maturity']}",
        f"- tech/biz: {s['by_category']}",
        f"- reference rate: {s['reference_rate']}  ({s['referenced']}/{s['total']})",
        f"- orphans (non-draft, never referenced): {len(s['orphans'])}",
    ]
    return "\n".join(lines) + "\n"
