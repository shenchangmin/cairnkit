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

## How role dispatch works here (the key difference from Claude Code)

Claude Code dispatches each stage's role as an isolated Task sub-agent. Codex has no native
multi-sub-agent dispatch, so **you (a single agent) play each role sequentially**: at each
stage you adopt that role's mandate, do its work, write its artifact, then advance. The role
mandates are the shared files in `agents/` — read the relevant one and follow it as your
persona for that stage. Keep roles separated *at the artifact level* even though one agent runs.

| stage | role file to adopt | writes |
|---|---|---|
| ANALYSE_PRODUCT | `agents/product.md` | docs/workflows/<run-id>/01-product.md |
| ANALYSE_TECH | `agents/tech.md` | 02-tech.md |
| ARCHITECT_BACKEND | `agents/architect-be.md` | 03-arch.md |
| ARCHITECT_FRONTEND | `agents/architect-fe.md` | 04-arch-fe.md |
| IMPLEMENT | `agents/dev.md` | 05-implement.md |
| BUILD_VERIFY / E2E_VERIFY / TEST | `agents/verify.md` | 06-build.md / 08-e2e.md / 09-test.md |
| VISUAL_REVIEW | `agents/visual.md` | 07-visual.md |
| ARCHIVE | `agents/archiver.md` | 10-archive.md |
| INIT, INTENT_GATE, CLARIFY_*, DONE | — | — |

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
5. **A role stage** → adopt the mapped role file, do its work, write its artifact, then `state advance`.
   Each role first pulls knowledge: `cairn --root . kb query --stage <STAGE> --budget 300` and
   records a `knowledgeReferences` block in its artifact.
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
