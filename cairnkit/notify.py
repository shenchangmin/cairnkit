"""Notification dispatch (M10) — reach people at key moments.

v1 channel is a Feishu/Lark webhook bot, triggered by hooks at CLARIFY-pending / blocked /
DONE. The channel is pluggable and the webhook URL is read from an environment variable named
in config — never hardcoded. With no webhook configured (or a send failure) it degrades to a
local file reminder so the workflow never breaks.
"""

from __future__ import annotations

import json
import os
import urllib.request

EVENTS = ("clarify_pending", "blocked", "done", "archived", "arch_review")
_LOCAL_FILE = ".cairnkit/notifications.log"


def build_message(event: str, project: str, detail: str = "") -> str:
    return f"[cairnkit:{project}] {event}" + (f" — {detail}" if detail else "")


def notify(event: str, config, detail: str = "", channel: str = "feishu") -> dict:
    """Send a notification, degrading to a local file when no webhook is configured/reachable."""
    text = build_message(event, config.project, detail)
    env_name = getattr(config, "notify_webhook_env", None)
    url = os.environ.get(env_name) if env_name else None
    if not url:
        return _local(config, text, reason="no webhook configured")
    try:
        _send_feishu(url, text)
        return {"sent": True, "channel": channel}
    except Exception as exc:  # network/HTTP failure -> degrade, never break the flow
        return _local(config, text, reason=f"send failed: {exc}")


def _send_feishu(url: str, text: str) -> None:
    payload = json.dumps({"msg_type": "text", "content": {"text": text}}).encode("utf-8")
    req = urllib.request.Request(  # noqa: S310 - url comes from operator config/env
        url, data=payload, headers={"Content-Type": "application/json"}
    )
    urllib.request.urlopen(req, timeout=10).read()  # noqa: S310


def _local(config, text: str, reason: str) -> dict:
    path = config.root / _LOCAL_FILE
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as fh:
        fh.write(text + "\n")
    return {"sent": False, "local": str(path.relative_to(config.root)), "reason": reason}
