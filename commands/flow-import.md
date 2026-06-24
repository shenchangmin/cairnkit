---
name: flow-import
description: Cold-start import — extract baseline knowledge from an existing project (resumable).
---

# /flow-import

Make an existing project's implicit knowledge explicit, as baseline draft entries.

## Steps
1. Start or resume: `python3 -m cairnkit --root . import show` (exit 0 → resume) else
   `python3 -m cairnkit --root . import init`.
2. Run the pipeline, one agent per step, advancing after each with
   `python3 -m cairnkit --root . import advance`:
   - **doc-collect** → dispatch `doc-collector` (multi-source: git history, docs, code scan).
   - **codebase-profile** → dispatch `codebase-profiler` (~60 search budget).
   - **knowledge-build** → dispatch `knowledge-builder` (≤13 draft entries + summary).
3. The builder writes draft entries under the knowledge root + a `knowledge-candidates.json`;
   then `python3 -m cairnkit --root . kb build-index`.
4. These drafts are consumed by the next `/flow-run` INIT.

Progress lives in `docs/knowledge-import/import-state.json` — a crash resumes from the last step.
