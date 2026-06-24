# Core engineering mechanics (M10, cross-cutting)

These apply to every stage and agent.

## Context firewall
Role agents run as Task sub-agents and interact only through files + the `cairnkit` CLI —
never directly with each other. A sub-agent failure must not pollute the orchestrator's
context: re-dispatch instead of absorbing its error.

## Three-level degradation
When a capability is missing, degrade rather than hard-fail:
1. Preferred tool/path → 2. a simpler local equivalent → 3. a documented manual step.
Example: no knowledge webhook → local `.cairnkit/notifications.log` reminder (flow continues).

## Budgets
- Knowledge query: per-stage line budget (`kb query --budget`), hard-capped, dropped items reported.
- Codebase profiling (import): ~60 searches; stop and note what was not covered.
- First-screen load discipline: keep injected context lean (~150 lines), drill down on demand.

## Notifications (M10.1)
Key moments (CLARIFY pending / blocked / DONE) trigger `cairnkit notify`. The webhook URL is an
env var named in `cairnkit.yaml` (`notify.feishu_webhook_env`) — never hardcoded. Channels are
pluggable; Feishu is the first. Unconfigured → local file reminder, no error.
