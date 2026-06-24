"""cairnkit — the deterministic Python core of the knowledge-precipitation harness.

The Markdown layer (skills/agents/commands) drives the fuzzy work; this package owns
the mechanical, verifiable work: state-machine transitions, stage admission gates,
and (in later batches) knowledge index/query/lifecycle. Everything here is pytest-able
without Claude Code. See CLAUDE.md and _dev/05-design.md.
"""

__version__ = "0.1.0"
