"""Shared pytest fixtures for the cairnkit core tests."""

from __future__ import annotations

from pathlib import Path

import pytest

CAIRNKIT_YAML = """\
project: demo-task
domain: null
repos:
  - name: demo-task
    path: .
"""


@pytest.fixture
def project(tmp_path: Path) -> Path:
    """A host project root with a valid cairnkit.yaml (no STATE yet)."""
    (tmp_path / "cairnkit.yaml").write_text(CAIRNKIT_YAML, encoding="utf-8")
    return tmp_path


def write_artifact(root: Path, run_id: str, name: str, body: str = "content") -> Path:
    """Helper: create docs/workflows/<run_id>/<name> with non-empty body."""
    run_dir = root / "docs" / "workflows" / run_id
    run_dir.mkdir(parents=True, exist_ok=True)
    path = run_dir / name
    path.write_text(body, encoding="utf-8")
    return path
