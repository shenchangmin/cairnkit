# Using & verifying cairnkit

There are **two tiers** you can exercise:

- **Tier A — the deterministic engine (no Claude Code needed).** Everything in the `cairnkit`
  Python package — the 16-stage state machine, knowledge model/index/query, lifecycle, the
  cross-project Git knowledge repo, notifications, import, `/evolve` — runs from the CLI and is
  fully testable on its own. This is most of the value and you can verify it right now.
- **Tier B — the full plugin inside Claude Code.** Installs the commands/agents/skill so the
  orchestrator dispatches role sub-agents end-to-end (`/flow-run`, `/flow-import`, `/evolve`).
  This validates the one thing the CLI can't: the model driving the loop.

Below, `PY` is the project's venv interpreter:

```bash
cd /Users/mac/work/cairnkit
python3 -m venv .venv && .venv/bin/pip install -e ".[dev]"   # one-time
PY=$(pwd)/.venv/bin/python
```

---

## 0. Fastest confidence: run the test suite

```bash
$PY -m pytest -q                                   # 165 tests
$PY -m coverage run -m pytest && $PY -m coverage report   # ~94% on the Python core
```

Green = the whole deterministic engine behaves to spec (state machine, gates, knowledge,
lifecycle, Git repo, evolve safety invariant, etc.).

---

## 1. See a full workflow run (state machine)

The orchestrator normally drives this in Claude Code; here you drive it by hand to see the
mechanics. A one-shot demo script walks INIT → DONE:

```bash
CAIRN_PY=$PY ./scripts/demo-run.sh lite     # try also: full, single
```

You'll see the run move through the stages, auto-pause at each `CLARIFY_*` (and the script
approves it), and the `lite` path skip the frontend stages. Do it manually to feel each lever:

```bash
mkdir /tmp/ck-demo && cd /tmp/ck-demo
cat > cairnkit.yaml <<'YAML'
project: demo
domain: ads
repos:
  - name: demo
    path: .
YAML

$PY -m cairnkit --root . config show                         # is the project initialised?
$PY -m cairnkit --root . state init --run-id 2026-06-25-x    # start a run
$PY -m cairnkit --root . state show                          # stage=INIT
$PY -m cairnkit --root . intent classify --text "fix a typo" # -> suggests "single"
$PY -m cairnkit --root . state advance                       # INIT -> INTENT_GATE
$PY -m cairnkit --root . state set-path-mode lite            # choose the route
$PY -m cairnkit --root . state advance                       # -> ANALYSE_PRODUCT
# a producing stage won't advance until its artifact exists (the hard gate):
$PY -m cairnkit --root . state advance ; echo "exit=$?"      # exit=3 (gate refused)
mkdir -p docs/workflows/2026-06-25-x
echo "# product analysis" > docs/workflows/2026-06-25-x/01-product.md
$PY -m cairnkit --root . state advance                       # -> CLARIFY_PRODUCT (paused)
$PY -m cairnkit --root . state resume                        # {"stage":"CLARIFY_PRODUCT","paused":true}
$PY -m cairnkit --root . state advance ; echo "exit=$?"      # exit=3 (needs approval)
$PY -m cairnkit --root . state approve-clarify
$PY -m cairnkit --root . state advance                       # proceeds
```

What you're verifying: **files are the only state** (kill your shell, `state show` resumes
exactly where you were), **gates are hard** (missing artifact → exit 3, no skipping),
**CLARIFY is an async pause**, and **IntentGate routes** small work onto a shorter path.

**Retry / block** (verify stages):

```bash
$PY -m cairnkit --root . state fail --stage BUILD_VERIFY      # bump retry counter
# after 5 failures the run is blocked:
for i in 1 2 3 4 5; do $PY -m cairnkit --root . state fail --stage BUILD_VERIFY >/dev/null; done
$PY -m cairnkit --root . state show | grep blocked            # blocked_reason set
$PY -m cairnkit --root . state unblock                        # human clears it, retries reset
```

---

## 2. The knowledge moat (model · index · query)

```bash
cd /tmp/ck-demo
mkdir -p docs/knowledge/tech-wiki
cat > docs/knowledge/tech-wiki/TK-001.md <<'MD'
---
id: TK-001
title: Keyset pagination beats OFFSET
category: tech
domain: null
type: decision
guideline_polarity: null
maturity: draft
knowledge_class: causal
layer: L1
tags: [pagination, mysql, performance]
applicable_phases: [ARCHITECT_BACKEND]
evidence:
  contributors: [you]
  sources: []
  projects: []
  last_referenced: null
  ref_count: 0
history: []
---
Under deep OFFSET, MySQL scans and discards N rows; keyset (WHERE id > :last) stays O(limit).
MD

$PY -m cairnkit --root . kb validate docs/knowledge/tech-wiki/TK-001.md   # schema check
$PY -m cairnkit --root . kb build-index                                   # generate the 3-level index
$PY -m cairnkit --root . kb query --stage ARCHITECT_BACKEND --budget 300  # budget-bounded injection
```

The query output includes `injected_ids`, `dropped` (what didn't fit), and `over_budget`
(true when the single top-ranked entry alone exceeds the budget) — **truncation is never
silent**. Try `--budget 3` to see an entry get reported as over-budget rather than dropped,
and add a second entry with a different `applicable_phases` to see stage filtering.

---

## 3. Knowledge lifecycle (maturity · references · lint)

```bash
# reference an entry from a run artifact, then promotion follows usage:
mkdir -p docs/workflows/r1
echo '{"knowledgeReferences": [{"id": "TK-001", "title": "...", "usedIn": "design step 2"}]}' \
  > docs/workflows/r1/05-implement.md
$PY -m cairnkit --root . kb touch --from docs/workflows/r1     # writeback: ref_count++/last_referenced
$PY -m cairnkit --root . lifecycle promote                    # draft -> verified (1 reference)
$PY -m cairnkit --root . lifecycle decay                      # demote entries past their half-life
$PY -m cairnkit --root . lint                                 # orphans/duplicates/conflicts/stale
$PY -m cairnkit --root . lint --fix                           # mechanical fixes only (rebuild index)
```

Lint never auto-resolves a content contradiction — it surfaces it for a maintainer.

---

## 4. Cross-project Git knowledge repo (the moat itself)

```bash
# a shared knowledge repo is an independent git repo:
mkdir /tmp/team-kb && cd /tmp/team-kb && git init -q && git config user.email t@t.t && git config user.name t
$PY - <<PYEOF
from pathlib import Path
from cairnkit.knowledge import kbrepo
kbrepo.init_repo(Path("."))
PYEOF
git add -A && git commit -qm init

# point a project at it, then push/stats/promote via the CLI:
cd /tmp/ck-demo
cat >> cairnkit.yaml <<'YAML'
knowledge_repo:
  local: /tmp/team-kb
YAML
cp docs/knowledge/tech-wiki/TK-001.md /tmp/team-kb/tech-wiki/   # (drop an entry in)
$PY -m cairnkit --root . kbrepo push --message "add TK-001"      # commit (degrades gracefully w/o remote)
$PY -m cairnkit --root . knowledge stats                         # health report, zero DB
$PY -m cairnkit --root . kbrepo promote --id TK-001 --to L1      # L3 -> L1 (only L3 promotable; never overwrites)
```

---

## 5. /evolve (self-improvement, never auto-applied)

```bash
cd /tmp/ck-demo
$PY -m cairnkit --root . evolve propose --id slow-build --content "root cause + suggested rule change"
$PY -m cairnkit --root . evolve list --state pending
$PY -m cairnkit --root . evolve apply  --id slow-build      # records the decision (you edit the harness yourself)
$PY -m cairnkit --root . evolve list --state applied
```

The CLI **cannot** edit `agents/` or `rules/` — applying a change is a human edit you make
before recording the decision. That guarantee is structural (see `cairnkit/evolve.py`).

---

## 6. Cold-start import (`import`)

```bash
cd /tmp/ck-demo
$PY -m cairnkit --root . import init       # start the resumable 3-step pipeline
$PY -m cairnkit --root . import advance    # doc-collect -> codebase-profile -> knowledge-build -> done
$PY -m cairnkit --root . import show       # progress survives a crash (docs/knowledge-import/import-state.json)
```

---

## 7. Tier B — run it as a Claude Code plugin

This is the real end-to-end where the model drives the loop and dispatches role agents.

1. Add this repo as a plugin marketplace and install it (the plugin manifest is in
   `.claude-plugin/`). In Claude Code: add the local marketplace, then install `cairnkit`.
2. Make sure the `cairnkit` package is importable where you run (`pip install -e .` in the repo,
   or otherwise on `PYTHONPATH`).
3. In a test project: `/team-init`, then `/flow-run add a small feature`.
4. Watch the orchestrator (skill `workflow-orchestrator`) read STATE, dispatch the per-stage
   agent (`product`, `tech`, `architect-be`, …), write each artifact, and advance — pausing at
   CLARIFY for your approval. Other commands: `/flow-status`, `/flow-import`, `/knowledge`,
   `/evolve`, `/evolve:apply`.

This is the only part not covered by the test suite (the model-driven sub-agent dispatch);
running one `/flow-run` confirms it.

---

## Command reference

```
config show
state init --run-id <id> | show | resume | advance | set-stage <S> | set-path-mode <full|lite|single>
state approve-clarify | fail --stage <BUILD_VERIFY|E2E_VERIFY> | unblock
gate check --stage <S>
intent classify --text "<req>" | --input <file>
kb build-index | query --stage <S> --budget <N> [--domain <d>] | validate <file>
kb extract --from <run-dir> | touch --from <run-dir>
lifecycle promote | decay
lint [--fix]
kbrepo pull | push --message <m> | promote --id <id> --to <L1|L2> | stage-conflict --id <id> --file <f>
knowledge stats
notify --event <e> [--detail <d>] [--channel feishu]
import init | show | advance
evolve propose --id <id> (--file <f> | --content <c>) | list --state <s> | apply|reject|defer --id <id>
```

Return codes: `0` ok · `2` usage/precondition · `3` admission-gate refused · `4` STATE corrupt.
