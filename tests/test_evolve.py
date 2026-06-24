"""B7 · M9 /evolve — proposal lifecycle + the never-auto-apply safety invariant."""

from __future__ import annotations

from pathlib import Path

import pytest

from cairnkit import evolve
from cairnkit.errors import UsageError


def test_propose_creates_pending(tmp_path: Path) -> None:
    p = evolve.propose(tmp_path, "fix-1", "root cause: X\nsuggestion: Y")
    assert p.parent.name == "pending"
    assert evolve.list_proposals(tmp_path, "pending") == ["fix-1"]


def test_propose_duplicate_rejected(tmp_path: Path) -> None:
    evolve.propose(tmp_path, "fix-1", "a")
    with pytest.raises(UsageError):
        evolve.propose(tmp_path, "fix-1", "b")


def test_apply_moves_pending_to_applied(tmp_path: Path) -> None:
    evolve.propose(tmp_path, "fix-1", "a")
    dest = evolve.transition(tmp_path, "fix-1", "applied")
    assert dest.parent.name == "applied"
    assert evolve.list_proposals(tmp_path, "pending") == []
    assert evolve.list_proposals(tmp_path, "applied") == ["fix-1"]


def test_reject_and_defer(tmp_path: Path) -> None:
    evolve.propose(tmp_path, "a", "x")
    evolve.propose(tmp_path, "b", "y")
    evolve.transition(tmp_path, "a", "rejected")
    evolve.transition(tmp_path, "b", "deferred")
    assert evolve.list_proposals(tmp_path, "rejected") == ["a"]
    assert evolve.list_proposals(tmp_path, "deferred") == ["b"]


def test_transition_requires_pending(tmp_path: Path) -> None:
    with pytest.raises(UsageError):
        evolve.transition(tmp_path, "nope", "applied")


@pytest.mark.parametrize("bad", ["../../evil", "with/slash", "new\nline", "tab\there", ""])
def test_invalid_id_rejected(tmp_path: Path, bad: str) -> None:
    with pytest.raises(UsageError):
        evolve.propose(tmp_path, bad, "x")


def test_log_is_appended(tmp_path: Path) -> None:
    evolve.propose(tmp_path, "fix-1", "a")
    evolve.transition(tmp_path, "fix-1", "applied")
    log = (tmp_path / "docs" / "workflows" / "evolve-log" / "log.md").read_text()
    assert "PROPOSE fix-1" in log
    assert "APPLIED fix-1" in log


def test_evolve_never_writes_harness_files(tmp_path: Path) -> None:
    """The core safety invariant: no evolve operation touches agents/ or rules/."""
    (tmp_path / "agents").mkdir()
    agent = tmp_path / "agents" / "dev.md"
    agent.write_text("ORIGINAL", encoding="utf-8")
    (tmp_path / "rules").mkdir()
    rule = tmp_path / "rules" / "r.md"
    rule.write_text("ORIGINAL", encoding="utf-8")

    evolve.propose(tmp_path, "fix-1", "would change dev.md")
    evolve.transition(tmp_path, "fix-1", "applied")

    # applying a proposal must NOT have modified any harness file
    assert agent.read_text() == "ORIGINAL"
    assert rule.read_text() == "ORIGINAL"
