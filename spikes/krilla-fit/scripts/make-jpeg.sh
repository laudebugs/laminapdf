#!/usr/bin/env sh
# Regenerates assets/photo.jpg: the spike writes a deterministic RGB pattern as PNG and macOS
# `sips` encodes it as a baseline JPEG. Committed because every pure-Rust JPEG encoder on
# crates.io is licensed (MIT OR Apache-2.0) AND IJG, which deny.toml does not allow.
set -eu
cd "$(dirname "$0")/.."
tmp="target/photo-source.png"
mkdir -p target
cargo run --release --quiet -- photo-source "$tmp"
sips -s format jpeg -s formatOptions 80 "$tmp" --out assets/photo.jpg >/dev/null
ls -l assets/photo.jpg
