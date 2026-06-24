"""IntentGate — route a request onto a path mode (M2).

A deterministic heuristic the orchestrator can use as a default; the model is free to
override the suggestion with ``state set-path-mode``. Keeping a testable Python heuristic
means small requests do not silently run the full 16-stage pipeline.
"""

from __future__ import annotations

from dataclasses import dataclass

# Signals that a request is a tiny single-point change.
_SINGLE_HINTS = (
    "typo", "rename", "bump", "comment", "log message", "one line", "one-line",
    "tweak", "constant", "config value", "version",
)
# Signals there is no UI/frontend surface (backend-only → lite path).
_FRONTEND_HINTS = (
    "ui", "page", "screen", "component", "css", "frontend", "front-end", "visual",
    "button", "layout", "style",
)


@dataclass(frozen=True)
class IntentResult:
    path_mode: str
    reason: str


def classify(text: str) -> IntentResult:
    """Suggest full | lite | single from the request text (heuristic, overridable)."""
    low = text.lower()
    words = len(low.split())

    if words <= 12 and any(h in low for h in _SINGLE_HINTS):
        return IntentResult("single", "short single-point change (keyword + brevity)")
    if not any(h in low for h in _FRONTEND_HINTS):
        return IntentResult("lite", "no frontend/UI surface detected — backend-only path")
    return IntentResult("full", "frontend surface present — full pipeline")
