"""Typed errors for the cairnkit core, each mapped to a CLI return code.

Return-code convention (see _dev/batches/B1/01-spec.md §4):
    0  success
    2  usage / argument / illegal-enum error
    3  admission-gate / precondition not satisfied
    4  STATE.yaml corrupt / unreadable
"""

from __future__ import annotations


class CairnkitError(Exception):
    """Base class. ``code`` is the CLI exit code this error maps to."""

    code: int = 1


class UsageError(CairnkitError):
    """Bad arguments, illegal stage enum, or no next stage from a terminal state."""

    code = 2


class ConfigError(UsageError):
    """cairnkit.yaml missing or invalid — host project not initialised (run /team-init)."""


class StateError(UsageError):
    """Illegal state operation, e.g. advancing past the terminal DONE stage."""


class GateError(CairnkitError):
    """Admission gate refused the transition: upstream artifacts missing/invalid."""

    code = 3

    def __init__(self, message: str, *, missing: tuple[str, ...] = ()) -> None:
        super().__init__(message)
        self.missing = missing


class StateCorruptError(CairnkitError):
    """STATE.yaml is unreadable or missing required fields."""

    code = 4
