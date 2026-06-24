"""B6 · M8 import progress — resumable pipeline state."""

from __future__ import annotations

from pathlib import Path

import pytest

from cairnkit import import_state as imp
from cairnkit.errors import StateError, UsageError


def test_init_then_advance_through_pipeline(tmp_path: Path) -> None:
    s = imp.init_import(tmp_path)
    assert s.step == "doc-collect"
    s = imp.advance_import(tmp_path)
    assert s.step == "codebase-profile"
    assert "doc-collect" in s.done
    s = imp.advance_import(tmp_path)
    assert s.step == "knowledge-build"
    s = imp.advance_import(tmp_path)
    assert s.step == "done"


def test_resume_reads_from_disk(tmp_path: Path) -> None:
    imp.init_import(tmp_path)
    imp.advance_import(tmp_path)
    # a fresh load (simulating a new process) sees the persisted progress
    assert imp.load_import(tmp_path).step == "codebase-profile"


def test_init_twice_rejected(tmp_path: Path) -> None:
    imp.init_import(tmp_path)
    with pytest.raises(UsageError):
        imp.init_import(tmp_path)


def test_advance_without_import_errors(tmp_path: Path) -> None:
    with pytest.raises(StateError):
        imp.advance_import(tmp_path)


def test_advance_past_done_errors(tmp_path: Path) -> None:
    imp.init_import(tmp_path)
    for _ in range(3):
        imp.advance_import(tmp_path)
    with pytest.raises(StateError):
        imp.advance_import(tmp_path)
