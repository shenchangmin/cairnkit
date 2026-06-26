# Setup — use cairnkit in any project

cairnkit's deterministic core is a **single self-contained `cairn` binary** (Rust, zero runtime
dependencies — no Python, no Node, no interpreter). Install the binary once, install the plugin
once, then it's a one-liner per project.

## 1. The `cairn` binary (one-time, machine-wide)

The plugin's commands call `cairn`, so it must be on your PATH. Two ways to get it:

**a) Download a prebuilt binary** (when releases are published): grab the `cairn` for your
platform from the GitHub releases page and drop it somewhere on PATH (e.g. `~/.local/bin/`),
then `chmod +x`.

**b) Build from source** (needs the Rust toolchain — [rustup.rs](https://rustup.rs)):

```bash
git clone https://github.com/shenchangmin/cairnkit && cd cairnkit
cargo install --path .          # builds + installs `cairn` to ~/.cargo/bin (on PATH)
cairn --version                 # smoke test from any directory
```

That's the whole runtime requirement — a single ~1.6 MB executable. No `pip`, no `python3`,
no environment to manage. Other machines just need the binary on PATH (or `cargo install`).

## 2. The Claude Code plugin (one-time)

Loads the slash commands (`/flow-run`, `/team-init`, …), role agents, and orchestrator skill.
In a Claude Code session:

```
/plugin marketplace add /path/to/cairnkit
/plugin install cairnkit@cairnkit
```

(or the interactive `/plugin` menu → *Add marketplace* → point at the cairnkit repo → *Install*.)
Reload if asked; type `/` and confirm `flow-run`, `team-init`, etc. appear.

## 3. Per project (each repo you use it on)

```
/team-init                       # generates cairnkit.yaml (single-repo default)
/flow-run <your feature request>
```

The orchestrator reads STATE, dispatches the right role agent per stage, writes each artifact,
and advances — pausing at CLARIFY for your approval. Other commands: `/flow-status`,
`/flow-import`, `/knowledge`, `/evolve` + `/evolve:apply`.

### Optional: share knowledge across projects (the moat)

Point each project's `cairnkit.yaml` at a shared knowledge Git repo so knowledge precipitates
across all your projects:

```yaml
knowledge_repo:
  local: ~/.cairnkit/team-knowledge   # a local clone of an independent git repo
```

Without it, knowledge stays project-local under `docs/knowledge/`.

### Notifications (optional)

```yaml
notify:
  feishu_webhook_env: CAIRNKIT_FEISHU_WEBHOOK   # env var holding the webhook URL (never hardcode)
```

Unset → key-moment notifications degrade to a local `.cairnkit/notifications.log`.

---

For trying the engine directly via the CLI (no Claude Code), see [USAGE.md](USAGE.md).
