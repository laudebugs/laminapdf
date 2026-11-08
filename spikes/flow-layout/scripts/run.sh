#!/usr/bin/env sh
# Runs a command for the spike with the workspace's skrifa pin moved from =0.42.0 to =0.42.1, the
# patch release Typst 0.15.1 needs (through hayro and vello_cpu). The root Cargo.toml is restored
# however the command ends: the product build never sees the bump.
#   scripts/run.sh cargo run --release
set -eu
here="$(cd "$(dirname "$0")/.." && pwd)"
root="$here/../.."
cp "$root/Cargo.toml" "$here/target/Cargo.toml.orig" 2>/dev/null || { mkdir -p "$here/target"; cp "$root/Cargo.toml" "$here/target/Cargo.toml.orig"; }
trap 'cp "$here/target/Cargo.toml.orig" "$root/Cargo.toml"' EXIT INT TERM
sed -i.bak 's/skrifa = { version = "=0.42.0"/skrifa = { version = "=0.42.1"/' "$root/Cargo.toml" && rm -f "$root/Cargo.toml.bak"
cd "$here" && "$@"
