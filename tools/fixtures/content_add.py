#!/usr/bin/env python3
"""Writes fixtures/content/add/ (no dependencies): the pages and payloads the content ops that add
objects to pages are tested on (E6 plan §8, todo E6b). Every page is US Letter unless it says.

    rotated.pdf    a page shown turned 90 degrees, its MediaBox off the origin and its /UserUnit 2
    unbalanced.pdf a page whose content ends with a q left open after a scale
    tagged.pdf     a tagged page: a Document element holding a P that owns MCID 0
    footer.pdf     a running footer ("Page 3 of 24") in the bottom 10 per cent, for the overlap check
    layers.pdf     a file with optional content already: one group, on, around a square
    logo.pdf       a one-page vector logo, 120 x 60 points, its words in Helvetica (not embedded)
    logo-alpha.png a 64 x 32 RGBA PNG: an opaque blue bar fading to clear

    python3 tools/fixtures/content_add.py
"""

import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "content" / "add"

HELVETICA = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"
TEXT = b"BT /F1 18 Tf 72 700 Td (Quarterly report) Tj ET 0.8 g 72 600 468 60 re f"


def write(name, objects, streams):
    """objects: {num: body}; streams: {num: bytes or (dictionary entries, bytes)}. Object 1 is
    the catalog."""
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    size = max(list(objects) + list(streams)) + 1
    for num in range(1, size):
        if num in streams:
            offsets[num] = len(out)
            extra, data = streams[num] if isinstance(streams[num], tuple) else ("", streams[num])
            out += f"{num} 0 obj\n<< {extra}/Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream\nendobj\n"
        elif num in objects:
            offsets[num] = len(out)
            out += f"{num} 0 obj\n{objects[num]}\nendobj\n".encode("latin-1")
    xref = len(out)
    out += f"xref\n0 {size}\n0000000000 65535 f\r\n".encode()
    for num in range(1, size):
        if num in offsets:
            out += f"{offsets[num]:010} 00000 n\r\n".encode()
        else:
            out += b"0000000000 65535 f\r\n"
    out += f"trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    (OUT / name).write_bytes(bytes(out))


def one_page(
    name,
    content,
    page="",
    catalog="",
    resources="<< /Font << /F1 4 0 R >> >>",
    extra=None,
    media="[0 0 612 792]",
):
    """One page drawing `content`; `page` and `catalog` add entries, `extra` more objects."""
    objects = {
        1: f"<< /Type /Catalog /Pages 2 0 R {catalog}>>",
        2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        3: f"<< /Type /Page /Parent 2 0 R /MediaBox {media} {page}/Contents 10 0 R /Resources {resources} >>",
        4: HELVETICA,
    }
    objects.update(extra or {})
    write(name, objects, {10: content})


def png(width, height, pixels):
    """An RGBA PNG; `pixels(x, y)` gives each pixel's (r, g, b, a)."""

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    rows = b"".join(b"\x00" + b"".join(bytes(pixels(x, y)) for x in range(width)) for y in range(height))
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    one_page(
        "rotated.pdf",
        TEXT.replace(b"72 700", b"172 750"),
        page="/Rotate 90 /UserUnit 2 ",
        media="[100 50 712 842]",
    )
    one_page("unbalanced.pdf", b"q 1 0 0 rg 72 600 200 100 re f q 2 0 0 2 0 0 cm 0 0 1 rg 36 150 50 50 re f")
    one_page(
        "tagged.pdf",
        b"/P <</MCID 0>> BDC " + TEXT.split(b" 0.8 g")[0] + b" EMC /Artifact BMC 0.8 g 72 600 468 60 re f EMC",
        page="/StructParents 0 /Tabs /S ",
        catalog="/MarkInfo << /Marked true >> /StructTreeRoot 5 0 R /Lang (en) ",
        extra={
            5: "<< /Type /StructTreeRoot /K 6 0 R /ParentTree 8 0 R /ParentTreeNextKey 1 >>",
            6: "<< /Type /StructElem /S /Document /P 5 0 R /K [7 0 R] >>",
            7: "<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K 0 >>",
            8: "<< /Nums [0 [7 0 R]] >>",
        },
    )
    one_page("footer.pdf", TEXT + b" BT /F1 9 Tf 270 36 Td (Page 3 of 24) Tj ET")
    one_page(
        "layers.pdf",
        TEXT + b" /OC /L1 BDC 0 0.6 0 rg 400 400 100 100 re f EMC",
        catalog="/OCProperties << /OCGs [5 0 R] /D << /Order [5 0 R] >> >> ",
        resources="<< /Font << /F1 4 0 R >> /Properties << /L1 5 0 R >> >>",
        extra={5: "<< /Type /OCG /Name (Notes) >>"},
    )
    write(
        "logo.pdf",
        {
            1: "<< /Type /Catalog /Pages 2 0 R >>",
            2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            3: "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 120 60] /Contents 10 0 R /Resources << /Font << /F1 4 0 R >> >> >>",
            4: HELVETICA,
        },
        {
            10: b"0.11 0.32 0.62 rg 30 30 m 30 46.6 16.6 60 0 60 c 0 0 l 16.6 0 30 13.4 30 30 c f "
            b"0.95 0.55 0.1 rg 6 18 18 24 re f BT /F1 16 Tf 0.11 0.32 0.62 rg 38 23 Td (Northwind) Tj ET"
        },
    )
    (OUT / "logo-alpha.png").write_bytes(png(64, 32, lambda x, y: (20, 80, 200, 255 - x * 4)))


if __name__ == "__main__":
    main()
