#!/usr/bin/env bash
# Signs the Mac App Store app for the store and wraps it in a .pkg (todo D8c; ADR 0014 §11).
#
# Tauri built the app ad hoc, so every piece of code is signed again here, inside out: PDFium's
# library with no entitlements, the engine with the sandbox it inherits, then the app with the
# store's entitlements. The store build leaves out the hardened runtime: Apple's review replaces
# notarisation, and the store accepts App Sandbox alone.
#
# With a real identity the app also gets the provisioning profile and the two identifiers the
# profile grants, which the store checks. With none (IDENTITY=-, the pull request's check) the
# same steps run ad hoc, so the entitlements and the layout are proved without a secret.
#
# Reads: APP (the .app), PKG (the .pkg to write), ENTITLEMENTS (the app-store/ folder),
# IDENTITY (a certificate's hash, or -), INSTALLER (the installer certificate's hash, or empty),
# PROFILE (the .provisionprofile, or empty), APP_ID and TEAM (from the profile; empty ad hoc).
set -euo pipefail

: "${APP:?}" "${PKG:?}" "${ENTITLEMENTS:?}" "${IDENTITY:?}"
engine="$APP/Contents/MacOS/laminapdf-desktop-engine"
timestamp=(--timestamp)
if [ "$IDENTITY" = - ]; then timestamp=(--timestamp=none); fi

# The profile sits in the bundle before the app is sealed: the seal covers it.
if [ -n "${PROFILE:-}" ]; then
  cp "$PROFILE" "$APP/Contents/embedded.provisionprofile"
fi

# The app's entitlements, and with a profile the identifiers it names: the store matches them
# against the profile.
app_entitlements="$RUNNER_TEMP/app-store-app.entitlements"
cp "$ENTITLEMENTS/app.entitlements" "$app_entitlements"
if [ -n "${APP_ID:-}" ]; then
  /usr/libexec/PlistBuddy -c "Add :com.apple.application-identifier string $APP_ID" "$app_entitlements"
  /usr/libexec/PlistBuddy -c "Add :com.apple.developer.team-identifier string $TEAM" "$app_entitlements"
fi

for lib in "$APP"/Contents/Resources/pdfium/*.dylib; do
  codesign --force --sign "$IDENTITY" "${timestamp[@]}" "$lib"
done
codesign --force --sign "$IDENTITY" "${timestamp[@]}" --identifier com.laminapdf.desktop.engine \
  --entitlements "$ENTITLEMENTS/engine.entitlements" "$engine"
codesign --force --sign "$IDENTITY" "${timestamp[@]}" --entitlements "$app_entitlements" "$APP"
codesign --verify --strict --deep --verbose=2 "$APP"

# The installer certificate signs the package; without one the package is unsigned, which is
# enough for the check that follows.
sign=()
if [ -n "${INSTALLER:-}" ]; then sign=(--sign "$INSTALLER"); fi
rm -f "$PKG"
productbuild --component "$APP" /Applications ${sign[@]+"${sign[@]}"} "$PKG"
ls -l "$PKG"
