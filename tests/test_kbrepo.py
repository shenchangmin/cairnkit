"""B5 · M7 cross-project knowledge Git repo (real git in temp dirs)."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from cairnkit.knowledge import kbrepo
from cairnkit.knowledge.kbrepo import KbRepoError
from cairnkit.knowledge.model import load_entry
from tests.knowledge_fixtures import make_entry


def _git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def _repo(tmp_path: Path) -> Path:
    repo = tmp_path / "kb"
    kbrepo.init_repo(repo)
    _git(repo, "config", "user.email", "t@t.t")
    _git(repo, "config", "user.name", "t")
    _git(repo, "add", "-A")
    _git(repo, "commit", "-q", "-m", "init")
    return repo


def test_init_repo_creates_skeleton(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    assert (repo / "tech-wiki").is_dir()
    assert (repo / "biz-wiki").is_dir()
    assert (repo / "log.md").exists()
    assert kbrepo.is_git_repo(repo)


def test_classify_contribution() -> None:
    assert kbrepo.classify_contribution("add") == "auto"
    assert kbrepo.classify_contribution("promote_verified") == "auto"
    assert kbrepo.classify_contribution("conflict") == "review"
    assert kbrepo.classify_contribution("promote_proven") == "review"
    assert kbrepo.classify_contribution("team_convention") == "review"
    with pytest.raises(KbRepoError):
        kbrepo.classify_contribution("bogus")


def test_push_commits_locally_without_remote(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    make_entry(repo, id="TK-1", title="P", phases=["IMPLEMENT"])
    res = kbrepo.push(repo, "add TK-1")
    assert res["committed"] is True
    assert res["pushed"] is False  # no remote -> degraded


def test_push_to_real_remote(tmp_path: Path) -> None:
    bare = tmp_path / "remote.git"
    subprocess.run(["git", "init", "--bare", "-q", str(bare)], check=True)
    repo = _repo(tmp_path)
    _git(repo, "remote", "add", "origin", str(bare))
    _git(repo, "push", "-q", "-u", "origin", "HEAD")
    make_entry(repo, id="TK-2", title="Q", phases=["IMPLEMENT"])
    res = kbrepo.push(repo, "add TK-2")
    assert res["committed"] and res["pushed"]
    # pull on a second clone sees it
    clone = tmp_path / "clone"
    subprocess.run(["git", "clone", "-q", str(bare), str(clone)], check=True)
    assert (clone / "tech-wiki" / "TK-2.md").exists()


def test_pull_without_remote_degrades(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    assert kbrepo.pull(repo)["pulled"] is False


def test_stage_conflict_never_overwrites(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    p1 = kbrepo.stage_conflict(repo, "TK-1", "first", today="2026-06-24")
    p2 = kbrepo.stage_conflict(repo, "TK-1", "second", today="2026-06-24")
    assert p1 != p2  # second conflict does not clobber the first
    assert p1.read_text() == "first"
    assert "CONFLICT staged for TK-1" in (repo / "log.md").read_text()


def test_promote_moves_l3_to_l1(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    make_entry(repo, id="TK-1", title="P", layer="L3", phases=["IMPLEMENT"])
    # entry initially lives under tech-wiki (fixture path) — promote rewrites layer + logs
    dest = kbrepo.promote_entry(repo, "TK-1", "L1")
    assert load_entry(dest).layer == "L1"
    assert "PROMOTE TK-1 -> L1" in (repo / "log.md").read_text()


def test_promote_biz_to_l2(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    make_entry(repo, id="BK-1", title="Flow", category="biz", domain="ads", layer="L3",
               phases=["ANALYSE_PRODUCT"])
    dest = kbrepo.promote_entry(repo, "BK-1", "L2")
    assert dest.parent.name == "ads"
    assert load_entry(dest).layer == "L2"


def test_promote_unknown_entry_errors(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    with pytest.raises(KbRepoError):
        kbrepo.promote_entry(repo, "NOPE", "L1")


def test_promote_rejects_non_l3_entry(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    make_entry(repo, id="TK-1", title="P", layer="L1", phases=["IMPLEMENT"])
    with pytest.raises(KbRepoError):  # already promoted -> not promotable
        kbrepo.promote_entry(repo, "TK-1", "L2")


def test_promote_refuses_to_overwrite_existing_destination(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    # an L3 biz draft lives at biz-wiki/ads/BK-1.md; promoting to L1 targets tech-wiki/BK-1.md
    make_entry(repo, id="BK-1", title="Draft", category="biz", domain="ads", layer="L3",
               phases=["IMPLEMENT"])
    # pre-occupy that destination with a *different* existing entry — must not be clobbered
    occupied = repo / "tech-wiki" / "BK-1.md"
    occupied.parent.mkdir(parents=True, exist_ok=True)
    occupied.write_text("EXISTING — do not overwrite\n", encoding="utf-8")
    with pytest.raises(KbRepoError):
        kbrepo.promote_entry(repo, "BK-1", "L1")
    assert "do not overwrite" in occupied.read_text()  # destination preserved


def test_stage_conflict_rejects_path_traversal(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    with pytest.raises(KbRepoError):
        kbrepo.stage_conflict(repo, "../../evil", "x")
    with pytest.raises(KbRepoError):
        kbrepo.stage_conflict(repo, "bad\nid", "x")


def test_stats(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    make_entry(repo, id="TK-1", title="A", maturity="proven", phases=["IMPLEMENT"])
    make_entry(repo, id="TK-2", title="B", maturity="draft", phases=["IMPLEMENT"])
    make_entry(repo, id="BK-1", title="C", category="biz", domain="ads", layer="L2",
               maturity="verified", phases=["ANALYSE_PRODUCT"])
    s = kbrepo.stats(repo)
    assert s["total"] == 3
    assert s["by_category"]["tech"] == 2
    assert s["by_category"]["biz"] == 1
    assert "TK-1" in s["orphans"]  # proven, never referenced


def test_concurrent_additions_auto_merge(tmp_path: Path) -> None:
    # two contributors add different files to a shared bare repo -> file-level, no conflict
    bare = tmp_path / "remote.git"
    subprocess.run(["git", "init", "--bare", "-q", str(bare)], check=True)
    a = _repo(tmp_path / "a_wrap")
    _git(a, "remote", "add", "origin", str(bare))
    _git(a, "push", "-q", "-u", "origin", "HEAD")
    clone_b = tmp_path / "b"
    subprocess.run(["git", "clone", "-q", str(bare), str(clone_b)], check=True)
    _git(clone_b, "config", "user.email", "b@b.b")
    _git(clone_b, "config", "user.name", "b")

    make_entry(a, id="TK-1", title="A", phases=["IMPLEMENT"])
    kbrepo.push(a, "a adds TK-1")
    make_entry(clone_b, id="TK-2", title="B", phases=["IMPLEMENT"])
    kbrepo.pull(clone_b)               # ff-only picks up TK-1
    res = kbrepo.push(clone_b, "b adds TK-2")
    assert res["committed"] and res["pushed"]
    # both files coexist
    final = tmp_path / "final"
    subprocess.run(["git", "clone", "-q", str(bare), str(final)], check=True)
    assert (final / "tech-wiki" / "TK-1.md").exists()
    assert (final / "tech-wiki" / "TK-2.md").exists()
