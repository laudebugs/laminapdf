#!/usr/bin/env bash
# Installs the npm packages tools/cli-release/generate.ts wrote and runs them, the way a user would
# (todo L4d; .github/workflows/cli-release.yml runs it after `dist build`):
#
#   tools/cli-release/install-test.sh <generate.ts's --out dir> <platform package folder> <fixture.pdf>
#
# 1. `npm pack` the wrapper and the platform package, install both tarballs into an empty
#    project and run `npx laminapdf --version` and `npx laminapdf info <fixture>`;
# 2. install the wrapper alone (`--omit=optional`): `laminapdf` must exit 1 naming the package;
# 3. install a copy of the wrapper whose `os` leaves out this machine's: npm must refuse it with
#    EBADPLATFORM, which lists the supported systems (what a user on another platform sees).
set -euo pipefail

out=$(cd "$1" && pwd)
platform=$2
fixture=$(cd "$(dirname "$3")" && pwd)/$(basename "$3")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

mkdir "$work/tarballs"
for dir in cli "$platform"; do
  (cd "$out/$dir" && npm pack --silent --pack-destination "$work/tarballs" >/dev/null)
done
wrapper=$(ls "$work"/tarballs/laminapdf-cli-[0-9]*.tgz)
binary=$(ls "$work"/tarballs/laminapdf-"$platform"-[0-9]*.tgz)

project() {
  mkdir "$work/$1"
  echo '{ "name": "install-test", "private": true }' > "$work/$1/package.json"
  cd "$work/$1"
}

echo "==> wrapper and $platform"
project full
npm install --no-audit --no-fund "$wrapper" "$binary"
npx --no-install laminapdf --version
npx --no-install laminapdf info "$fixture"

echo "==> wrapper without its optional dependencies"
project bare
npm install --no-audit --no-fund --omit=optional "$wrapper"
status=0
npx --no-install laminapdf --version 2> "$work/bare.err" || status=$?
cat "$work/bare.err"
[ "$status" -eq 1 ] || { echo "expected exit 1, got $status"; exit 1; }
grep -q "@laminapdf/$platform" "$work/bare.err" || { echo "the message doesn't name @laminapdf/$platform"; exit 1; }

echo "==> wrapper on an unsupported system"
project other
cp -R "$out/cli" "$work/other-wrapper"
node -e 'const fs = require("node:fs"); const f = process.argv[1]; const p = JSON.parse(fs.readFileSync(f, "utf8")); p.os = ["aix"]; fs.writeFileSync(f, JSON.stringify(p));' "$work/other-wrapper/package.json"
if npm install --no-audit --no-fund "$work/other-wrapper" 2> "$work/other.err"; then
  echo "npm installed a wrapper for another system"; exit 1
fi
grep -q EBADPLATFORM "$work/other.err" || { cat "$work/other.err"; exit 1; }
grep "Valid os" "$work/other.err"
echo "install-test: @laminapdf/cli with @laminapdf/$platform passed"
