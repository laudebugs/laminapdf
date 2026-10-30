#!/usr/bin/env bash
# Every Android .so must have its LOAD segments aligned to 16 KB (0x4000), which Google Play
# requires for apps targeting Android 15+ on 16 KB-page devices. Prints one line per library.
set -euo pipefail
cd "$(dirname "$0")/.."
readelf=$(ls "${ANDROID_NDK_HOME:?}"/toolchains/llvm/prebuilt/*/bin/llvm-readelf | head -1)
status=0
while IFS= read -r so; do
  aligns=$("$readelf" -lW "$so" | awk '$1 == "LOAD" { print $NF }' | sort -u | tr '\n' ' ')
  if [[ "$aligns" == "0x4000 " ]]; then echo "ok    $so  LOAD align $aligns"; else echo "FAIL  $so  LOAD align $aligns"; status=1; fi
done < <(find target/jniLibs target/jniLibs-notext -name '*.so' 2>/dev/null | sort)
exit $status
