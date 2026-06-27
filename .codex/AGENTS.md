# cairnkit for Codex — supplement

Supplements the root `AGENTS.md` cairnkit block with Codex-specific notes.

- **Role agents** are installed as `~/.codex/agents/<role>.toml` (product, tech, architect-be/-fe,
  dev, verify, visual, archiver, doc-collector, codebase-profiler, knowledge-builder), each with its
  own reasoning effort, sandbox, and `developer_instructions` (the role mandate). The parent
  orchestrator **dispatches** the stage's role agent — real role isolation, like Claude Code's
  Task sub-agents. Requires `multi_agent = true` (see config.toml).
- **Full role mandates** are mirrored at `~/.codex/cairnkit/roles/<role>.md`.
- **Commands** are installed as Codex prompts named `cairnkit-*` (e.g. `cairnkit-flow-run`).
- The `cairn` binary is the only writer of state; honour gates (exit 3) and CLARIFY pauses.
