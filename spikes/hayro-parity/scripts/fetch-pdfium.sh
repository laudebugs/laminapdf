#!/usr/bin/env bash
# Download a pinned PDFium native build (bblanchon/pdfium-binaries, chromium/7881)
# into ./pdfium and verify its SHA-256. pdfium-render 0.9.4 (feature pdfium_latest
# = pdfium_7881) binds it dynamically at run time.
set -euo pipefail
cd "$(dirname "$0")/.."

TAG="chromium/7881"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) ASSET=pdfium-mac-arm64.tgz; SHA=52e94ca5aa8847934330daf3f8150c190682c5ca93831468794f8b90d4392e40 ;;
  Darwin-x86_64) ASSET=pdfium-mac-x64.tgz; SHA=6dedf83990e0e3d6b7c93c9e7589c5a126b0ae14b7464d76120cff7a26afb18b ;;
  Linux-x86_64) ASSET=pdfium-linux-x64.tgz; SHA=1470e21b8b4a3b4ad7f85684e2da11d94f3b69a86d81dee11b9b6709d927ac1d ;;
  *) echo "unsupported host $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

mkdir -p pdfium
TGZ="pdfium/$ASSET"
verify() { echo "$SHA  $1" | shasum -a 256 -c - >/dev/null 2>&1; }
# A cached archive that doesn't match (e.g. a partial download) is discarded.
if [ -f "$TGZ" ] && ! verify "$TGZ"; then
  echo "discarding $TGZ: SHA-256 mismatch" >&2
  rm -f "$TGZ"
fi
if [ ! -f "$TGZ" ]; then
  # Download to a temporary name and only keep it once the hash matches.
  TMP="$TGZ.part"
  rm -f "$TMP"
  curl -fsSL -o "$TMP" "https://github.com/bblanchon/pdfium-binaries/releases/download/$TAG/$ASSET"
  if ! verify "$TMP"; then
    rm -f "$TMP"
    echo "SHA-256 mismatch for $ASSET; expected $SHA" >&2
    exit 1
  fi
  mv "$TMP" "$TGZ"
fi
echo "$SHA  $TGZ" | shasum -a 256 -c -
# Extract only the library and the licence files; nothing from the archive is executed.
tar -xzf "$TGZ" -C pdfium lib licenses LICENSE VERSION
ls pdfium/lib
