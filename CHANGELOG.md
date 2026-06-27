# Changelog

All notable changes to cairnkit are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **Knowledge-loop end-to-end proof tests** — `tests/cli.rs` now drives the real `cairn` binary
  through the full moat loop offline (seed → `build-index` → query → `extract` via the gate →
  re-index → a second query that returns the just-extracted entry), proving knowledge *compounds*
  across runs — the project's central claim, previously unverified by any automated test. Adds
  focused guards for the two never-silent budget signals (`dropped` for truncated follow-on
  entries, `over_budget` for an oversized top entry) and for extract-gate rejection-with-reasons.
- **Codex harness adapter** — cairnkit now ships a Codex (`AGENTS.md`) form alongside Claude Code.
  Reorganized as **one shared engine (`cairn`) + shared role/command content + per-harness
  adapters** (the everything-claude-code pattern): the same `agents/*.md` role files and the
  `cairn` engine drive both harnesses. Both get **real role isolation** — Claude Code via Task
  sub-agents, Codex via its **native multi-agent** (`multi_agent=true` + per-role
  `.codex/agents/<role>.toml` with their own reasoning effort / sandbox / `developer_instructions`),
  dispatched by the parent orchestrator. Adds `scripts/sync-to-codex.sh` (merge-safe projection
  into `~/.codex/`), `.codex/` baseline + the 11 role agents, and `docs/ADAPTERS.md`.

### Changed
- **IntentGate classification moved to the model layer.** The Rust keyword heuristic mis-routed
  non-English / governance requests; classification is now the orchestrator's (model's) job, with
  the heuristic demoted to a safe fallback that defaults to `full` when unsure.
- **`main` is PR-protected** — all changes land via pull request, maintainers included.
- **Release CI**: the x86_64 macOS binary is built on an Apple Silicon runner (the macos-13
  Intel runner is being retired and was hanging).

## [0.1.0] — 2026-06-27

First public release. The deterministic core is a single `cairn` binary (Rust, zero runtime
dependencies) packaged with a Claude Code plugin.

### Added

- **Orchestration** — 16-stage file-as-state-machine (`INIT → … → DONE`) with IntentGate
  routing (full / lite / single), CLARIFY async pauses, verify-stage retry + block, and
  automatic crash-resume (state lives entirely in `.cairnkit/STATE.yaml`).
- **Role agents (11)** — product, tech, architect-be/-fe, dev, verify, visual, archiver, plus
  the 3 cold-start import agents, and the `workflow-orchestrator` skill that drives them.
- **Knowledge model & store** — Markdown + YAML-frontmatter entries; tech/biz classification,
  five types, three maturities, and a knowledge-class axis; schema validation.
- **3-level progressive index + budget query** — panorama → category catalog → full entry,
  with a hard line budget whose truncation is never silent (`over_budget` flag + reported `dropped`).
- **Knowledge lifecycle** — usage-driven maturity promotion, event-triggered decay, Lint
  (orphans / duplicates / contradictions / stale), and the reference-tracking closed loop.
- **Cross-project Git knowledge repo** — pull/push with graceful no-remote degradation,
  L3 → L1/L2 promotion, hybrid contribution classification, conflict staging, append-only log,
  and zero-DB stats — with L3-only, no-overwrite, and path-traversal safety guards.
- **Cold-start import** (`/flow-import`) — resumable 3-step pipeline.
- **Notifications** — pluggable webhook (Feishu first), degrading to a local file when unset.
- **Self-evolution** (`/evolve`) — improvement proposals with a *structural* never-auto-apply
  guarantee (the engine has no code path that writes to `agents/` or `rules/`).
- **`cairn` CLI** — config/state/gate/intent/kb/lifecycle/lint/kbrepo/knowledge/notify/import/
  evolve, with a stable exit-code contract (`0` ok · `2` usage · `3` gate refusal · `4` corrupt state).
- **Distribution** — Claude Code plugin (commands, agents, skill, marketplace) and
  `cargo install` of the single binary.

### Notes

- The engine repository contains **no knowledge** — entries live in a separate private
  knowledge repo and inside host projects (public engine / private moat).
- Originally prototyped in Python, then rewritten in Rust to ship a single dependency-free
  binary (no Python / Node / interpreter required).

[0.1.0]: https://github.com/shenchangmin/cairnkit/releases/tag/v0.1.0
