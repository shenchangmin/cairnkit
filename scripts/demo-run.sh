#!/usr/bin/env bash
# Demo: drive a full cairnkit workflow INIT -> DONE through the CLI, in a throwaway project.
# Usage:  ./scripts/demo-run.sh [full|lite|single]
# Requires the package installed (e.g. in the repo venv): pip install -e .
set -euo pipefail

MODE="${1:-lite}"
PY="${CAIRN_PY:-python3}"           # override with CAIRN_PY=/path/to/.venv/bin/python
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

stage() { "$PY" -m cairnkit --root . state show | "$PY" -c "import sys,json;print(json.load(sys.stdin)['stage'])"; }
paused() { "$PY" -m cairnkit --root . state resume | "$PY" -c "import sys,json;print(json.load(sys.stdin)['paused'])"; }
artifact_for() { case "$1" in
  ANALYSE_PRODUCT) echo 01-product.md;; ANALYSE_TECH) echo 02-tech.md;; ARCHITECT_BACKEND) echo 03-arch.md;;
  ARCHITECT_FRONTEND) echo 04-arch-fe.md;; IMPLEMENT) echo 05-implement.md;; BUILD_VERIFY) echo 06-build.md;;
  VISUAL_REVIEW) echo 07-visual.md;; E2E_VERIFY) echo 08-e2e.md;; TEST) echo 09-test.md;; ARCHIVE) echo 10-archive.md;;
  *) echo "";; esac; }

echo "== init =="
"$PY" -m cairnkit --root . state init --run-id "$RUN" >/dev/null
"$PY" -m cairnkit --root . config show

echo "== drive INIT -> DONE (mode: $MODE) =="
i=0
while [ "$(stage)" != "DONE" ]; do
  s="$(stage)"
  if [ "$s" = "INTENT_GATE" ]; then "$PY" -m cairnkit --root . state set-path-mode "$MODE" >/dev/null; fi
  f="$(artifact_for "$s")"
  if [ -n "$f" ]; then mkdir -p "docs/workflows/$RUN"; echo "# $s artifact" > "docs/workflows/$RUN/$f"; fi
  if [ "$(paused)" = "True" ]; then echo "   (CLARIFY pause -> approving) "; "$PY" -m cairnkit --root . state approve-clarify >/dev/null; fi
  printf "  %-22s -> " "$s"
  "$PY" -m cairnkit --root . state advance | "$PY" -c "import sys,json;print('now', json.load(sys.stdin)['stage'])"
  i=$((i+1)); [ $i -gt 30 ] && { echo "loop guard"; break; }
done

echo "== final =="
"$PY" -m cairnkit --root . state show | "$PY" -c "import sys,json;d=json.load(sys.stdin);print('stage:',d['stage']);print('history:',' -> '.join(d['history']))"
echo "(scratch project at: $DIR)"
