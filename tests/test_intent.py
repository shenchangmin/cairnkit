"""B3 · intent.py — IntentGate heuristic routing."""

from __future__ import annotations

from cairnkit.intent import classify


def test_short_single_point_change_routes_single() -> None:
    assert classify("fix a typo in the README").path_mode == "single"


def test_backend_only_routes_lite() -> None:
    res = classify("add a coupon redemption endpoint to the order service with rate limiting")
    assert res.path_mode == "lite"


def test_frontend_surface_routes_full() -> None:
    res = classify("build a new checkout page UI component with a styled button and layout")
    assert res.path_mode == "full"


def test_long_typo_like_request_is_not_single() -> None:
    # has 'tweak' but is long & mentions a page -> not a single-point change
    res = classify(
        "tweak the entire onboarding page layout, restructure the component tree, "
        "and redesign the visual hierarchy across every screen in the flow"
    )
    assert res.path_mode != "single"
