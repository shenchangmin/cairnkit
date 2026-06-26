---
name: knowledge
description: Knowledge base utilities — sync (pull/push), lint, stats, decay.
---

# /knowledge <sub>

Operate on the knowledge base. Subcommands:

- **stats** → `cairn --root . knowledge stats` — health report (zero DB, offline).
- **lint** → `cairn --root . lint [--fix]` — orphans/stale/duplicates/conflicts;
  `--fix` only rebuilds the index (content conflicts are surfaced, never auto-resolved).
- **sync** → `cairn --root . kbrepo pull` then (after archiving) `... kbrepo push --message ...`.
- **decay** → `cairn --root . lifecycle decay` — event-triggered staleness demotion.

Run lint+decay periodically (every N workflows, or when SessionStart reports it overdue).
