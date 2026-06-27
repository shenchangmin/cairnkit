#!/usr/bin/env bash
# cairnkit → Codex adapter. Projects the shared cairnkit content into Codex's home (~/.codex/),
# mirroring everything-claude-code's sync-ecc-to-codex.sh. Idempotent and merge-safe: it never
# clobbers your existing AGENTS.md / config — it writes inside clearly-marked cairnkit blocks.
#
# Usage:  ./scripts/sync-to-codex.sh [--dry-run]
# Requires: the `cairn` binary on PATH (cargo install --path .).
set -euo pipefail

MODE="apply"; [[ "${1:-}" == "--dry-run" ]] && MODE="dry-run"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
BEGIN="<!-- BEGIN cairnkit (managed) -->"
END="<!-- END cairnkit (managed) -->"

log() { printf '[cairnkit-codex] %s\n' "$*"; }
do_or_echo() { if [[ "$MODE" == "dry-run" ]]; then printf '[dry-run] %s\n' "$*"; else eval "$*"; fi; }

command -v cairn >/dev/null 2>&1 || { log "WARN: 'cairn' not on PATH — install it: cargo install --path ."; }

# 1. Role mandates → ~/.codex/cairnkit/roles/  (so the single Codex agent can read+adopt each role)
ROLES_DEST="$CODEX_HOME/cairnkit/roles"
do_or_echo "mkdir -p '$ROLES_DEST'"
do_or_echo "cp '$REPO_ROOT/agents/'*.md '$ROLES_DEST/'"
log "roles -> $ROLES_DEST"

# 1b. Role agents (Codex native multi-agent profiles) -> ~/.codex/agents/
AGENTS_DEST="$CODEX_HOME/agents"
do_or_echo "mkdir -p '$AGENTS_DEST'"
do_or_echo "cp '$REPO_ROOT/.codex/agents/'*.toml '$AGENTS_DEST/'"
log "role agents -> $AGENTS_DEST (needs multi_agent=true in config.toml)"

# 2. Commands → ~/.codex/prompts/cairnkit-*.md  (Codex custom prompts ≈ slash commands)
PROMPTS_DEST="$CODEX_HOME/prompts"
do_or_echo "mkdir -p '$PROMPTS_DEST'"
for f in "$REPO_ROOT/commands/"*.md; do
  name="$(basename "$f")"
  do_or_echo "cp '$f' '$PROMPTS_DEST/cairnkit-$name'"
done
log "commands -> $PROMPTS_DEST/cairnkit-*.md"

# 3. AGENTS.md → injected into ~/.codex/AGENTS.md inside the managed block (merge-safe)
AGENTS_GLOBAL="$CODEX_HOME/AGENTS.md"
do_or_echo "mkdir -p '$CODEX_HOME'"
# point the role-file references at the installed location
BODY="$(sed 's#agents/\([a-z-]*\)\.md#~/.codex/cairnkit/roles/\1.md#g' "$REPO_ROOT/AGENTS.md")"
if [[ "$MODE" == "dry-run" ]]; then
  log "[dry-run] would inject cairnkit block into $AGENTS_GLOBAL"
else
  touch "$AGENTS_GLOBAL"
  # strip any previous cairnkit block, then append a fresh one
  awk -v b="$BEGIN" -v e="$END" '
    $0==b{skip=1} !skip{print} $0==e{skip=0}' "$AGENTS_GLOBAL" > "$AGENTS_GLOBAL.tmp" || true
  { mv "$AGENTS_GLOBAL.tmp" "$AGENTS_GLOBAL"; printf '\n%s\n%s\n%s\n' "$BEGIN" "$BODY" "$END" >> "$AGENTS_GLOBAL"; }
  log "AGENTS.md -> $AGENTS_GLOBAL (cairnkit block)"
fi

# 4. Codex config baseline (non-destructive note; user merges as needed)
if [[ -f "$REPO_ROOT/.codex/config.toml" ]]; then
  log "baseline config at .codex/config.toml — review and merge into $CODEX_HOME/config.toml if desired"
fi

log "done ($MODE). In Codex, open your project and use the cairnkit-* prompts (e.g. cairnkit-flow-run)."
