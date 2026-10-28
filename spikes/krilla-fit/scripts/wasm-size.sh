#!/usr/bin/env sh
# Builds the size probe for wasm32-unknown-unknown in three variants and prints raw and gzip
# sizes (wasm-opt not applied; the relative difference is what matters).
set -eu
cd "$(dirname "$0")/../wasm-size"
for f in "" krilla-gen lamina-gen; do
  cargo build --quiet --release --target wasm32-unknown-unknown ${f:+--features $f}
  w=target/wasm32-unknown-unknown/release/krilla_fit_wasm_size.wasm
  printf '%-12s raw %8d B  gzip %8d B\n' "${f:-cos-only}" "$(wc -c < "$w")" "$(gzip -9c "$w" | wc -c)"
done
