# CLAUDE.md

Guidance for AI coding agents (and humans) working in the **cairnkit** repository.
This file is the project's operating contract. Read it before writing code.

> cairnkit is a knowledge-precipitation harness for Claude Code.
> **The workflow is the pipe; knowledge is the moat.** See [README.md](README.md) for the public-facing narrative.

---

## 1. First principle (never violate)

The **purpose** of this project is the knowledge moat (modules **M4–M7**).
The delivery workflow (M2–M3) is the **vehicle** — it exists to *force* every
delivery to capture, inject, and extract knowledge.

When workflow convenience conflicts with knowledge-precipitation quality,
**knowledge wins.** A workflow with no knowledge loop is a single-use script;
that is the anti-pattern we exist to avoid.

Do **not** build "yet another BMAD." The differentiation is the knowledge layer.
If a change does not serve the knowledge loop, question whether it belongs in v1.

## 2. The four-layer architecture

Work is split by certainty: **fuzzy work → Markdown (model-driven); mechanical,
verifiable work → Python (deterministic, testable).**

```
┌─ Markdown layer (skills / agents / commands) ─ model-driven "fuzzy" work ─┐
│  analysis / design / implementation prompts · role personas               │
└────────────────────────────────────────────────────────────────────────-─┘
        │ calls via Bash on each stage transition ↓     ↑ returns structured result
┌─ Python layer (the `cairnkit` package / CLI) ─ deterministic "mechanical" work ┐
│  state-machine transitions · stage admission gates · index building            │
│  query-budget enforcement · frontmatter schema validation · maturity/decay     │
│  Lint · promotion judging · Git pull/push/conflict staging · notification       │
└────────────────────────────────────────────────────────────────────────────-──┘
        │ all state / knowledge lands in ↓
┌─ File layer ─ the single source of truth (Markdown + YAML, Git-managed) ───┐
│  .cairnkit/STATE.yaml · docs/workflows/** · knowledge *.md · index *.md     │
└────────────────────────────────────────────────────────────────────────-───┘
        │ light triggers ↓
┌─ Hooks layer ─ thin triggers only ─────────────────────────────────────────┐
│  stage event → notification · SessionStart "is Lint overdue?" · ref capture │
└─────────────────────────────────────────────────────────────────────────-──┘
```

### Hard invariants

- **The file system is the state machine.** All state, artifacts, and knowledge
  are human-readable Markdown + YAML. No database, no resident service, no central server.
- **Only Python mutates state.** The model never holds workflow state in its head.
  Re-running `/flow-run` = Python reads `STATE.yaml` and resumes. Crash-resume is automatic.
- **Stage gates are hard-enforced in Python (`gate.py`), not model goodwill.**
  Missing/invalid upstream artifacts → transition refused.
- **Context firewall.** Role agents interact only through files / Python — never directly
  with each other. A sub-agent failure must not pollute the main context.
- **`/evolve` never auto-applies.** Self-modification always passes through
  pending → human approval → applied, Git-versioned.

## 3. Repository structure

```
cairnkit/                         ← this repo · the engine · MIT · public
├── .claude-plugin/
│   ├── marketplace.json
│   └── plugin.json               ← userConfig (knowledge-repo URL / webhook) + hooks
├── commands/                     ← M11: /team-init /flow-import /flow-run /flow-status
│                                       /knowledge /evolve /evolve:apply
├── agents/                       ← M3: product/tech/architect-be/-fe/dev/verify/visual/archiver
│                                       + the 3 import agents
├── skills/
│   └── workflow-orchestrator/SKILL.md   ← M2 orchestrator shell
│   └── knowledge-*/SKILL.md             ← knowledge extract / query / promote skills
├── rules/                        ← M10: context firewall / degradation / budgets / red lines
├── hooks/
│   ├── hooks.json
│   └── scripts/                  ← notification, ref capture, lint reminder (thin shell → python)
├── cairnkit/                     ← Python package (M4–M7, M10) — deterministic, testable
│   ├── cli.py                    ← `python -m cairnkit <cmd>`
│   ├── state.py                  ← state transitions / checkpoints
│   ├── gate.py                   ← stage admission gate
│   ├── knowledge/
│   │   ├── model.py  schema.py  index.py  query.py
│   │   ├── lifecycle.py          ← maturity / decay / promotion judging
│   │   ├── lint.py
│   │   └── extract_gate.py       ← strict extraction gate
│   ├── kbrepo.py                 ← independent Git knowledge repo: pull/push/stage/conflict
│   └── notify.py                 ← webhook notification (pluggable channels)
├── tests/                        ← pytest (Python layer, 80%+ coverage)
└── README.md  LICENSE  NOTICE.md  CONTRIBUTING.md
```

Artifacts produced **inside a host project** once cairnkit is installed (never in this repo):

```
<host-project>/
├── cairnkit.yaml                 ← repos[], knowledge-repo URL, domain, notify config
├── .cairnkit/STATE.yaml          ← workflow state (the file IS the state machine)
├── docs/workflows/<run-id>/      ← per-stage artifacts + evolve-log
└── docs/knowledge/               ← L3 project knowledge (promotable to the shared repo)
```

## 4. Tech stack

| Concern | Choice |
|---|---|
| Harness form | Claude Code plugin: Markdown skills/agents/commands + `plugin.json` + `hooks.json` |
| Glue scripts | **Python 3.10+**, packaged as `cairnkit`, invoked via `python -m cairnkit ...` |
| Python deps | **Minimal**: `ruamel.yaml` (order-preserving frontmatter) + stdlib; Git via `subprocess` (no gitpython) |
| Knowledge store | Markdown + YAML frontmatter + Git |
| Retrieval (v1) | Structured filtering (tags / applicable_phases / category / domain + 3-level index). Semantic/vector retrieval is v2. |
| Notification | Python + webhook, hook-triggered, pluggable channel |
| Testing | pytest + coverage (Python layer 80%+) |
| **Not in v1** | ❌ database ❌ vector embeddings ❌ central service / MCP ❌ resident process |

## 5. The `cairnkit` CLI (the deterministic surface)

Every subcommand: ① read-only queries print JSON to stdout; ② mutations change files
and return a result code; ③ all of it is pytest-able **without** Claude Code.

```
python -m cairnkit state show|advance|set-stage|resume
python -m cairnkit gate check --stage <S>
python -m cairnkit intent classify --input <file>
python -m cairnkit kb build-index
python -m cairnkit kb query --stage <S> --budget <N> [--domain D]
python -m cairnkit kb extract --from <run-dir>
python -m cairnkit kb touch --from <run-dir>
python -m cairnkit kb validate <file>
python -m cairnkit lifecycle promote|decay
python -m cairnkit lint [--fix]
python -m cairnkit kbrepo pull|push|promote|stage-conflict
python -m cairnkit notify --event <E> [--channel feishu]
```

## 6. Coding standards

- **Python is the testable core.** If logic is mechanical (state transitions, gates,
  index, budget, schema, decay, Lint, promotion), it lives in Python and has tests.
  If it is creative/fuzzy (analysis, design, extraction prompts), it lives in Markdown.
- **Many small files > few large files.** 200–400 lines typical, **800 max**.
  High cohesion, low coupling; organize by feature/domain.
- **Immutability.** Return new objects; do not mutate in place.
- **Explicit error handling.** Never silently swallow; budget truncation must **log what was dropped** (no silent truncation).
- **Validate at boundaries.** frontmatter and config are schema-validated; missing fields are rejected.
- **No hardcoded secrets.** Webhooks/tokens go through env vars named in config; never commit `.env`.
- **No path hardcoding.** Use `${CLAUDE_PLUGIN_ROOT}`; support single-repo and multi-repo via the unified `repos[]` model.

## 7. Build order (risk-driven batches)

Implementation is **staged**; design is already 100% complete (`_dev/01`–`05`, private).
Order: validate the top risk first, then stack the moat, defer the highest-risk piece.

| Batch | Modules | Purpose |
|---|---|---|
| **B1** | M1 skeleton + M2 minimal state machine (3–4 stages wired through) | **Validate R1** — is the orchestration model feasible? ← start here |
| B2 | M4 knowledge model/store + M5 index/query | Lay the moat foundation |
| B3 | M3 eight role agents wired to knowledge query + M2 completed to 16 stages | Workflow serves the knowledge loop |
| B4 | M6 lifecycle (maturity / decay / Lint / ref loop) | Keep knowledge fresh |
| B5 | M7 cross-project Git knowledge repo (pull/push/promote/stage/conflict) | **Biggest blank space — the moat itself** |
| B6 | M8 cold-start import + M10 notification / cross-cutting | Feeding + reachability |
| B7 | M9 `/evolve` self-evolution | Highest risk, no reference, deferred |

Each batch ends with: full test suite green + one code review (SOP steps 7–8).
The detailed per-module dependency graph, TDD steps, test-case tables, and acceptance
checklists live in the private `_dev/06-development-sop.md`.

## 8. Development workflow — the standard loop

Every batch (and every non-trivial unit) follows one full loop. **Do not jump straight to
code.** This is the 10-step SOP compressed to batch scale:

```
P0 Intent decomposition  →  P1 Spec/design doc  →  P2 Plan  →  (human CONFIRM)
   →  P3 TDD implementation  →  P4 independent Code Review  →  P5 independent Functional Test  →  P6 Deliver & precipitate
```

- **P0–P2 are mandatory and gated.** No code is written before the plan is confirmed.
- **Implementer ≠ reviewer ≠ tester (hard rule).** P4 Code Review is run by a *separate*
  agent (`python-reviewer`/`code-reviewer`), and P5 Functional Test by *another* agent
  (`qa-tester`/`e2e-runner`) — never the agent that wrote the code. Independent perspective
  is the verification; self-review does not count. This mirrors the project's own context-firewall principle.
- **P3 TDD.** Tests first (RED) → minimal implementation (GREEN) → refactor (IMPROVE);
  Python layer 80%+ coverage; Python gate before Markdown shell.
- **P4** fixes all HIGH issues before proceeding.
- **P6** commits with conventional-commit format (`feat:`/`fix:`/`refactor:`/`docs:`/`test:`/`chore:`)
  and **dogfoods**: precipitates the batch's pitfalls/decisions into draft knowledge — we are
  a knowledge-precipitation tool, so our own build must feed the moat.

The full loop, per-batch specifics, test-case tables, and acceptance checklists live in the
private `_dev/06-development-sop.md`.

## 9. Open-source discipline (public engine / private moat)

This repo is the **engine** — generic, MIT, give-it-away. The competitive asset is the
**private knowledge repo** it feeds, which **never** lives here.

- **This repo must never contain** real knowledge entries, real business-domain content,
  or real project names. `docs/` and examples use neutral placeholders (`domain: ecommerce`, `<host-project>`).
- Public docs frame cairnkit as an independent **clean-room implementation inspired by
  publicly published knowledge-precipitation ideas** — never as a clone of any internal product.
- `_dev/` (private design docs) and `_research/` (reference repos) are git-ignored and never published.
- Before any release: run the open-source sanitizer (secrets / PII / internal-ref scan);
  ensure `pytest` is green and coverage meets target. See `_dev/opensource-design.md §8`.
