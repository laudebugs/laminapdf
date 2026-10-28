#!/usr/bin/env bash
# SPEC case 6 (portability) plus lint and tests, in one go.
# Usage: scripts/check-targets.sh   (from anywhere; needs the three targets installed via rustup)
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

step() { printf '\n==> %s\n' "$*"; }

for target in wasm32-unknown-unknown aarch64-apple-ios aarch64-linux-android; do
  step "cargo check --lib --target $target"
  cargo check --locked --lib --target "$target"
done

step "cargo build --release --target wasm32-unknown-unknown (C-ABI shim)"
cargo build --locked --release --lib --target wasm32-unknown-unknown
wasm=target/wasm32-unknown-unknown/release/lamina_pdf_basics.wasm
printf 'wasm: %s (%s bytes)\n' "$wasm" "$(wc -c < "$wasm" | tr -d ' ')"

step "cargo fmt --check"
cargo fmt --check

step "cargo clippy (all targets, -D warnings)"
cargo clippy --locked --all-targets --all-features -- -D warnings

step "cargo clippy --target wasm32-unknown-unknown (lints the shim)"
cargo clippy --locked --lib --target wasm32-unknown-unknown -- -D warnings

step "cargo test"
cargo test --locked

step "library must not touch fs/net/process"
if grep -rnE 'std::(fs|net|process)|std::thread|SystemTime|Instant' src/; then
  echo "forbidden API in src/" >&2
  exit 1
fi

step "all checks passed"
