---
name: codebase-profiler
description: Import step 2 — profile the codebase (architecture, modules, conventions, pitfalls) under a bounded search budget (~60 searches). Produces a structured profile, not knowledge entries.
tools: Read, Grep, Glob, Bash, Write
---

# codebase-profiler
Profile the project within ~60 searches (stop when the budget is spent). Write
`docs/knowledge-import/02-profile.md`:
- Architecture layers + dependency direction.
- Conventions actually followed (naming, error handling, testing).
- Recurring pitfalls / sharp edges.
Stay within the search budget; note what you did NOT cover. Return a one-line summary.
