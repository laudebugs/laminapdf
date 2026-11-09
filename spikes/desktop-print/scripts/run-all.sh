#!/usr/bin/env bash
# macOS and Linux: build the spike, print every fixture through every path into PDFs (no
# dialog), score them against the fixtures, then open each path's print dialog for a screenshot.
# Results land in results/: <label>-jobs.csv, <label>-fidelity.csv, dialogs/<label>-*.png.
#
#   LAMINA_PDF_PDFIUM_DIR=… scripts/run-all.sh <label>
#
# Linux needs a display (Xvfb) and, for path C, CUPS with a cups-pdf printer (see README).
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
label=${1:?usage: run-all.sh <label>}
: "${LAMINA_PDF_PDFIUM_DIR:?set it to the folder printed by cargo xtask fetch-pdfium}"
cd "$here"

pnpm install --frozen-lockfile
pnpm tauri build --no-bundle
(cd src-tauri && cargo build --release --bin compare)
app=src-tauri/target/release/desktop-print
mkdir -p results/out results/dialogs
scripts/specs.sh > "results/$label-machine.txt"
# On macOS, keep the display awake: AppKit and WebKit need a live window.
if [ "$(uname)" = Darwin ]; then caffeinate -d -i -w $$ & fi

# Runs "$2" with a deadline of "$1" seconds (macOS has no timeout(1)).
run() {
  "$2" &
  local pid=$! code=0
  (sleep "$1" && kill "$pid" 2>/dev/null) &
  local watchdog=$!
  wait "$pid" || code=$?
  kill "$watchdog" 2>/dev/null || true
  if [ "$code" -ne 0 ]; then echo "$2 ($LAMINA_SPIKE_MODE) exited $code (143: timed out after $1 s)" >&2; fi
  return 0
}

rm -f results/out/"$label"-*.pdf "results/$label-jobs.csv" "results/$label-fidelity.csv"
LAMINA_SPIKE_MODE=auto LAMINA_SPIKE_LABEL="$label" run 1800 "$app"
src-tauri/target/release/compare fixtures results/out "results/$label-fidelity.csv"

screenshot() {
  case "$(uname)" in
    Darwin) screencapture -x "$1" ;;
    *) xwd -root -silent | convert xwd:- "$1" ;;
  esac
}
for path in raster vector webview windowprint; do
  LAMINA_SPIKE_MODE="dialog-$path" LAMINA_SPIKE_LABEL="$label" "$app" > "results/dialogs/$label-$path.log" 2>&1 &
  pid=$!
  sleep 12
  kill -0 "$pid" 2>/dev/null || echo "the app had exited before the $path screenshot" >> "results/dialogs/$label-$path.log"
  screenshot "results/dialogs/$label-$path.png" || echo "no screenshot for $path" >&2
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
done
echo "results in $here/results"
