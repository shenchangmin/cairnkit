# cairnkit

> A knowledge-precipitation harness for Claude Code.
> **The workflow is the pipe; knowledge is the moat.**

A *cairn* is a pile of stones that travelers add to, one by one, to mark the trail
for those who come after. **cairnkit** does the same for software teams using AI
coding agents: every delivery adds a stone — a piece of durable, verified knowledge —
to a shared store, so each new task starts by *standing on the shoulders* of every
task before it.

Models iterate, tool-chains change, workflows get rewritten. But the architectural
decisions, the pitfalls, the domain models your team accumulates are **permanent** —
they don't expire when the model changes. cairnkit treats that accumulated knowledge
as the real, compounding asset, and makes a disciplined delivery workflow the vehicle
that *forces* knowledge to be captured.

## Why this exists

Most AI-coding workflows are stateless across sessions and projects: every task starts
from zero, last week's pitfall gets stepped on again, last month's architecture decision
gets re-derived. The workflow is commodity; **the knowledge is the moat** — and almost no
tool treats knowledge precipitation as a first-class, lifecycle-managed, cross-project asset.

cairnkit does. The 16-stage delivery workflow exists primarily to **inject**, **consume**,
and **extract** knowledge — not the other way around.

## How it works (one screen)

```
INIT (pull knowledge)  →  multi-stage delivery  →  ARCHIVE (extract knowledge, push)
       ↑ stand on past work        ↑ query per stage           ↓ next run benefits
       └──────────────── shared Git knowledge repo ────────────────┘
```

- **Delivery harness** — a Claude Code plugin: a 16-stage orchestrator + role subagents
  (Product / Tech / Architect / Dev / Verify / Visual / Archiver). State lives entirely
  in files — *the file system is the state machine* — so any run is resumable from any
  device, with no database and no server.
- **Knowledge model** — every entry is classified on three orthogonal axes
  (**storage layer** `personal/team/tech/biz/project` × **type** `model/decision/guideline/pitfall/process`
  × **maturity** `draft→verified→proven`) plus a knowledge-class (`point/causal/spatiotemporal`).
  **Tech vs business** is the primary axis: tech knowledge is cross-project and globally
  shareable; business knowledge is domain-bounded.
- **Progressive index + query budget** — agents navigate a 3-level index
  (catalog → category list → full entry) under a token budget, so knowledge consumption
  never bloats context.
- **Lifecycle** — maturity promotion driven by real usage, automatic decay of stale
  knowledge, Lint for contradictions/orphans/duplicates, and a reference-tracking closed loop.
- **Cross-project moat** — knowledge precipitates into an **independent Git repo**, shared
  across all your projects. No DB, no central service — Git *is* the sync protocol.

## Public engine, private moat

cairnkit (this repo) is the **engine** — generic, MIT-licensed, give-it-away commodity.
Your team's actual competitive asset is the **private knowledge repo** it feeds, which
**never** lives here:

```
cairnkit/              ← this repo · the engine · open source
team-knowledge.git     ← your team's knowledge · PRIVATE, never published
<your project>/.cairnkit/   ← per-project state & local knowledge · private
```

Open-sourcing the engine costs you nothing strategically — by design, the moat is the
knowledge, not the harness.

## Install

cairnkit is a single `cairn` binary (Rust, zero runtime deps) plus a Claude Code plugin.

**1. The `cairn` binary** (needs the [Rust toolchain](https://rustup.rs)):

```bash
git clone https://github.com/shenchangmin/cairnkit && cd cairnkit
cargo install --path .          # builds `cairn` onto your PATH
```

**2a. Claude Code** — in a Claude Code session:

```
/plugin marketplace add /path/to/cairnkit
/plugin install cairnkit@cairnkit
```

**2b. Codex** — project the same harness into `~/.codex/`:

```bash
./scripts/sync-to-codex.sh        # installs AGENTS.md + cairnkit-* prompts (merge-safe)
```

cairnkit ships **both** a Claude Code and a Codex form over one shared engine + content — see
[docs/ADAPTERS.md](docs/ADAPTERS.md). Full setup (optional shared knowledge repo + notifications)
is in [SETUP.md](SETUP.md).

## Quick start

In any project:

```
/cairnkit:team-init                          # once per project — generates cairnkit.yaml
/cairnkit:flow-run <your feature request>    # runs the pipeline, pausing at each CLARIFY for you
```

The orchestrator dispatches a role agent per stage, writes each artifact, advances only when the
gate passes, and archives what it learned. You can also drive the engine directly via the CLI
without Claude Code — see [USAGE.md](USAGE.md).

## Status

🟢 **v1 released — open source.** All staged modules are complete:

- **Orchestration** — 16-stage file-as-state-machine with IntentGate routing
  (full/lite/single), CLARIFY async pauses, verify-stage retry/block, and 11 role agents.
- **Knowledge moat** — entry model + schema, 3-level progressive index + budget query,
  maturity lifecycle (promote/decay/lint + reference closed loop), and a cross-project Git
  knowledge repo (pull/push, L3→L1/L2 promotion, hybrid contribution, conflict staging).
- **Feeding & reach** — cold-start `/flow-import` pipeline and pluggable notifications.
- **Self-evolution** — `/evolve` with a *structural* never-auto-apply guarantee.

The deterministic core is a **single `cairn` binary** (Rust, zero runtime dependencies — no
Python/Node/interpreter), with unit + integration tests, running independently of Claude Code.

> Heads-up: end-to-end orchestration (the model dispatching role sub-agents inside Claude Code)
> is best validated by running a real `/flow-run`; the CLI engine itself is fully tested.

## License & credits

MIT — see [LICENSE](LICENSE). Built on prior open-source work; see [NOTICE.md](NOTICE.md).

cairnkit draws on well-established ideas about knowledge bases and long-lived memory for
AI agents — notably the *LLM Wiki* pattern (Ingest / Query / Lint) — and builds on the
open-source projects credited in [NOTICE.md](NOTICE.md).
