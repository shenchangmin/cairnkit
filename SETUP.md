# Setup — use cairnkit in any project

Two one-time installs, then a one-liner per project.

## 1. The `cairn` CLI (one-time, machine-wide) — ✅ already done on this machine

The plugin's commands shell out to the `cairn` console script, so it must be on your PATH.
Installed in an isolated environment via pipx:

```bash
pipx install -e /Users/mac/work/cairnkit
cairn --root . config show     # smoke test from any dir (a "cairnkit.yaml not found" error = it runs)
```

> Why pipx and not `pip install`: this machine's `python3` is Homebrew-managed (PEP 668), so a
> plain `pip install` is blocked. pipx gives a global `cairn` without touching system Python.
> `-e` (editable) means `cairn` tracks the repo, so engine updates need no reinstall.
> To update later: `pipx reinstall cairnkit`.

## 2. The Claude Code plugin (one-time)

This loads the slash commands (`/flow-run`, `/team-init`, …), the role agents, and the
orchestrator skill into Claude Code.

In a Claude Code session:

```
/plugin marketplace add /Users/mac/work/cairnkit
/plugin install cairnkit@cairnkit
```

(or use the interactive `/plugin` menu → *Add marketplace* → point at `/Users/mac/work/cairnkit`
→ *Install* `cairnkit`.) Reload Claude Code if it asks. Verify the commands appear by typing `/`
and looking for `flow-run`, `team-init`, etc.

## 3. Per project (each new repo you want to use it on)

Open the project in Claude Code, then:

```
/team-init            # generates cairnkit.yaml (single-repo default)
/flow-run <your feature request>
```

That's it. The orchestrator reads STATE, dispatches the right role agent per stage, writes each
artifact, and advances — pausing at CLARIFY for your approval. Use `/flow-status` anytime,
`/flow-import` to seed knowledge from an existing codebase, `/knowledge` for stats/lint/sync,
and `/evolve` + `/evolve:apply` to improve the harness itself (human-gated).

### Optional: share knowledge across projects (the moat)

To let knowledge precipitate across all your projects, point each project's `cairnkit.yaml` at a
shared knowledge Git repo:

```yaml
knowledge_repo:
  local: ~/.cairnkit/team-knowledge   # a local clone of an independent git repo
```

Create that repo once (`git init` + `cairn`'s repo skeleton), and `/flow-run` will pull on INIT
and push verified knowledge on ARCHIVE. Without it, knowledge stays project-local under
`docs/knowledge/`.

### Notifications (optional)

```yaml
notify:
  feishu_webhook_env: CAIRNKIT_FEISHU_WEBHOOK   # env var holding the webhook URL (never hardcode)
```

Unset → key-moment notifications degrade to a local `.cairnkit/notifications.log`.

---

For trying the engine directly via the CLI (no Claude Code), see [USAGE.md](USAGE.md).
