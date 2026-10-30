#!/usr/bin/env bash
# Writes the size table (Markdown) to target/size-report.md: every built library per ABI and
# slice, raw and gzip -9 (a proxy for the download size), with and without `text`, plus the
# XCFramework zip and the AAR when they exist. Run after build-ios.sh / build-android.sh.
set -euo pipefail
cd "$(dirname "$0")/.."
row() { # label path
  [[ -f "$2" ]] || return 0
  local raw gz
  raw=$(wc -c <"$2" | tr -d ' ')
  gz=$(gzip -9 -c "$2" | wc -c | tr -d ' ')
  printf '| %s | `%s` | %s | %s |\n' "$1" "${2#target/}" "$(numfmt --to=iec --suffix=B "$raw" 2>/dev/null || echo "$raw")" \
    "$(numfmt --to=iec --suffix=B "$gz" 2>/dev/null || echo "$gz")"
}
{
  echo "| Artefact | Path | Raw | gzip -9 |"
  echo "| --- | --- | --- | --- |"
  for v in "" "-notext"; do
    label=${v:+ (no text)}
    for abi in arm64-v8a x86_64 armeabi-v7a; do row "Android $abi .so$label" "target/jniLibs$v/$abi/liblamina_ffi.so"; done
  done
  for d in target target/notext; do
    label=""; [[ "$d" == target/notext ]] && label=" (no text)"
    for t in aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin; do
      row "Apple $t .dylib$label (app-size estimate)" "$d/$t/mobile-release/liblamina_ffi.dylib"
      row "Apple $t .a$label (static, before dead-stripping)" "$d/$t/mobile-release/liblamina_ffi.a"
    done
  done
  for f in target/*.xcframework.zip target/*.aar; do [[ -f "$f" ]] && row "$(basename "$f")" "$f"; done
  row "Linux x86_64 .so (host tests)" "target/mobile-release/liblamina_ffi.so"
} | tee target/size-report.md
