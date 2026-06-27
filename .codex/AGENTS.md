# cairnkit for Codex — supplement

This supplements the root `AGENTS.md` cairnkit block with Codex-specific notes.

- **Role personas** are installed at `~/.codex/cairnkit/roles/*.md`. At each stage, read the
  mapped role file and adopt it as your persona for that stage's work and artifact.
- **Commands** are installed as Codex prompts named `cairnkit-*` (e.g. `cairnkit-flow-run`,
  `cairnkit-team-init`). Use them as entry points.
- **No native sub-agents**: one Codex agent plays all roles sequentially. Preserve role
  separation at the artifact level (one `NN-*.md` per stage under `docs/workflows/<run-id>/`).
- The `cairn` binary is the only writer of state; honour gates (exit 3) and CLARIFY pauses.
