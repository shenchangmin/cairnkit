# cairnkit — Agent instructions (Codex / AGENTS.md harness)

cairnkit is a knowledge-precipitation delivery harness. **The workflow is the pipe; knowledge
is the moat.** This file is the entry point for harnesses that read `AGENTS.md` (OpenAI Codex
CLI and compatibles). The Claude Code form of the same harness lives in `.claude-plugin/` +
`commands/` + `skills/`; both forms drive the **same `cairn` engine** and the **same role
content in `agents/`** — see [docs/ADAPTERS.md](docs/ADAPTERS.md).

## The engine: `cairn`

The deterministic core is a single binary, `cairn` (Rust, zero runtime deps). It owns the
state machine, the admission gates, and the knowledge layer. You never hold workflow state in
your head — **`cairn` is the only writer of state** (`.cairnkit/STATE.yaml`). All commands run
from the host project root:

```bash
cairn --root . config show                       # is the project initialised? (has_run flag)
cairn --root . state init --run-id <id>          # start a run
cairn --root . state show | resume               # current state / {stage, paused}
cairn --root . state advance                      # -> next stage (gate-checked)
cairn --root . state set-path-mode <full|lite|single>
cairn --root . state approve-clarify | unblock
cairn --root . state fail --stage <BUILD_VERIFY|E2E_VERIFY>
cairn --root . kb build-index | query --stage <S> --budget 300 | extract --from <dir> | touch --from <dir>
cairn --root . lifecycle promote | decay   ·   lint [--fix]   ·   knowledge stats
cairn --root . kbrepo pull | push --message <m> | promote --id <id> --to <L1|L2> | init
```

Exit codes: `0` ok · `2` usage · `3` gate refused · `4` STATE corrupt. **Never** edit
`.cairnkit/STATE.yaml` by hand.

## How role dispatch works here

Codex has **native multi-agent** (`multi_agent = true`; see developers.openai.com/codex/multi-agent).
cairnkit uses it: you are the **parent orchestrator**, and at each stage you **dispatch the stage's
role agent** — an isolated Codex sub-agent defined in `~/.codex/agents/<role>.toml` with its own
reasoning effort, sandbox, and `developer_instructions` (the role's mandate). This gives real
role isolation (a context firewall per role), the same discipline as the Claude Code form — not
one agent blurring every role. Each role's full mandate is also at `~/.codex/cairnkit/roles/<role>.md`.

| stage | role agent to dispatch | writes |
|---|---|---|
| ANALYSE_PRODUCT | `product` | docs/workflows/<run-id>/01-product.md |
| ANALYSE_TECH | `tech` | 02-tech.md |
| ARCHITECT_BACKEND | `architect-be` | 03-arch.md |
| ARCHITECT_FRONTEND | `architect-fe` | 04-arch-fe.md |
| IMPLEMENT | `dev` | 05-implement.md |
| BUILD_VERIFY / E2E_VERIFY / TEST | `verify` | 06-build.md / 08-e2e.md / 09-test.md |
| VISUAL_REVIEW | `visual` | 07-visual.md |
| ARCHIVE | `archiver` | 10-archive.md |
| INIT, INTENT_GATE, CLARIFY_*, DONE | — (parent handles directly) | — |

(The cold-start import roles are `agents/doc-collector.md`, `codebase-profiler.md`, `knowledge-builder.md`.)

## The delivery loop (`/flow-run`)

1. **Verify init.** `cairn --root . config show`. Exit 2 → no `cairnkit.yaml`; tell the user to
   run team-init first (see `commands/team-init.md`). Else read `has_run`.
2. **Start or resume.** If `has_run` is false, derive `run-id` = `<YYYY-MM-DD>-<slug>` and
   `cairn --root . state init --run-id <run-id>`. Else resume the existing run.
3. **INIT** → `cairn --root . kb build-index`, then `state advance`.
4. **INTENT_GATE** → classify the request yourself and `state set-path-mode <mode>`:
   - `single` — single-point / config / governance / docs (no analysis or design).
   - `lite` — backend feature, no UI.
   - `full` — feature with frontend/UI. *When unsure, route higher (full is safer). Language-independent.*
   Then `state advance`. (`cairn intent classify` is only an optional fallback hint.)
5. **A role stage** → **dispatch the mapped role sub-agent** (`~/.codex/agents/<role>.toml`) to do
   its work and write its artifact; then `state advance`. Each role first pulls knowledge:
   `cairn --root . kb query --stage <STAGE> --budget 300` and records a `knowledgeReferences` block.
   - Verify stages: on failure run `cairn --root . state fail --stage <stage>` and redo the fix;
     after the cap the run blocks → tell the user.
6. **CLARIFY stages** pause the run (`state advance` into one sets `pending_clarify`). **Stop and
   present the artifact for the user's approval; never force a stage.** On approval,
   `state approve-clarify`, then continue.
7. Repeat until `stage` is `DONE`. **ARCHIVE** runs `agents/archiver.md` to precipitate knowledge.

Other entry points mirror the `commands/` files: `team-init`, `flow-status`, `flow-import`,
`knowledge`, `evolve`, `evolve:apply`.

## Hard rules

- Files are the only source of truth; `cairn` is the only writer of state.
- One step at a time — the gate enforces it. Exit 3 from `advance` = the current stage's
  artifact is missing/empty or a CLARIFY is unapproved; fix the cause, never force the stage.
- Exit 4 (corrupt STATE) → surface the repair guidance; do not improvise a new STATE file.
- `/evolve` never auto-applies: improvements are recorded as proposals for human approval.
