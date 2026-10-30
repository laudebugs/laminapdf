#!/usr/bin/env bash
# The Mac half of the spike in one go (Apple silicon, Xcode 16+): the XCFramework and XCTest
# (macOS slice, then an iOS simulator if one is installed), the cargo-swift probe, the Android
# NDK (downloaded into ~/.cache/lamina-ffi when ANDROID_NDK_HOME is unset), the Android
# libraries with and without `text`, the 16 KB check, the Kotlin JVM tests on the macOS
# library, the AAR and the size table. Writes target/mac-report.md; paste it into the README.
#
# Needs: rustup, Xcode, a JDK 17+ and Gradle 8 on PATH (for the Kotlin half), curl, hdiutil.
set -uo pipefail
cd "$(dirname "$0")/.."
report=target/mac-report.md
mkdir -p target
: >"$report"
log() { echo "$@" | tee -a "$report"; }
run() { # title, command…: runs it, records pass/fail and the last lines
  local title=$1; shift
  echo "==> $title"
  if "$@" >"target/step.log" 2>&1; then log "- [x] $title"; else log "- [ ] **FAILED:** $title"; fi
  { echo '```'; tail -25 target/step.log; echo '```'; } >>"$report"
}

log "## Environment"
{ echo '```'; sw_vers; uname -m; xcodebuild -version; swift --version 2>&1 | head -1; rustc --version
  java -version 2>&1 | head -1; gradle --version 2>/dev/null | grep '^Gradle'; echo '```'; } >>"$report"

log "## Apple"
run "XCFramework (scripts/build-ios.sh)" scripts/build-ios.sh
run "Swift tests, macOS slice (swift test)" bash -c 'cd swift && swift test'
sim=$(xcrun simctl list devices available | sed -n 's/^ *\(iPhone [^(]*\) (.*/\1/p' | head -1 | sed 's/ *$//')
if [[ -n "$sim" ]]; then
  run "Swift tests, iOS simulator ($sim)" bash -c "cd swift && xcodebuild test -scheme Lamina -destination 'platform=iOS Simulator,name=$sim'"
else
  log "- iOS simulator tests: no simulator installed (skipped)"
fi
run "XCFramework zip" bash -c 'cd swift/build && ditto -c -k --keepParent LaminaFFIFFI.xcframework ../../target/LaminaFFIFFI.xcframework.zip'
run "Apple libraries without text" scripts/build-ios.sh --no-text
run "cargo-swift 0.11.1 probe" bash -c '
  command -v cargo-swift >/dev/null || cargo install cargo-swift@0.11.1 --locked
  rm -rf /tmp/l6a-cargo-swift && mkdir /tmp/l6a-cargo-swift && rsync -a --exclude target --exclude swift/build . /tmp/l6a-cargo-swift/
  cd /tmp/l6a-cargo-swift
  # cargo-swift reads the path dependency relative to the crate: point it back at the repo.
  sed -i "" "s|path = \"../../crates/lamina\"|path = \"'"$PWD"'/../../crates/lamina\"|" Cargo.toml
  cargo swift package -y -p ios macos -n LaminaCargoSwift -r --lib-type static \
    --exclude-arch x86_64-apple-ios --exclude-arch x86_64-apple-darwin && du -sh LaminaCargoSwift/*.xcframework'

log "## Android"
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  ver=r28c
  ANDROID_NDK_HOME="$HOME/.cache/lamina-ffi/android-ndk-$ver"
  if [[ ! -d "$ANDROID_NDK_HOME" ]]; then
    mkdir -p "$HOME/.cache/lamina-ffi"
    curl -fsSL -o /tmp/ndk.dmg "https://dl.google.com/android/repository/android-ndk-$ver-darwin.dmg"
    hdiutil attach -nobrowse -readonly /tmp/ndk.dmg -mountpoint /tmp/ndk-mnt
    cp -R /tmp/ndk-mnt/AndroidNDK*.app/Contents/NDK "$ANDROID_NDK_HOME"
    hdiutil detach /tmp/ndk-mnt && rm /tmp/ndk.dmg
  fi
fi
export ANDROID_NDK_HOME
log "NDK: $(grep Pkg.Revision "$ANDROID_NDK_HOME/source.properties")"
run "Android libraries (scripts/build-android.sh)" scripts/build-android.sh
run "Android libraries without text" scripts/build-android.sh --no-text
run "16 KB alignment (scripts/check-alignment.sh)" scripts/check-alignment.sh
run "Kotlin JVM tests on the macOS library" scripts/build-kotlin.sh
run "AAR (gradle aar)" bash -c 'cd kotlin && gradle aar --no-daemon -q && unzip -l ../target/lamina-ffi.aar'

log "## Sizes"
scripts/size-report.sh >/dev/null && cat target/size-report.md >>"$report"
echo "report: $report"
