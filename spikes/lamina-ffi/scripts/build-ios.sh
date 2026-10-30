#!/usr/bin/env bash
# Builds the XCFramework (ios-arm64, ios-arm64-simulator, macos-arm64 for host tests) and the
# Swift bindings into swift/build/, for `swift test` (macOS slice) or Xcode. macOS only (Xcode).
#
# Usage: scripts/build-ios.sh [--no-text]   (--no-text: the libraries without `text`, for sizes only)
# Bash 3.2 (macOS's /bin/bash) safe: no empty arrays expanded under `set -u`.
# The bindings come from the unstripped `release` dylib: `mobile-release` strips UniFFI's
# metadata symbols. The libraries in the XCFramework are the `mobile-release` ones.
set -euo pipefail
cd "$(dirname "$0")/.."
targets=(aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin)
rustup target add "${targets[@]}" >/dev/null
if [[ "${1:-}" == "--no-text" ]]; then
  # Size measurement only: the libraries without fonts and the builder, in their own target dir.
  for t in "${targets[@]}"; do
    cargo build --profile mobile-release --target "$t" --no-default-features --target-dir target/notext
  done
  exit 0
fi
cargo build --release
for t in "${targets[@]}"; do
  cargo build --profile mobile-release --target "$t"
done
gen=target/bindings/swift
rm -rf "$gen"
cargo run -q --features bindgen --bin uniffi-bindgen -- generate --library target/release/liblamina_ffi.dylib \
  --language swift --no-format --out-dir "$gen"
out=swift/build
rm -rf "$out/Sources/LaminaFFI" "$out/headers" "$out/LaminaFFIFFI.xcframework"
mkdir -p "$out/Sources/LaminaFFI" "$out/headers"
cp "$gen/LaminaFFI.swift" "$out/Sources/LaminaFFI/"
cp "$gen/LaminaFFIFFI.h" "$out/headers/"
cp "$gen/LaminaFFIFFI.modulemap" "$out/headers/module.modulemap"
args=()
for t in "${targets[@]}"; do
  args+=(-library "target/$t/mobile-release/liblamina_ffi.a" -headers "$out/headers")
done
xcodebuild -create-xcframework "${args[@]}" -output "$out/LaminaFFIFFI.xcframework"
echo "built $out/LaminaFFIFFI.xcframework"
