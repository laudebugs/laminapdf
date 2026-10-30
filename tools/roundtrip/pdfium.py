#!/usr/bin/env python3
"""The PDFium half of the round-trip suite (todo B3c, ADR 0018 tier 7).

Opens every output listed in <out-dir>/manifest.json (written by lamina-save's `roundtrip`
example) with pypdfium2 and checks the page count, each page's rotation, the Info /Title
and whether the file is encrypted against it. `cargo xtask roundtrip` runs it; it prints one JSON line
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
def title_of(pdf: pdfium.PdfDocument) -> str | None:
    return pdf.get_metadata_dict().get("Title") or None


def info_of(pdf: pdfium.PdfDocument, keys: list[str]) -> dict[str, str | None]:
    metadata = pdf.get_metadata_dict()
    return {key: metadata.get(key) or None for key in keys}


def encrypted(pdf: pdfium.PdfDocument) -> bool:
    """FPDF_GetSecurityHandlerRevision is -1 for a document without encryption."""
    return pdfium.raw.FPDF_GetSecurityHandlerRevision(pdf) != -1


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
            # PDFium reads no title from a direct /Info dictionary (lamina records which
            # outputs have one): there, a missing title is not a difference.
            if case["direct_info"] and title is None:
                want_title = None
            # The setInfo cases also wrote /Author, /Subject and /Keywords (case["info"]);
            # None elsewhere. A direct /Info dictionary hides them from PDFium like the title.
            want_info = case["info"]
            info = None if want_info is None else info_of(pdf, list(want_info))
            if want_info is not None and case["direct_info"] and all(v is None for v in info.values()):
                want_info = info
            got = {
                "pages": len(pdf),
                "rotations": rotations,
                "title": title,
                "encrypted": encrypted(pdf),
                "info": info,
            }
            want = {
                "pages": case["pages"],
                "rotations": case["rotations"],
                "title": want_title,
                "encrypted": case["encrypted"],
                "info": want_info,
            }
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
