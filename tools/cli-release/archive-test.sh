#!/usr/bin/env bash
# Checks a `laminapdf` release archive the way a user would install it (todo L4d;
# .github/workflows/cli-release.yml runs it after `dist build`):
#
#   tools/cli-release/archive-test.sh <laminapdf-cli-<target>.tar.xz or .zip> <fixture.pdf>
#
# Windows archives are .zip files holding laminapdf.exe; on Windows it runs in Git Bash.
# The checksum next to the archive matches; the archive holds the binary, both licences, the
# third-party notices, man pages and completions; the extracted binary runs `--version` and
# `info` on the fixture.
set -euo pipefail

archive=$1
fixture=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

expected=$(cut -d ' ' -f 1 "$archive.sha256")
if command -v sha256sum >/dev/null; then sum=(sha256sum); else sum=(shasum -a 256); fi
actual=$("${sum[@]}" "$archive" | cut -d " " -f 1)
[ "$expected" = "$actual" ] || { echo "checksum mismatch: $actual, expected $expected"; exit 1; }

case $archive in
  # Git Bash's tar is GNU tar, which reads no zip.
  *.zip) unzip -q "$archive" -d "$work"; binary=laminapdf.exe ;;
  *) tar -xf "$archive" -C "$work"; binary=laminapdf ;;
esac
# dist puts the files in one folder named after the archive, or at the top of a flat zip.
found=$(find "$work" -maxdepth 2 -type f -name "$binary" | head -n 1)
[ -n "$found" ] || { echo "$(basename "$archive") lacks $binary"; exit 1; }
dir=$(dirname "$found")
for file in "$binary" LICENSE-MIT LICENSE-APACHE THIRD_PARTY_NOTICES.md man/laminapdf.1 completions/laminapdf.bash; do
  [ -s "$dir/$file" ] || { echo "$(basename "$archive") lacks $file"; exit 1; }
done
"$dir/$binary" --version
"$dir/$binary" info "$fixture"
echo "archive-test: $(basename "$archive") passed"
