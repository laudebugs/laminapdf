"""Validates the files `crates/laminapdf/tests/signed_gating.rs` saves.

Run the test with `LAMINA_SIGNED_GATING_OUT=<dir>`, then this script on `<dir>`:

* `allowed/`: an op that broke nothing, saved incrementally on an approval-signed file. Each
  signature must still validate in pyHanko, intact (its `/ByteRange` covers the bytes it
  signed), with the later revision reported as a modification; `pdfsig` must call it valid.
* `overridden/`: an op saved through the override. pyHanko must find no signature, and
  `pdfsig` none that is broken.

Exits non-zero, listing each file that fails. Needs this folder's requirements.txt and
`pdfsig` (poppler-utils).
"""

from __future__ import annotations

import subprocess
import sys
import logging
from pathlib import Path

from pyhanko.pdf_utils.reader import PdfFileReader
from pyhanko.sign.diff_analysis import ModificationLevel
from pyhanko.sign.validation import validate_pdf_signature
from pyhanko_certvalidator import ValidationContext


def pdfsig(path: Path) -> str:
    return subprocess.run(["pdfsig", str(path)], capture_output=True, text=True).stdout


def allowed(path: Path) -> str | None:
    with path.open("rb") as f:
        sigs = PdfFileReader(f).embedded_signatures
        if not sigs:
            return "no signature left"
        for sig in sigs:
            # The fixtures' CA is thrown away after each run: trust and revocation can't be
            # checked, only the signature's integrity and what changed after it.
            status = validate_pdf_signature(sig, ValidationContext(allow_fetching=False))
            if not (status.intact and status.valid):
                return f"{sig.field_name}: not intact"
            if status.coverage.name != "ENTIRE_REVISION":
                return f"{sig.field_name}: covers {status.coverage.name}"
            if status.modification_level is None or status.modification_level == ModificationLevel.NONE:
                return f"{sig.field_name}: no later revision reported"
    out = pdfsig(path)
    if "Signature is Valid." not in out or "Signature is Invalid" in out:
        return "pdfsig: " + " ".join(l.strip() for l in out.splitlines() if "Validation" in l)
    return None


def overridden(path: Path) -> str | None:
    with path.open("rb") as f:
        if PdfFileReader(f).embedded_signatures:
            return "a signature is left"
    # pdfsig still lists the signature field, which the override keeps unsigned.
    out = pdfsig(path)
    if "Signature Validation" in out:
        return "pdfsig: a signature is left"
    return None


def main() -> int:
    # pyHanko logs each page-tree change it can't explain as a traceback; the verdict is what
    # the check reads (the modification level).
    logging.disable(logging.ERROR)
    root = Path(sys.argv[1])
    failed = []
    counts = {}
    for kind, check in [("allowed", allowed), ("overridden", overridden)]:
        files = sorted((root / kind).glob("*.pdf"))
        counts[kind] = len(files)
        for path in files:
            if (why := check(path)) is not None:
                failed.append(f"{kind}/{path.name}: {why}")
    print(f"allowed: {counts['allowed']} files, overridden: {counts['overridden']} files, failed: {len(failed)}")
    for line in failed:
        print(line)
    return 1 if failed or not all(counts.values()) else 0


if __name__ == "__main__":
    sys.exit(main())
