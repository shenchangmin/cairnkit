# Harness adapters

cairnkit is organized as **one shared engine + shared content, projected into each AI-coding
harness via a thin adapter** (the pattern everything-claude-code uses). No harness is the
privileged "source" form — Claude Code and Codex are both adapters over the same content.

```
cairnkit/
├── src/                      # the `cairn` engine — Rust, harness-agnostic, the only writer of state
│
├── agents/                   # SHARED role mandates (product, tech, architect, dev, verify, …)
├── commands/                 # SHARED command logic (flow-run, team-init, flow-status, …)
├── rules/                    # SHARED engineering mechanics
│   #   ↑ written once; both harnesses use these same files
│
├── .claude-plugin/           # Claude Code adapter — packaging (plugin.json, marketplace.json)
│   #   CC reads agents/ as Task sub-agents, commands/ as slash commands, skills/ as the orchestrator
│
├── AGENTS.md                 # Codex adapter — entry instructions (the delivery loop for a single agent)
├── .codex/                   # Codex adapter — baseline config.toml + Codex-specific supplement
└── scripts/sync-to-codex.sh  # Codex adapter — projects AGENTS.md + agents/ + commands/ into ~/.codex/
```

## The two adapters

| | Claude Code | Codex |
|---|---|---|
| Entry | slash commands (`/cairnkit:flow-run`) + `workflow-orchestrator` skill | `AGENTS.md` + `cairnkit-*` prompts |
| Role dispatch | each role is an isolated **Task sub-agent** | one agent **plays each role sequentially** (reads `agents/<role>.md`) |
| Install | `/plugin marketplace add . && /plugin install cairnkit@cairnkit` | `./scripts/sync-to-codex.sh` → `~/.codex/` |
| Config home | `~/.claude/` | `~/.codex/` |
| Engine | `cairn` (identical) | `cairn` (identical) |

## The load-bearing idea

The **`cairn` engine** (state machine, gates, knowledge layer) is harness-agnostic — it is just a
CLI. The **role/orchestrator content** lives once in `agents/` + `commands/`. An adapter only has
to map "how this harness exposes commands and dispatches roles" — everything else is shared. Adding
a third harness (Cursor, OpenCode, …) is a new adapter, not a fork.

> Trade-off, honestly: Codex has no native multi-sub-agent dispatch, so its role isolation is
> weaker than Claude Code's (sequential personas in one context vs. isolated sub-agents). Role
> separation is preserved at the artifact level (one stage artifact each). See `AGENTS.md`.

## Adding / updating the Codex form

```bash
cargo install --path .            # ensure `cairn` is on PATH
./scripts/sync-to-codex.sh --dry-run   # preview what lands in ~/.codex/
./scripts/sync-to-codex.sh             # install/update the cairnkit block in ~/.codex/
```
The sync is idempotent and merge-safe (it manages only a marked block in `~/.codex/AGENTS.md`).
