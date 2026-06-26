# Contributing to cairnkit

Thanks for your interest! cairnkit is the **engine** — a generic, reusable harness.
Contributions to the engine are welcome. (Note: your *own team's knowledge repo* is a
separate, private thing cairnkit feeds — that is never part of this project.)

## Ground rules

- **License**: by contributing you agree your contribution is licensed under MIT.
- **No secrets, ever**: credentials, webhooks, tokens go through env vars / config — never
  hard-coded, never committed. See `.gitignore`.
- **No internal/proprietary references**: keep examples generic (e.g. `domain: ecommerce`,
  not a real internal project name).
- **Philosophy**: the file system is the state machine. No databases, no central services,
  no heavyweight deps. If a change pulls in a server or a vector DB, it belongs in an
  optional v2 read-layer, not the core.
- **Knowledge is the point**: the workflow serves knowledge precipitation. Features that
  don't help inject / consume / extract / curate knowledge are low priority.

## Development

- Engine logic that is deterministic and verifiable lives in the Rust core
  (`src/`, the `cairn` binary) and **must have tests** (`cargo test`).
- Fuzzy, model-driven logic lives in Markdown (`skills/`, `agents/`, `commands/`).
- Run `cargo test` before opening a PR.

## Workflow

1. Open an issue describing the change first for anything non-trivial.
2. Branch, implement with tests, ensure `cargo test` is green.
3. Open a PR with a clear description and rationale.
4. Keep diffs surgical — match existing style.

See [`docs/`](docs/) (`01`–`06`) for the full problem analysis, requirements, research,
trade-offs, design, and open-source design.
