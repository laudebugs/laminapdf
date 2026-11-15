#!/usr/bin/env bash
# The Breakpad symbols of the desktop app's own two binaries, the main process and the engine
# sidecar (todo D7d; docs/guidelines/desktop-release.md §7), in the layout the symbols bucket keeps:
#
#   <debug file>/<debug ID>/<debug file without .pdb>.sym   what rust-minidump's symbol suppliers read
#   <debug file>/<debug ID>/<raw debug file>                 beside it: the dSYM (tar.gz), the PDB or
#                                                            the ELF .debug file, to dump again later
#
# PDFium is a prebuilt third-party library built without our symbols, so it has none here.
#
#   symbols.sh <target triple> <out dir>
#
# On Linux the Tauri CLI runs it as the build's beforeBundleCommand, between compiling and
# bundling: it dumps each binary while its DWARF is still in it, moves the DWARF into a .debug file
# (objcopy --only-keep-debug) and strips the binary the bundles then carry. On macOS (dSYM,
# split-debuginfo = packed) and Windows (PDB) the debug information is already beside the binary,
# so a step after the build runs it.
set -euo pipefail

target=$1
out=$2
root=${GITHUB_WORKSPACE:?}
if command -v cygpath > /dev/null; then
  root=$(cygpath -u "$root")
  out=$(cygpath -u "$out")
fi
cd "$root"
mkdir -p "$out"
scratch=$(mktemp -d)

fail() {
  echo "::error::$1"
  exit 1
}

# Writes the .sym that dump_syms makes of "$@" into its place in $out, and sets $dir to that folder.
dump() {
  local sym="$scratch/dump.sym" id name
  dump_syms --inlines -o "$sym" "$@" > /dev/null
  read -r _ _ _ id name < "$sym"
  name=${name%$'\r'}
  # A module dumped without its debug information has no FUNC record: a stripped binary, or a
  # PE whose PDB dump_syms didn't find.
  grep -q '^FUNC ' "$sym" || fail "$name: dump_syms found no functions in $*"
  dir="$out/$name/$id"
  mkdir -p "$dir"
  mv "$sym" "$dir/${name%.pdb}.sym"
}

case "$RUNNER_OS" in
  macOS)
    for binary in "target/$target/release/laminapdf-desktop" target/release/laminapdf-desktop-engine; do
      [ -d "$binary.dSYM" ] || fail "no $binary.dSYM: the release build kept no debug information"
      # The binary and its dSYM together: the binary's unwind tables, the dSYM's functions and lines,
      # under the binary's name (the dSYM alone would be named <binary>.dSYM).
      dump "$binary" "$binary.dSYM"
      tar -chzf "$dir/$(basename "$binary").dSYM.tar.gz" -C "$(dirname "$binary")" "$(basename "$binary").dSYM"
    done
    ;;
  Windows)
    for binary in "target/$target/release/laminapdf-desktop.exe" target/release/laminapdf-desktop-engine.exe; do
      # dump_syms finds the PDB through the path the linker wrote into the executable.
      dump "$binary"
      pdb="$(dirname "$binary")/$(basename "${binary%.exe}" | tr - _).pdb"
      [ -f "$pdb" ] || fail "no $pdb beside $binary"
      cp "$pdb" "$dir/"
    done
    ;;
  Linux)
    # The engine the bundles carry is desktop-sidecar's copy, under Tauri's sidecar name; the build
    # also leaves one in the target folder. Each is the same file as the one dumped.
    sidecar="apps/desktop/src-tauri/binaries/laminapdf-desktop-engine-$target"
    for binary in "target/$target/release/laminapdf-desktop" target/release/laminapdf-desktop-engine; do
      dump "$binary"
      debug="$dir/$(basename "$binary").debug"
      objcopy --only-keep-debug "$binary" "$debug"
      shipped=("$binary")
      if [ "$(basename "$binary")" = laminapdf-desktop-engine ]; then
        shipped+=("$sidecar")
        if [ -f "target/$target/release/laminapdf-desktop-engine" ]; then
          shipped+=("target/$target/release/laminapdf-desktop-engine")
        fi
      fi
      for file in "${shipped[@]}"; do
        [ -f "$file" ] || fail "no $file"
        # What CARGO_PROFILE_RELEASE_STRIP=symbols did before, plus a link to the .debug file
        # (its name and CRC) that debuggers follow.
        objcopy --strip-all --add-gnu-debuglink="$debug" "$file"
        # DWARF proper: rustc also leaves .debug_gdb_scripts, an allocated section strip keeps, which
        # holds only the names of gdb's pretty-printer scripts.
        dwarf=$(readelf -SW "$file" | grep -oE '\.debug_(info|abbrev|line|line_str|str|str_offsets|ranges|rnglists|loc|loclists|aranges|addr|frame|types|macro|pubnames|pubtypes)\b' | sort -u | tr '\n' ' ' || true)
        [ -z "$dwarf" ] || fail "$file still carries DWARF: $dwarf"
      done
    done
    ;;
  *) fail "no symbols for $RUNNER_OS" ;;
esac

rm -rf "$scratch"
echo "Symbols in $out:"
find "$out" -type f | sort
