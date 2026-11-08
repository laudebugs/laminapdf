#!/usr/bin/env sh
# Builds the WebAssembly probe (wasm/) once per variant with the wasm-release profile, runs
# binaryen's wasm-opt as `cargo xtask wasm` does (-O1 with names, then stripped), and prints raw
# and gzip -9 sizes. binaryen version_133 is fetched into target/tools/ and checked by SHA-256
# (the pins in tools/xtask/src/wasm.rs). Run it through scripts/run.sh (Typst's skrifa pin).
set -eu
here="$(cd "$(dirname "$0")/.." && pwd)"
tools="$here/target/tools"
case "$(uname -m)-$(uname -s)" in
  x86_64-Linux) host=x86_64-linux; sum=2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489 ;;
  aarch64-Linux) host=aarch64-linux; sum=89c07ea56faf38d0fbecf36ca8ec0721756716185f265b568e133d427f299bf8 ;;
  arm64-Darwin) host=arm64-macos; sum=ad66da82ac13f163e424b1643f16c6dfcccc98b5966296b43e52d3cab04f84a8 ;;
  *) echo "no binaryen pin for this host" >&2; exit 1 ;;
esac
opt="$tools/binaryen-version_133/bin/wasm-opt"
if [ ! -x "$opt" ]; then
  mkdir -p "$tools"
  file="binaryen-version_133-$host.tar.gz"
  curl -fsSL --retry 3 -o "$tools/$file" "https://github.com/WebAssembly/binaryen/releases/download/version_133/$file"
  if command -v sha256sum >/dev/null; then found=$(sha256sum "$tools/$file"); else found=$(shasum -a 256 "$tools/$file"); fi
  [ "${found%% *}" = "$sum" ] || { echo "$file: SHA-256 ${found%% *}, want $sum" >&2; rm -f "$tools/$file"; exit 1; }
  tar -xzf "$tools/$file" -C "$tools"
fi
features="--enable-bulk-memory --enable-mutable-globals --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types --enable-multivalue"
mkdir -p "$here/target/wasm"
for variant in ${VARIANTS:-baseline own typst frames markdown template}; do
  (cd "$here/wasm" && cargo build --quiet --profile wasm-release --target wasm32-unknown-unknown --features "$variant")
  built="$here/wasm/target/wasm32-unknown-unknown/wasm-release/flow_layout_wasm.wasm"
  out="$here/target/wasm/$variant.wasm"
  # shellcheck disable=SC2086
  "$opt" -g -O1 $features "$built" -o "$out.named"
  # shellcheck disable=SC2086
  "$opt" --strip-debug --strip-producers $features "$out.named" -o "$out"
  printf '%-9s raw %9d B  gzip %9d B\n' "$variant" "$(wc -c < "$out")" "$(gzip -9c "$out" | wc -c)"
done
