#!/usr/bin/env python3
"""Writes fixtures/revisions/hybrid.pdf (no dependencies): a hybrid-reference file (ISO 32000-2
§7.5.8.4). Its classic cross-reference table lists the page objects and leaves out object 5, a
font kept in an object stream; the trailer's /XRefStm names a cross-reference stream that lists
object 5 inside object stream 6, so a reader that knows cross-reference streams finds it and
one that doesn't sees a page without its font.

    python3 tools/fixtures/hybrid_xref.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "revisions" / "hybrid.pdf"

CONTENT = b"BT /F1 24 Tf 40 100 Td (Hybrid) Tj ET"
FONT = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"


def main():
    out = bytearray(b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}

    def obj(num, body):
        offsets[num] = len(out)
        out.extend(b"%d 0 obj\n" % num + body + b"\nendobj\n")

    obj(1, b"<< /Type /Catalog /Pages 2 0 R >>")
    obj(2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
    obj(3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] "
           b"/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>")
    obj(4, b"<< /Length %d >>\nstream\n" % len(CONTENT) + CONTENT + b"\nendstream")
    header = b"5 0 "
    data = header + FONT
    obj(6, b"<< /Type /ObjStm /N 1 /First %d /Length %d >>\nstream\n" % (len(header), len(data)) + data
           + b"\nendstream")
    # The cross-reference stream: object 5 is the first (index 0) of object stream 6.
    rows = bytes([2, 0, 6, 0])
    obj(7, b"<< /Type /XRef /Size 8 /Index [5 1] /W [1 2 1] /Length %d >>\nstream\n" % len(rows) + rows
           + b"\nendstream")
    xref = len(out)
    # Two subsections, objects 0-4 and 6-7: object 5 is only in the cross-reference stream.
    out.extend(b"xref\n0 5\n0000000000 65535 f\r\n")
    for num in range(1, 5):
        out.extend(b"%010d 00000 n\r\n" % offsets[num])
    out.extend(b"6 2\n")
    for num in (6, 7):
        out.extend(b"%010d 00000 n\r\n" % offsets[num])
    out.extend(b"trailer\n<< /Size 8 /Root 1 0 R /XRefStm %d >>\nstartxref\n%d\n%%%%EOF\n" % (offsets[7], xref))
    OUT.write_bytes(out)


if __name__ == "__main__":
    main()
