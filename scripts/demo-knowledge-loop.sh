#!/usr/bin/env bash
# Demo: close the cairnkit knowledge loop in a throwaway project.
#   run 1 (extract)  seeds a knowledge entry from a candidate file
#   build-index      recomputes the 3-level index/catalogs (explicit, visible step)
#   run 2 (query)    a later run retrieves the entry seeded by run 1 -> the loop closes
# Usage:  ./scripts/demo-knowledge-loop.sh
# Resolves `cairn` self-containedly (see resolve_cairn below) so a fresh `git clone &&
# cargo build` works out of the box. Override with CAIRN=/path/to/cairn.
set -euo pipefail

# Capture the repo root from the script's own location BEFORE we cd into the temp dir,
# so target/{release,debug}/cairn resolve relative to the checkout, not the scratch project.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Resolve the cairn binary, in precedence:
#   1. $CAIRN explicit override always wins.
#   2. a freshly-built binary in the checkout: target/release then target/debug.
#   3. `cairn` on PATH.
#   4. build it (cargo build) and use target/debug/cairn; clear error + non-zero if that fails.
resolve_cairn() {
  if [ -n "${CAIRN:-}" ]; then echo "$CAIRN"; return 0; fi
  if [ -x "$REPO_ROOT/target/release/cairn" ]; then echo "$REPO_ROOT/target/release/cairn"; return 0; fi
  if [ -x "$REPO_ROOT/target/debug/cairn" ]; then echo "$REPO_ROOT/target/debug/cairn"; return 0; fi
  if command -v cairn >/dev/null 2>&1; then command -v cairn; return 0; fi
  echo "== no cairn found — building from $REPO_ROOT (cargo build) ==" >&2
  if ( cd "$REPO_ROOT" && cargo build --quiet ) && [ -x "$REPO_ROOT/target/debug/cairn" ]; then
    echo "$REPO_ROOT/target/debug/cairn"; return 0
  fi
  echo "ERROR: could not find or build the cairn binary. Run 'cargo build' in $REPO_ROOT first." >&2
  return 1
}

CK="$(resolve_cairn)"        # the resolved binary (override with CAIRN=)
echo "== using cairn: $CK ($("$CK" --version 2>/dev/null || echo 'version unknown')) =="
DIR="$(mktemp -d)"           # throwaway project
trap 'rm -rf "$DIR"' EXIT    # clean up the scratch dir (nicer for CI)
cd "$DIR"

echo "== setup: throwaway project =="
cat > cairnkit.yaml <<'YAML'
project: demo-loop
domain: ecommerce
repos:
  - name: demo-loop
    path: .
YAML
mkdir -p docs/workflows/run1
cat > docs/workflows/run1/knowledge-candidates.json <<'JSON'
[{
  "id": "TK-DEMO-1",
  "title": "Build the index after extract, before query",
  "category": "tech",
  "type": "process",
  "knowledge_class": "point",
  "layer": "L1",
  "tags": ["demo", "loop"],
  "applicable_phases": ["ANALYSE_TECH"],
  "contributors": ["archiver"],
  "body": "This body is comfortably over eighty characters so the strict extraction gate accepts it as a real transferable knowledge entry."
}]
JSON

echo "== run 1: extract — seed knowledge =="
# Writes docs/knowledge/tech-wiki/TK-DEMO-1.md; prints {"written":["TK-DEMO-1"],"rejected":[]}.
"$CK" --root . kb extract --from docs/workflows/run1

echo "== build-index (explicit step between extract and query) =="
# extract does NOT auto-index (TK-DOG-004): a fresh entry is queryable only after build-index
# rebuilds the human-readable catalogs. `query` itself reads entries directly via iter_entries,
# so the grep below would pass even without this step — we run it because it is the canonical,
# visible shape of the loop and what real runs rely on for their catalogs, not a grep prerequisite.
"$CK" --root . kb build-index

echo "== run 2: query — consume knowledge =="
# The loop closes here: a later run, querying the same stage, gets the entry run 1 seeded.
OUT="$("$CK" --root . kb query --stage ANALYSE_TECH --budget 300)"
echo "$OUT"

# Loop-closure assertion: the seeded id must appear in run-2's query output (its injected_ids).
# Asserts the id string only — never a date or ref_count — so the demo is deterministic.
if echo "$OUT" | grep -q '"TK-DEMO-1"'; then
  echo "LOOP CLOSED: run-2 query returned 'TK-DEMO-1', the entry seeded in run-1."
else
  echo "FAIL: seeded entry TK-DEMO-1 not returned by run-2 query" >&2
  exit 1
fi

# Forced-failure spot check (manual): re-run the SAME query with a wrong stage and the grep
# misses, so the identical exit-1 path fires non-zero — proving the script fails loudly:
#   OUT2="$("$CK" --root . kb query --stage E2E_VERIFY --budget 300)"
#   echo "$OUT2" | grep -q '"TK-DEMO-1"' || echo "spot check: wrong stage correctly returns no entry"
