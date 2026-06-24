---
name: team-init
description: Initialise cairnkit in this project — generate cairnkit.yaml (single-repo default).
---

# /team-init

Set up cairnkit for the current project.

## Steps

1. **Check for an existing `cairnkit.yaml`.** If present, do not overwrite — report the
   current config and stop (re-init is an explicit, separate action).

2. **Gather minimal config:**
   - `project`: default to the directory name; confirm with the user.
   - `domain`: optional (used by business-knowledge layers in later batches); default `null`.

3. **Write `cairnkit.yaml`** at the project root (single-repo default):
   ```yaml
   project: <name>
   domain: null
   repos:
     - name: <name>
       path: .
   ```
   > Multi-repo (`repos[]` with several entries), the knowledge-repo URL, notification
   > webhook, and budgets are added in later batches. B1 only needs the block above.

4. **Git degradation:** if this directory is not a Git repo, that is fine for B1 —
   cairnkit runs in pure-local mode (no knowledge repo is connected yet). Do not error.

5. **Verify:** run `python3 -m cairnkit --root . config show`. Exit `0` with `has_run: false`
   is the expected, healthy result (config valid; no run started yet). Tell the user
   `/flow-run` is ready.
