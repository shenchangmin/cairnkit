"""The full 16-stage delivery pipeline metadata (M2, completed in B3).

Single source of truth for: the canonical stage order, each stage's produced artifact,
which stages are CLARIFY pauses, which are retryable verify gates, and the path-mode
stage sets (IntentGate routes small work onto a shorter path). state.py/gate.py consume this.
"""

from __future__ import annotations

# Canonical order. advance() walks this, skipping stages not in the active path mode.
FULL_SEQUENCE = (
    "INIT",
    "INTENT_GATE",
    "ANALYSE_PRODUCT",
    "CLARIFY_PRODUCT",
    "ANALYSE_TECH",
    "CLARIFY_TECH",
    "ARCHITECT_BACKEND",
    "CLARIFY_ARCH_BACKEND",
    "ARCHITECT_FRONTEND",
    "CLARIFY_ARCH_FRONTEND",
    "IMPLEMENT",
    "BUILD_VERIFY",
    "VISUAL_REVIEW",
    "E2E_VERIFY",
    "TEST",
    "ARCHIVE",
    "DONE",
)

# Artifact a stage produces (verified on advance out of that stage). Others produce nothing.
STAGE_ARTIFACT = {
    "ANALYSE_PRODUCT": "01-product.md",
    "ANALYSE_TECH": "02-tech.md",
    "ARCHITECT_BACKEND": "03-arch.md",
    "ARCHITECT_FRONTEND": "04-arch-fe.md",
    "IMPLEMENT": "05-implement.md",
    "BUILD_VERIFY": "06-build.md",
    "VISUAL_REVIEW": "07-visual.md",
    "E2E_VERIFY": "08-e2e.md",
    "TEST": "09-test.md",
    "ARCHIVE": "10-archive.md",
}

CLARIFY_STAGES = frozenset({
    "CLARIFY_PRODUCT", "CLARIFY_TECH", "CLARIFY_ARCH_BACKEND", "CLARIFY_ARCH_FRONTEND",
})

# Verify gates that loop on failure with a retry cap before being blocked.
RETRY_STAGES = frozenset({"BUILD_VERIFY", "E2E_VERIFY"})
RETRY_CAP = 5

# The role agent the orchestrator dispatches at each stage (Markdown layer).
STAGE_AGENT = {
    "ANALYSE_PRODUCT": "product",
    "ANALYSE_TECH": "tech",
    "ARCHITECT_BACKEND": "architect-be",
    "ARCHITECT_FRONTEND": "architect-fe",
    "IMPLEMENT": "dev",
    "BUILD_VERIFY": "verify",
    "VISUAL_REVIEW": "visual",
    "E2E_VERIFY": "verify",
    "TEST": "verify",
    "ARCHIVE": "archiver",
}

PATH_MODES = ("full", "lite", "single")

# Stages excluded from the shorter paths (IntentGate routing).
_LITE_EXCLUDE = frozenset({"ARCHITECT_FRONTEND", "CLARIFY_ARCH_FRONTEND", "VISUAL_REVIEW"})
_SINGLE_INCLUDE = frozenset({
    "INIT", "INTENT_GATE", "IMPLEMENT", "BUILD_VERIFY", "TEST", "ARCHIVE", "DONE",
})


def stages_for(path_mode: str) -> tuple[str, ...]:
    """The ordered stages active for a path mode. Raises on an unknown mode (no silent fallback)."""
    if path_mode == "single":
        return tuple(s for s in FULL_SEQUENCE if s in _SINGLE_INCLUDE)
    if path_mode == "lite":
        return tuple(s for s in FULL_SEQUENCE if s not in _LITE_EXCLUDE)
    if path_mode == "full":
        return FULL_SEQUENCE
    raise ValueError(f"unknown path_mode {path_mode!r}; valid: {PATH_MODES}")


def next_stage(current: str, path_mode: str) -> str | None:
    """The next active stage after ``current`` for the path mode, or None at the end."""
    active = stages_for(path_mode)
    if current not in active:
        # current was skipped/foreign — fall back to its position in the full sequence
        idx = FULL_SEQUENCE.index(current)
        for s in FULL_SEQUENCE[idx + 1:]:
            if s in active:
                return s
        return None
    i = active.index(current)
    return active[i + 1] if i + 1 < len(active) else None


def is_clarify(stage: str) -> bool:
    return stage in CLARIFY_STAGES
