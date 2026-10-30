#!/usr/bin/env bash
# Builds the Android libraries with cargo-ndk for arm64-v8a, x86_64 and armeabi-v7a (measured
# only) into target/jniLibs[-notext]/<abi>/, linked for 16 KB pages, and the Kotlin bindings.
# Needs ANDROID_NDK_HOME (NDK r28+, whose default is already 16 KB) and cargo-ndk 4.1.2.
#
# Usage: scripts/build-android.sh [--no-text]
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to an NDK r28+}"
features=()
out=target/jniLibs
if [[ "${1:-}" == "--no-text" ]]; then
  features=(--no-default-features --target-dir target/notext)
  out=target/jniLibs-notext
fi
command -v cargo-ndk >/dev/null || cargo install cargo-ndk@4.1.2 --locked
rustup target add aarch64-linux-android x86_64-linux-android armv7-linux-androideabi >/dev/null
rm -rf "$out"
# max-page-size: explicit, so an older NDK can't silently produce 4 KB-aligned libraries.
RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" \
  cargo ndk -t arm64-v8a -t x86_64 -t armeabi-v7a --platform 24 -o "$out" \
  build --profile mobile-release "${features[@]}"
find "$out" -name '*.so' -print
