---
name: flow-status
description: Show the current cairnkit run — stage, history, pending approvals, artifacts.
---

# /flow-status

Report where the current run stands. This is read-only.

## Steps

1. Run `python3 -m cairnkit --root . config show`.
   - Exit `2` → no `cairnkit.yaml`; suggest `/team-init` and stop.
   - Exit `0` → if `has_run` is false, report "no run yet — start one with `/flow-run`" and stop.
2. Run `python3 -m cairnkit --root . state show`.
   - Exit `4` → STATE corrupt; show the repair guidance from stderr verbatim and stop.
   - Exit `0` → parse the JSON, then run `python3 -m cairnkit --root . state resume` for the `paused` flag.
3. Present a concise human summary:
   - **Stage** and whether the run is **paused** (awaiting a CLARIFY approval — name what).
   - **History** (stages completed so far).
   - **Artifacts** produced (paths under `docs/workflows/<run-id>/`).
   - Next action: if paused → approve with `/flow-run` after review; else → continue `/flow-run`.
