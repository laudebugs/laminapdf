#!/usr/bin/env python3
"""The PDFium half of the round-trip suite (todo B3c, ADR 0018 tier 7).

Opens every output listed in <out-dir>/manifest.json (written by laminapdf-save's `roundtrip`
example) with pypdfium2 and checks the page count, each page's rotation, the Info /Title
and whether the file is encrypted against it. A copy case (todo E2p, `copy_of`) must also draw
its page as the page it was copied from draws. `cargo xtask roundtrip` runs it; it prints one JSON line
({ tool, version, pdfium, checked, failures }) and exits 1 on any failure.

    python3 tools/roundtrip/pdfium.py <out-dir>
"""

from __future__ import annotations

import ctypes
import json
import sys
from pathlib import Path

import pypdfium2 as pdfium
import pypdfium2.raw as c
import pypdfium2.version as pdfium_version

PINNED = "5.14.0"
# A copied page draws as its source page: at 72 dpi, at most this share of pixels may differ,
# and a pixel differs when a channel is more than CHANNEL_SLACK apart.
PIXEL_SHARE = 0.001
CHANNEL_SLACK = 8
def title_of(pdf: pdfium.PdfDocument) -> str | None:
    return pdf.get_metadata_dict().get("Title") or None


def info_of(pdf: pdfium.PdfDocument, keys: list[str]) -> dict[str, str | None]:
    metadata = pdf.get_metadata_dict()
    return {key: metadata.get(key) or None for key in keys}


def labels_of(pdf: pdfium.PdfDocument) -> list[str | None]:
    """Each page's label (FPDF_GetPageLabel: UTF-16LE with a terminating NUL), None for none."""
    labels = []
    for index in range(len(pdf)):
        size = pdfium.raw.FPDF_GetPageLabel(pdf, index, None, 0)
        if size <= 2:
            labels.append(None)
            continue
        buf = ctypes.create_string_buffer(size)
        pdfium.raw.FPDF_GetPageLabel(pdf, index, buf, size)
        labels.append(buf.raw[: size - 2].decode("utf-16-le"))
    return labels


def encrypted(pdf: pdfium.PdfDocument) -> bool:
    """FPDF_GetSecurityHandlerRevision is -1 for a document without encryption."""
    return pdfium.raw.FPDF_GetSecurityHandlerRevision(pdf) != -1


def pixels(page: pdfium.PdfPage) -> tuple[int, int, bytes]:
    bitmap = page.render(scale=1)
    rows = bytes(bitmap.buffer)
    row = bitmap.width * bitmap.n_channels
    packed = b"".join(rows[y * bitmap.stride : y * bitmap.stride + row] for y in range(bitmap.height))
    return bitmap.width, bitmap.height, packed


def drawn_apart(copy: pdfium.PdfDocument, copy_of: dict) -> str | None:
    """Why the copy's only page doesn't draw as the source page it came from, or None."""
    source = pdfium.PdfDocument(copy_of["path"], password=copy_of["password"])
    try:
        width, height, want = pixels(source[copy_of["page"]])
        got_width, got_height, got = pixels(copy[0])
    finally:
        source.close()
    if (got_width, got_height) != (width, height):
        return f"drawn {got_width}x{got_height}, the source page {width}x{height}"
    channels = len(want) // (width * height)
    apart = sum(
        1
        for i in range(0, len(want), channels)
        if max(abs(a - b) for a, b in zip(want[i : i + channels], got[i : i + channels])) > CHANNEL_SLACK
    )
    share = apart / (width * height)
    return None if share <= PIXEL_SHARE else f"{share:.2%} of its pixels differ from the source page"


def outline_of(pdf: pdfium.PdfDocument) -> list:
    """The outline as [depth, title, page index or None] per item in document order."""
    return [[item.level, item.get_title(), page_of(item)] for item in pdf.get_toc()]


def page_of(item) -> int | None:
    dest = item.get_dest()
    return None if dest is None else dest.get_index()


def links_of(pdf: pdfium.PdfDocument) -> list:
    """Each page's links, sorted: ["page", index], ["uri", address] or ["other"], as lamina lists them."""
    def target(link) -> list:
        dest = c.FPDFLink_GetDest(pdf.raw, link)
        action = c.FPDFLink_GetAction(link)
        if not dest and action and c.FPDFAction_GetType(action) == c.PDFACTION_GOTO:
            dest = c.FPDFAction_GetDest(pdf.raw, action)
        if dest:
            index = c.FPDFDest_GetDestPageIndex(pdf.raw, dest)
            return ["page", index] if index >= 0 else ["other"]
        if action and c.FPDFAction_GetType(action) == c.PDFACTION_URI:
            size = c.FPDFAction_GetURIPath(pdf.raw, action, None, 0)
            buffer = ctypes.create_string_buffer(size)
            c.FPDFAction_GetURIPath(pdf.raw, action, buffer, size)
            return ["uri", buffer.value.decode("ascii", "replace")]
        return ["other"]

    pages = []
    for i in range(len(pdf)):
        page = pdf[i]
        start = ctypes.c_int(0)
        link = c.FPDF_LINK()
        found = []
        while c.FPDFLink_Enumerate(page.raw, ctypes.byref(start), ctypes.byref(link)):
            found.append(target(link))
        pages.append(sorted(found, key=lambda x: json.dumps(x, separators=(",", ":"))))
    return pages


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
                # The setPageLabels cases list every page's label; None elsewhere.
                "labels": None if case.get("labels") is None else labels_of(pdf),
            }
            want = {
                "pages": case["pages"],
                "rotations": case["rotations"],
                "title": want_title,
                "encrypted": case["encrypted"],
                "info": want_info,
                "labels": case.get("labels"),
            }
            if case.get("outline") is not None:
                got["outline"] = outline_of(pdf)
                want["outline"] = case["outline"]
            if case.get("links") is not None:
                got["links"] = links_of(pdf)
                want["links"] = case["links"]
            if got != want:
                failures.append(f"{case['file']}: got {got}, expected {want}")
            elif case.get("copy_of") is not None:
                apart = drawn_apart(pdf, case["copy_of"])
                if apart is not None:
                    failures.append(f"{case['file']}: {apart}")
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
