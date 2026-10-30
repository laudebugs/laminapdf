#!/usr/bin/env bash
# Builds the host library and the Kotlin bindings, then runs the JVM tests (JNA loads the
# host .so/.dylib). Needs a JDK (17+) and Gradle 8.
#
# Bindings are generated from the unstripped `release` build: `mobile-release` strips the
# symbols UniFFI reads its metadata from. The tests load the stripped `mobile-release` build,
# the one that ships.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release
cargo build --profile mobile-release
ext=so; [[ "$(uname)" == Darwin ]] && ext=dylib
rm -rf target/bindings/kotlin
cargo run -q --features bindgen --bin uniffi-bindgen -- generate --library "target/release/liblamina_ffi.$ext" \
  --language kotlin --no-format --out-dir target/bindings/kotlin
(cd kotlin && gradle test --no-daemon --rerun-tasks "$@")
