#!/bin/sh
# Packs the local @lamina/wasm and @lamina/pdf builds (unpublished) into vendor/, which
# package.json depends on by file: tarball. Run `cargo xtask wasm` at the repository root first.
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
root=$(cd "$here/../.." && pwd)
mkdir -p "$here/vendor"
test -f "$root/packages/wasm/dist/lamina_base.wasm" || { echo "run cargo xtask wasm first" >&2; exit 1; }
(cd "$root/packages/wasm" && pnpm pack --pack-destination "$here/vendor" >/dev/null)
(cd "$root/packages/pdf" && pnpm pack --pack-destination "$here/vendor" >/dev/null)
ls "$here/vendor"
