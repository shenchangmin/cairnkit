"""T2 · config.py — models + persistence (RED first)."""

from __future__ import annotations

from pathlib import Path

import pytest

from cairnkit import config as cfg
from cairnkit.errors import ConfigError, StateCorruptError


# --- load_config -----------------------------------------------------------

def test_load_config_reads_valid_yaml(project: Path) -> None:
    # Act
    c = cfg.load_config(project)
    # Assert
    assert c.project == "demo-task"
    assert c.domain is None
    assert c.root == project
    assert len(c.repos) == 1
    assert c.repos[0].name == "demo-task"
    assert c.repos[0].path == "."


def test_load_config_missing_raises_config_error(tmp_path: Path) -> None:
    with pytest.raises(ConfigError):
        cfg.load_config(tmp_path)


def test_load_config_bad_yaml_raises_config_error(tmp_path: Path) -> None:
    (tmp_path / "cairnkit.yaml").write_text("project: [unterminated\n", encoding="utf-8")
    with pytest.raises(ConfigError):
        cfg.load_config(tmp_path)


def test_load_config_missing_project_raises(tmp_path: Path) -> None:
    (tmp_path / "cairnkit.yaml").write_text("domain: null\n", encoding="utf-8")
    with pytest.raises(ConfigError):
        cfg.load_config(tmp_path)


def test_load_config_malformed_repos_raises_config_error(tmp_path: Path) -> None:
    # repos entry without a 'name' must surface as ConfigError, not a raw TypeError/KeyError
    (tmp_path / "cairnkit.yaml").write_text(
        "project: x\nrepos:\n  - path: .\n", encoding="utf-8"
    )
    with pytest.raises(ConfigError):
        cfg.load_config(tmp_path)


def test_load_config_empty_repos_uses_single_repo_default(tmp_path: Path) -> None:
    (tmp_path / "cairnkit.yaml").write_text("project: solo\nrepos: []\n", encoding="utf-8")
    c = cfg.load_config(tmp_path)
    assert len(c.repos) == 1
    assert c.repos[0].name == "solo"


def test_config_paths(project: Path) -> None:
    c = cfg.load_config(project)
    assert c.state_path == project / ".cairnkit" / "STATE.yaml"
    assert c.run_dir("2026-06-24-demo") == project / "docs" / "workflows" / "2026-06-24-demo"


# --- init_state / save / load ---------------------------------------------

def test_init_state_writes_initial_state(project: Path) -> None:
    c = cfg.load_config(project)
    # Act
    st = cfg.init_state(c, "2026-06-24-demo")
    # Assert
    assert st.run_id == "2026-06-24-demo"
    assert st.stage == "INIT"
    assert st.path_mode == "full"
    assert st.history == ()
    assert st.artifacts == {}
    assert st.retries == {}
    assert st.pending_clarify is None
    assert st.updated_at  # non-empty timestamp
    assert c.state_path.exists()


def test_save_state_overwrites_updated_at_and_roundtrips(project: Path) -> None:
    c = cfg.load_config(project)
    st = cfg.init_state(c, "2026-06-24-demo")
    # Arrange: craft a new state with a stale timestamp
    stale = st.with_(stage="ANALYSE_PRODUCT", updated_at="1999-01-01T00:00:00")
    # Act
    cfg.save_state(c.state_path, stale)
    reloaded = cfg.load_state(c.state_path)
    # Assert: Python re-stamps updated_at (not the stale value)
    assert reloaded.stage == "ANALYSE_PRODUCT"
    assert reloaded.updated_at != "1999-01-01T00:00:00"


def test_save_state_preserves_field_order(project: Path) -> None:
    from ruamel.yaml import YAML

    c = cfg.load_config(project)
    # populate history so the test exercises a multi-line list field, not just []
    st = cfg.init_state(c, "2026-06-24-demo").with_(
        history=("INIT", "ANALYSE_PRODUCT"), stage="CLARIFY_PRODUCT",
    )
    cfg.save_state(c.state_path, st)
    data = YAML().load(c.state_path.read_text(encoding="utf-8"))
    assert list(data.keys()) == [
        "run_id", "stage", "path_mode", "history",
        "artifacts", "retries", "pending_clarify", "blocked_reason", "updated_at",
    ]


def test_pending_clarify_none_serialized_as_explicit_null(project: Path) -> None:
    c = cfg.load_config(project)
    cfg.save_state(c.state_path, cfg.init_state(c, "2026-06-24-demo"))
    text = c.state_path.read_text(encoding="utf-8")
    assert "pending_clarify: null" in text


def test_state_mappings_are_read_only(project: Path) -> None:
    c = cfg.load_config(project)
    st = cfg.init_state(c, "2026-06-24-demo")
    with pytest.raises(TypeError):
        st.artifacts["x"] = "y"  # type: ignore[index]
    with pytest.raises(TypeError):
        st.retries["x"] = 1  # type: ignore[index]


def test_load_state_missing_file_raises_corrupt(project: Path) -> None:
    c = cfg.load_config(project)
    with pytest.raises(StateCorruptError):
        cfg.load_state(c.state_path)


def test_load_state_without_blocked_reason_is_backward_compatible(project: Path) -> None:
    # an older STATE.yaml omits blocked_reason -> loads with None, not corrupt
    c = cfg.load_config(project)
    cfg.init_state(c, "2026-06-24-demo")
    text = c.state_path.read_text(encoding="utf-8")
    text = "\n".join(ln for ln in text.splitlines() if not ln.startswith("blocked_reason"))
    c.state_path.write_text(text + "\n", encoding="utf-8")
    assert cfg.load_state(c.state_path).blocked_reason is None


def test_load_state_unknown_path_mode_raises_corrupt(project: Path) -> None:
    c = cfg.load_config(project)
    cfg.init_state(c, "2026-06-24-demo")
    text = c.state_path.read_text(encoding="utf-8").replace("path_mode: full", "path_mode: turbo")
    c.state_path.write_text(text, encoding="utf-8")
    with pytest.raises(StateCorruptError):
        cfg.load_state(c.state_path)


def test_load_state_unknown_stage_raises_corrupt(project: Path) -> None:
    c = cfg.load_config(project)
    cfg.init_state(c, "2026-06-24-demo")
    text = c.state_path.read_text(encoding="utf-8").replace("stage: INIT", "stage: GARBAGE")
    c.state_path.write_text(text, encoding="utf-8")
    with pytest.raises(StateCorruptError):
        cfg.load_state(c.state_path)


def test_state_with_returns_new_copy(project: Path) -> None:
    c = cfg.load_config(project)
    st = cfg.init_state(c, "2026-06-24-demo")
    # Act
    nxt = st.with_(stage="ANALYSE_PRODUCT")
    # Assert: original unchanged (immutability)
    assert st.stage == "INIT"
    assert nxt.stage == "ANALYSE_PRODUCT"
    assert nxt is not st


def test_load_state_missing_field_raises_corrupt(project: Path) -> None:
    c = cfg.load_config(project)
    c.state_path.parent.mkdir(parents=True, exist_ok=True)
    c.state_path.write_text("stage: INIT\n", encoding="utf-8")  # missing required fields
    with pytest.raises(StateCorruptError):
        cfg.load_state(c.state_path)


def test_load_state_bad_yaml_raises_corrupt(project: Path) -> None:
    c = cfg.load_config(project)
    c.state_path.parent.mkdir(parents=True, exist_ok=True)
    c.state_path.write_text("::: not valid yaml :::\n", encoding="utf-8")
    with pytest.raises(StateCorruptError):
        cfg.load_state(c.state_path)
