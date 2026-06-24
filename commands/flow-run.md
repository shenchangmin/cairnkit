---
name: flow-run
description: Start or resume a cairnkit delivery run (the file-as-state-machine workflow).
---

# /flow-run

Run or resume the delivery workflow for this project.

## Steps

1. **Verify the project is initialised.** Run `python3 -m cairnkit --root . config show`.
   - Exit `2` → no/invalid `cairnkit.yaml`. Tell the user to run `/team-init` first and stop.
   - Exit `0` → read the `has_run` flag from the JSON.

2. **If `has_run` is false** (no `.cairnkit/STATE.yaml` yet): create a run. Derive a `run-id`
   as `<YYYY-MM-DD>-<short-slug>` from today's date and the user's request, then:
   ```bash
   python3 -m cairnkit --root . state init --run-id <run-id>
   ```
   **If `has_run` is true**: you will **resume** the existing run (do not re-init). If
   `state show` then returns exit `4`, STATE is corrupt — surface the repair guidance and stop.

3. **Hand off to the orchestrator.** Invoke the `workflow-orchestrator` skill and follow its
   loop: read state → dispatch the stage's role agent → write the artifact → `state advance`,
   repeating until `DONE`. Honour CLARIFY pauses (stop for approval; never force a stage).

The user's feature request for this run is: $ARGUMENTS
