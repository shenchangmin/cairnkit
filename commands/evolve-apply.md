---
name: evolve:apply
description: Review a pending /evolve proposal and, only after human approval, apply it to the harness.
---

# /evolve:apply

Apply a pending self-improvement — **with a human gate**. The cairnkit CLI never edits the
harness itself; you make the edits only after the user explicitly approves.

## Steps
1. List pending: `cairn --root . evolve list --state pending`.
2. Show the proposal (`docs/workflows/evolve-log/pending/<id>.md`) to the user. **Wait for explicit approval.**
3. On approval: make the proposed edits to `agents/*.md` / `rules/*.md` yourself, then record the decision:
   `cairn --root . evolve apply --id <id>` (moves it to applied/).
   Commit the change to Git (versioned, reversible).
4. On rejection: `evolve reject --id <id>`. To postpone: `evolve defer --id <id>`.

Never apply without step 2's explicit human approval. Proposal id: $ARGUMENTS
