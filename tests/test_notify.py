"""B6 · M10 notify — channel abstraction + degradation."""

from __future__ import annotations

from pathlib import Path

from cairnkit import notify as notifier
from cairnkit.config import load_config

YAML_NO_HOOK = "project: demo\nrepos:\n  - name: demo\n    path: .\n"
YAML_HOOK = ("project: demo\nrepos:\n  - name: demo\n    path: .\n"
            "notify:\n  feishu_webhook_env: CAIRNKIT_TEST_HOOK\n")


def test_build_message() -> None:
    assert notifier.build_message("blocked", "p", "BUILD x5") == "[cairnkit:p] blocked — BUILD x5"
    assert notifier.build_message("done", "p") == "[cairnkit:p] done"


def test_degrades_to_local_when_no_webhook(tmp_path: Path) -> None:
    (tmp_path / "cairnkit.yaml").write_text(YAML_NO_HOOK, encoding="utf-8")
    config = load_config(tmp_path)
    res = notifier.notify("clarify_pending", config, detail="approve me")
    assert res["sent"] is False
    assert (tmp_path / ".cairnkit" / "notifications.log").read_text().strip().endswith("approve me")


def test_sends_when_webhook_configured(tmp_path: Path, monkeypatch) -> None:
    (tmp_path / "cairnkit.yaml").write_text(YAML_HOOK, encoding="utf-8")
    monkeypatch.setenv("CAIRNKIT_TEST_HOOK", "https://example.invalid/hook")
    sent = {}
    monkeypatch.setattr(notifier, "_send_feishu", lambda url, text: sent.update(url=url, text=text))
    res = notifier.notify("done", load_config(tmp_path))
    assert res["sent"] is True
    assert sent["url"] == "https://example.invalid/hook"


def test_send_failure_degrades_to_local(tmp_path: Path, monkeypatch) -> None:
    (tmp_path / "cairnkit.yaml").write_text(YAML_HOOK, encoding="utf-8")
    monkeypatch.setenv("CAIRNKIT_TEST_HOOK", "https://example.invalid/hook")

    def boom(url, text):
        raise OSError("network down")

    monkeypatch.setattr(notifier, "_send_feishu", boom)
    res = notifier.notify("blocked", load_config(tmp_path))
    assert res["sent"] is False
    assert "send failed" in res["reason"]
    assert (tmp_path / ".cairnkit" / "notifications.log").exists()
