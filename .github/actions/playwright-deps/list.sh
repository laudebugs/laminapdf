#!/usr/bin/env bash
# Writes to $LIST the Ubuntu packages Playwright's browsers ($BROWSERS) still need, one per line:
# what `playwright install-deps` would install, dependencies included, as its `--dry-run` reports
# them (`apt-get install -s`). An empty file means none. Run inside the action's retry, after a
# fresh `apt-get update`, so the simulation reads current indexes.
set -euo pipefail
bash "$APT_UPDATE"
# Exits 1 when packages are missing, which isn't a failure here; its output says which case it is.
# shellcheck disable=SC2086
out=$(packages/sdk/node_modules/.bin/playwright install-deps --dry-run $BROWSERS 2>&1) || true
if grep -qx 'All system dependencies are installed.' <<< "$out"; then
  : > "$LIST"
  exit 0
fi
# "Missing system dependencies (N):", then the N packages, two spaces before each.
expected=$(sed -nE 's/^Missing system dependencies \(([0-9]+)\):$/\1/p' <<< "$out")
sed -nE '/^Missing system dependencies \(/,$ s/^  ([a-z0-9][a-z0-9+.:-]*)$/\1/p' <<< "$out" > "$LIST"
found=$(wc -l < "$LIST" | tr -d ' ')
if [ -z "$expected" ] || [ "$found" != "$expected" ]; then
  printf '%s\n' "$out"
  echo "::error::couldn't read the package list from playwright install-deps --dry-run (${found} of ${expected:-no} packages)"
  exit 1
fi
