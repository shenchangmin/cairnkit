#!/usr/bin/env bash
# Demo: drive a full cairnkit workflow INIT -> DONE through the CLI, in a throwaway project.
# Usage:  ./scripts/demo-run.sh [full|lite|single]
# Uses the installed `cairn` console script — no python3 command-name dependency.
set -euo pipefail

MODE="${1:-lite}"
CK="${CAIRN:-cairn}"        # the installed console script
DIR="$(mktemp -d)"
RUN="demo-$(date +%s)"

cd "$DIR"
cat > cairnkit.yaml <<YAML
project: demo
domain: ads
repos:
  - name: demo
    path: .
YAML

stage()  { $CK --root . state show   | sed -E 's/.*"stage": *"([^"]+)".*/\1/'; }
paused() { $CK --root . state resume | grep -q '"paused": true' && echo True || echo False; }
artifact_for() { case "$1" in
  ANALYSE_PRODUCT) echo 01-product.md;; ANALYSE_TECH) echo 02-tech.md;; ARCHITECT_BACKEND) echo 03-arch.md;;
  ARCHITECT_FRONTEND) echo 04-arch-fe.md;; IMPLEMENT) echo 05-implement.md;; BUILD_VERIFY) echo 06-build.md;;
  VISUAL_REVIEW) echo 07-visual.md;; E2E_VERIFY) echo 08-e2e.md;; TEST) echo 09-test.md;; ARCHIVE) echo 10-archive.md;;
  *) echo "";; esac; }

echo "== init =="
$CK --root . state init --run-id "$RUN" >/dev/null
$CK --root . config show

echo "== drive INIT -> DONE (mode: $MODE) =="
i=0
while [ "$(stage)" != "DONE" ]; do
  s="$(stage)"
  if [ "$s" = "INTENT_GATE" ]; then $CK --root . state set-path-mode "$MODE" >/dev/null; fi
  f="$(artifact_for "$s")"
  if [ -n "$f" ]; then mkdir -p "docs/workflows/$RUN"; echo "# $s artifact" > "docs/workflows/$RUN/$f"; fi
  if [ "$(paused)" = "True" ]; then echo "   (CLARIFY pause -> approving) "; $CK --root . state approve-clarify >/dev/null; fi
  printf "  %-22s -> " "$s"
  $CK --root . state advance | sed -E 's/.*"stage": *"([^"]+)".*/now \1/'
  i=$((i+1)); [ $i -gt 30 ] && { echo "loop guard"; break; }
done

echo "== final =="
echo "stage: $(stage)"
$CK --root . state show | sed -E 's/.*"history": \[([^]]*)\].*/history: \1/' | tr -d '"'
echo "(scratch project at: $DIR)"
