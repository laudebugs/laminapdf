#!/usr/bin/env python3
"""The PDFium half of the render goldens (todo B5c, ADR 0018 tier 8).

Renders every page of every PDF in <pdf-dir> with pypdfium2 at each DPI into
<out-dir>/<dpi>/<stem>/<page>.ppm (binary PPM, RGB, no dependencies beyond pypdfium2), for
`tools/render` (the `render-golden` binary) to compare. `cargo xtask render-golden` runs it;
it prints one JSON line ({ tool, version, pdfium, pages }) and exits 1 on any failure.

Settings match the hayro-parity spike (spikes/hayro-parity/README.md, "Method"): the page box
PDFium reports (CropBox with /Rotate), floor(points * dpi / 72) pixels per axis, a white
background, annotations on, anti-aliasing as shipped (no LCD text).

    python3 tools/render/pdfium.py <pdf-dir> <out-dir> <dpi>...
"""

from __future__ import annotations

import ctypes
import json
import math
import sys
from pathlib import Path

import pypdfium2 as pdfium
import pypdfium2.raw as raw
import pypdfium2.version as pdfium_version

PINNED = "5.14.0"


def render(page: pdfium.PdfPage, dpi: int) -> tuple[int, int, bytes]:
    scale = dpi / 72
    width = math.floor(page.get_width() * scale)
    height = math.floor(page.get_height() * scale)
    # BGR with reversed byte order is RGB in memory.
    bitmap = pdfium.PdfBitmap.new_native(width, height, raw.FPDFBitmap_BGR, rev_byteorder=True)
    bitmap.fill_rect((255, 255, 255, 255), 0, 0, width, height)
    flags = raw.FPDF_ANNOT | raw.FPDF_REVERSE_BYTE_ORDER
    raw.FPDF_RenderPageBitmap(bitmap, page, 0, 0, width, height, 0, flags)
    data = ctypes.string_at(bitmap.buffer, bitmap.stride * height)
    rows = (data[y * bitmap.stride : y * bitmap.stride + width * 3] for y in range(height))
    pixels = b"".join(rows)
    bitmap.close()
    return width, height, pixels


def main() -> int:
    if len(sys.argv) < 4:
        raise SystemExit("usage: pdfium.py <pdf-dir> <out-dir> <dpi>...")
    pdf_dir, out = Path(sys.argv[1]), Path(sys.argv[2])
    dpis = [int(d) for d in sys.argv[3:]]
    version = pdfium_version.PYPDFIUM_INFO.version
    if version != PINNED:
        raise SystemExit(f"pypdfium2 {version}, expected {PINNED} (tools/render/requirements.txt)")
    pages = 0
    for path in sorted(pdf_dir.glob("*.pdf")):
        pdf = pdfium.PdfDocument(str(path))
        for index in range(len(pdf)):
            page = pdf[index]
            for dpi in dpis:
                width, height, pixels = render(page, dpi)
                target = out / str(dpi) / path.stem / f"{index}.ppm"
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(b"P6\n%d %d\n255\n" % (width, height) + pixels)
            page.close()
            pages += 1
        pdf.close()
    print(
        json.dumps(
            {
                "tool": "pypdfium2",
                "version": version,
                "pdfium": str(pdfium_version.PDFIUM_INFO.version),
                "pages": pages,
            }
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
