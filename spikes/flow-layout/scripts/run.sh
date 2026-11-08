#!/usr/bin/env sh
# Runs a command for the spike with the workspace's skrifa pin moved from =0.42.0 to =0.42.1, the
# patch release Typst 0.15.1 needs (through hayro and vello_cpu). On exit the pin is moved back by
# the same substitution in reverse, so other edits to the root Cargo.toml made meanwhile survive.
# It refuses to start when the pin already reads =0.42.1 (another run, or one that was killed:
# put the pin back to =0.42.0 by hand). The product build never sees the bump.
#   scripts/run.sh cargo run --release
set -eu
here="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$here/../../Cargo.toml"
old='skrifa = { version = "=0.42.0"'
new='skrifa = { version = "=0.42.1"'
if grep -qF "$new" "$manifest"; then
  echo "run.sh: the root Cargo.toml already pins skrifa =0.42.1; another run may be going" >&2
  exit 1
fi
if ! grep -qF "$old" "$manifest"; then
  echo "run.sh: the root Cargo.toml no longer pins skrifa =0.42.0; update this script" >&2
  exit 1
fi
# Rewrites only the skrifa line, from one exact pin to the other.
swap() { tmp="$manifest.flow-layout"; sed "/^skrifa = /s/\"=$1\"/\"=$2\"/" "$manifest" > "$tmp" && mv "$tmp" "$manifest"; }
trap 'swap 0.42.1 0.42.0' EXIT INT TERM
swap 0.42.0 0.42.1
cd "$here" && "$@"
