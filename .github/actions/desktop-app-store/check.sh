#!/usr/bin/env bash
# Checks the Mac App Store package before it is uploaded (todo D8c; ADR 0014 §11): what the store
# would reject or what would break the app's promises, found here instead of in App Store Connect.
#
# - Universal: the app, the engine and PDFium's library each hold arm64 and x86_64.
# - The version the plan gave (up to three numbers) and the build number, which must rise per upload.
# - Entitlements: exactly the files in apps/desktop/src-tauri/app-store/, the app's and the engine's,
#   and none on PDFium's library; with a real signature, the two identifiers the profile grants.
# - No updater key or configuration: the updater's public key and the endpoint its configuration
#   named are not in the app, which has no plugin config to trust or fetch an update with. The
#   updater's and the crash reports' code, and their addresses, are still linked but unreachable
#   (`Install::AppStore` and `crash::AVAILABLE` turn them off); compiling them out is a follow-up.
# - Signed: a store certificate signs the app, the provisioning profile is inside, and the
#   package carries the installer certificate. Ad hoc: none of the three.
#
# Reads: APP, PKG, ENTITLEMENTS (the app-store/ folder), VERSION, BUILD, SIGNED (true or false).
set -euo pipefail

: "${APP:?}" "${PKG:?}" "${ENTITLEMENTS:?}" "${VERSION:?}" "${BUILD:?}" "${SIGNED:?}"
fail() {
  echo "::error::$*"
  exit 1
}
plist="$APP/Contents/Info.plist"
read_plist() { /usr/libexec/PlistBuddy -c "Print :$1" "$plist"; }

exe="$APP/Contents/MacOS/$(read_plist CFBundleExecutable)"
engine="$APP/Contents/MacOS/laminapdf-desktop-engine"
lib="$APP/Contents/Resources/pdfium/libpdfium.dylib"

for file in "$exe" "$engine" "$lib"; do
  [ -f "$file" ] || fail "the package has no $file"
  archs=$(lipo -archs "$file" | tr ' ' '\n' | sort | paste -sd' ' -)
  [ "$archs" = "arm64 x86_64" ] || fail "$(basename "$file") holds [$archs], not arm64 and x86_64"
done

[ "$(read_plist CFBundleIdentifier)" = com.laminapdf.desktop ] || fail "the bundle identifier isn't com.laminapdf.desktop"
[ "$(read_plist CFBundleShortVersionString)" = "$VERSION" ] || fail "CFBundleShortVersionString isn't $VERSION"
[ "$(read_plist CFBundleVersion)" = "$BUILD" ] || fail "CFBundleVersion isn't $BUILD"
[ "$(read_plist LSMinimumSystemVersion)" = 13.0 ] || fail "LSMinimumSystemVersion isn't 13.0"
[ "$(read_plist LSApplicationCategoryType)" = public.app-category.productivity ] || fail "the category isn't Productivity"
test -s "$APP/Contents/Resources/THIRD_PARTY_NOTICES.txt" || fail "the notices are missing from the bundle"

# The entitlement keys of a signed file, sorted, one per line (empty for a file with none).
keys_of() {
  codesign -d --entitlements :- "$1" 2> /dev/null | python3 -I -c '
import plistlib, sys
data = sys.stdin.buffer.read()
print("\n".join(sorted(plistlib.loads(data))) if data.strip() else "", end="")'
}
keys_in() {
  python3 -I -c '
import plistlib, sys
print("\n".join(sorted(plistlib.load(open(sys.argv[1], "rb")))), end="")' "$1"
}
expected_app=$(keys_in "$ENTITLEMENTS/app.entitlements")
if [ "$SIGNED" = true ]; then
  expected_app=$(printf '%s\n%s\n' "$expected_app" "com.apple.application-identifier
com.apple.developer.team-identifier" | sort)
fi
[ "$(keys_of "$APP")" = "$expected_app" ] || fail "the app's entitlements differ from app-store/app.entitlements:
$(keys_of "$APP")"
[ "$(keys_of "$engine")" = "$(keys_in "$ENTITLEMENTS/engine.entitlements")" ] || fail "the engine's entitlements differ from app-store/engine.entitlements:
$(keys_of "$engine")"
[ -z "$(keys_of "$lib")" ] || fail "PDFium's library carries entitlements"

# The updater's configuration: its public key (the base64 of minisign's "untrusted comment" line,
# which starts a key), the endpoint it named and its Windows install mode, all of which
# `plugins.updater: null` in tauri.appstore.conf.json takes out of the config the app is built
# with. The strings go to a file: `strings | grep -q` under pipefail fails when grep stops early.
strings_file="$(mktemp)"
strings -a "$exe" > "$strings_file"
for needle in dW50cnVzdGVkIGNvbW1lbnQ6 '/beta/{{target}}' installMode; do
  if grep -q -F -- "$needle" "$strings_file"; then
    fail "the app still holds '$needle' from the updater's configuration (ADR 0014 §11)"
  fi
done
rm -f "$strings_file"

if [ "$SIGNED" = true ]; then
  details=$(codesign -dvv "$APP" 2>&1)
  authority=$(sed -n 's/^Authority=//p' <<< "$details" | head -n 1)
  case "$authority" in
    "Apple Distribution:"* | "3rd Party Mac Developer Application:"*) ;;
    *) fail "the app is signed by '$authority', not an Apple Distribution certificate" ;;
  esac
  [ -s "$APP/Contents/embedded.provisionprofile" ] || fail "the provisioning profile isn't in the bundle"
  signature=$(pkgutil --check-signature "$PKG" 2>&1 || true)
  echo "$signature"
  grep -q -E 'Mac Installer Distribution|3rd Party Mac Developer Installer' <<< "$signature" \
    || fail "the package isn't signed with the Mac Installer Distribution certificate"
else
  [ ! -e "$APP/Contents/embedded.provisionprofile" ] || fail "an ad-hoc package holds a provisioning profile"
  signature=$(pkgutil --check-signature "$PKG" 2>&1 || true)
  grep -q 'no signature' <<< "$signature" || fail "an ad-hoc package is signed"
fi

payload="$(mktemp)"
pkgutil --payload-files "$PKG" > "$payload"
grep -q 'Contents/MacOS/laminapdf-desktop-engine' "$payload" || fail "the package doesn't hold the engine"
rm -f "$payload"
echo "The Mac App Store package $VERSION ($BUILD) passes."
