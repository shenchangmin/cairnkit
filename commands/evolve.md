---
name: evolve
description: Analyze a bug/issue and record a harness-improvement proposal (analysis only — never applied).
---

# /evolve

Diagnose a problem and propose a harness improvement. **This never changes the harness** —
it only writes a proposal for later human review.

## Steps
1. Replay the relevant workflow log/artifacts under `docs/workflows/<run-id>/` and do root-cause analysis.
2. Simulate the improved behavior in context (what would the agent/rule have needed?).
3. Write the analysis + concrete suggested change, then record it:
   `cairn --root . evolve propose --id <slug> --file <analysis.md>`
4. `cairn --root . notify --event arch_review --detail "evolve proposal <slug>"`.

The proposal lands in `docs/workflows/evolve-log/pending/`. Apply it only via `/evolve:apply`.

The issue to analyze: $ARGUMENTS
