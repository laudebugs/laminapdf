#!/usr/bin/env bash
# macOS and Linux: build the spike twice (without and with the COOP/COEP headers), run the
# isolation check with each build, then the benchmark with the plain one. CSVs land in results/.
#
#   LAMINA_PDF_PDFIUM_DIR=… scripts/run-all.sh <label> [full|quick]
#
# On Linux, run it under the display you want to measure (see README "Reproduce").
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
label=${1:?usage: run-all.sh <label> [full|quick]}
mode=${2:-full}
: "${LAMINA_PDF_PDFIUM_DIR:?set it to the folder printed by cargo xtask fetch-pdfium}"
cd "$here"

pnpm install --frozen-lockfile
mkdir -p bin results
pnpm tauri build --no-bundle
cp src-tauri/target/release/desktop-ipc bin/desktop-ipc-plain
pnpm tauri build --no-bundle --config src-tauri/tauri.coi.conf.json
cp src-tauri/target/release/desktop-ipc bin/desktop-ipc-coi
scripts/specs.sh > "results/$label-machine.txt"

# On macOS, keep the display awake until this script exits: WebKit sends no animation frames
# while it sleeps. The window is always on top for the same reason; don't lock the screen or
# switch Spaces during a run.
if [ "$(uname)" = Darwin ]; then caffeinate -d -i -w $$ & fi

# Runs "$2" with a deadline of "$1" seconds (macOS has no timeout(1)).
run() {
  "$2" &
  local pid=$! code=0
  (sleep "$1" && kill "$pid" 2>/dev/null) &
  local watchdog=$!
  wait "$pid" || code=$?
  kill "$watchdog" 2>/dev/null || true
  if [ "$code" -ne 0 ]; then echo "$2 ($LAMINA_SPIKE_LABEL) exited $code (143: timed out after $1 s)" >&2; fi
  return "$code"
}

LAMINA_SPIKE_MODE=coi LAMINA_SPIKE_LABEL="$label-headers-off" run 120 bin/desktop-ipc-plain
LAMINA_SPIKE_MODE=coi LAMINA_SPIKE_LABEL="$label-headers-on" run 120 bin/desktop-ipc-coi
LAMINA_SPIKE_MODE="$mode" LAMINA_SPIKE_LABEL="$label" run 5400 bin/desktop-ipc-plain
echo "results in $here/results"
