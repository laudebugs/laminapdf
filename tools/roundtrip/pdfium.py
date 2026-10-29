#!/usr/bin/env python3
"""The PDFium half of the round-trip suite (todo B3c, ADR 0018 tier 7).

Opens every output listed in <out-dir>/manifest.json (written by lamina-save's `roundtrip`
example) with pypdfium2 and checks the page count, each page's rotation and the Info /Title
against it. `cargo xtask roundtrip` runs it; it prints one JSON line
({ tool, version, pdfium, checked, failures }) and exits 1 on any failure.

    python3 tools/roundtrip/pdfium.py <out-dir>
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pypdfium2 as pdfium
import pypdfium2.version as pdfium_version

PINNED = "5.14.0"
FIXTURES = Path(__file__).resolve().parent.parent.parent / "fixtures"


def title_of(pdf: pdfium.PdfDocument) -> str | None:
    return pdf.get_metadata_dict().get("Title") or None


def source_title(case: dict) -> str | None:
    """The title PDFium reads from the case's source fixture (None when it reads none, or the
    source is a variant built in memory)."""
    path = FIXTURES / case["source"]
    if not path.is_file():
        return None
    pdf = pdfium.PdfDocument(str(path))
    try:
        return title_of(pdf)
    finally:
        pdf.close()


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit("usage: pdfium.py <out-dir>")
    out = Path(sys.argv[1])
    version = pdfium_version.PYPDFIUM_INFO.version
    if version != PINNED:
        raise SystemExit(f"pypdfium2 {version}, expected {PINNED}")
    cases = json.loads((out / "manifest.json").read_text())["cases"]
    failures = []
    for case in cases:
        try:
            pdf = pdfium.PdfDocument(str(out / case["file"]))
            rotations = [pdf[i].get_rotation() for i in range(len(pdf))]
            title = title_of(pdf)
            want_title = case["title"]
            # PDFium reads no title from a direct /Info dictionary (text-example.pdf): when it
            # reads none from the source either, an unchanged title is not a difference.
            if title is None and want_title is not None and source_title(case) is None:
                want_title = None
            got = {"pages": len(pdf), "rotations": rotations, "title": title}
            want = {"pages": case["pages"], "rotations": case["rotations"], "title": want_title}
            if got != want:
                failures.append(f"{case['file']}: got {got}, expected {want}")
            pdf.close()
        except Exception as error:  # noqa: BLE001 - every failure is reported, none hidden
            failures.append(f"{case['file']}: {error}")
    print(
        json.dumps(
            {
                "tool": "pypdfium2",
                "version": version,
                "pdfium": str(pdfium_version.PDFIUM_INFO.version),
                "checked": len(cases),
                "failures": failures,
            }
        )
    )
    return 0 if not failures else 1


if __name__ == "__main__":
    sys.exit(main())
