#!/usr/bin/env python3
"""Writes fixtures/corpus/outline.pdf (no dependencies): three pages with an outline, named
destinations, page labels and an Info dictionary, for lamina-model's views (B2c).

The outline exercises: nesting (three levels), a closed item (negative /Count), titles in
PDFDocEncoding (with codes outside Latin-1) and in UTF-16BE (a surrogate pair), explicit
destinations (/XYZ, /Fit, /FitH), a named destination from the /Names /Dests tree (a string)
and from the catalog's /Dests dictionary (a name), and a GoTo action. Page labels: roman
numerals for the first page, then "A-1", "A-2". The Info dictionary has a valid
/CreationDate and an invalid /ModDate.

    python3 tools/fixtures/outline.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "corpus" / "outline.pdf"


def utf16(text):
    return "<FEFF" + text.encode("utf-16-be").hex().upper() + ">"


def page(num, text):
    return f"<< /Type /Page /Parent 2 0 R /Contents {num} 0 R /Resources << /Font << /F1 3 0 R >> >> >>", (
        f"BT /F1 24 Tf 72 720 Td ({text}) Tj ET"
    )


OBJECTS = {}
OBJECTS[1] = (
    "<< /Type /Catalog /Pages 2 0 R /Outlines 10 0 R /PageMode /UseOutlines"
    " /Names << /Dests 20 0 R >> /Dests << /chapter2 [5 0 R /Fit] >>"
    " /PageLabels << /Nums [0 << /S /r >> 1 << /S /D /P (A-) >>] >> >>"
)
OBJECTS[2] = "<< /Type /Pages /Kids [4 0 R 5 0 R 6 0 R] /Count 3 /MediaBox [0 0 612 792] >>"
OBJECTS[3] = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"
for i, (pnum, cnum) in enumerate([(4, 7), (5, 8), (6, 9)]):
    OBJECTS[pnum], content = page(cnum, f"Page {i + 1}")
    OBJECTS[cnum] = content  # a stream, written below
# Outline: 10 is the root; 11..16 the items.
OBJECTS[10] = "<< /Type /Outlines /First 11 0 R /Last 13 0 R /Count 5 >>"
OBJECTS[11] = "<< /Title (Introduction) /Parent 10 0 R /Next 12 0 R /Dest [4 0 R /XYZ 0 792 0] >>"
OBJECTS[12] = (
    "<< /Title (Chapter \\2251 \\204 Caf\\351) /Parent 10 0 R /Prev 11 0 R /Next 13 0 R"
    " /First 14 0 R /Last 15 0 R /Count 2 /Dest /chapter2 >>"
)
OBJECTS[13] = (
    f"<< /Title {utf16('Appendix 𝄞 Ω')} /Parent 10 0 R /Prev 12 0 R /First 16 0 R /Last 16 0 R"
    " /Count -1 /A << /S /GoTo /D (appendix) >> >>"
)
OBJECTS[14] = "<< /Title (Section 2.1) /Parent 12 0 R /Next 15 0 R /Dest [5 0 R /FitH 700] >>"
OBJECTS[15] = "<< /Title (Section 2.2) /Parent 12 0 R /Prev 14 0 R /A << /S /URI /URI (https://example.org/) >> >>"
OBJECTS[16] = "<< /Title (Notes) /Parent 13 0 R /Dest [6 0 R /Fit] >>"
OBJECTS[20] = "<< /Kids [21 0 R] >>"
OBJECTS[21] = "<< /Limits [(appendix) (appendix)] /Names [(appendix) << /D [6 0 R /XYZ null null null] >>] >>"
OBJECTS[22] = (
    "<< /Title (Outline fixture) /Author (lamina) /CreationDate (D:20261028120000Z)"
    " /ModDate (yesterday) /Producer (tools/fixtures/outline.py) >>"
)
STREAMS = {7, 8, 9}


def main():
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    size = max(OBJECTS) + 1
    for num in sorted(OBJECTS):
        offsets[num] = len(out)
        body = OBJECTS[num]
        if num in STREAMS:
            data = body.encode("latin-1")
            out += f"{num} 0 obj\n<< /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream\nendobj\n"
        else:
            out += f"{num} 0 obj\n{body}\nendobj\n".encode("latin-1")
    xref = len(out)
    out += f"xref\n0 {size}\n0000000000 65535 f\r\n".encode()
    for num in range(1, size):
        if num in offsets:
            out += f"{offsets[num]:010} 00000 n\r\n".encode()
        else:
            out += b"0000000000 65535 f\r\n"
    out += f"trailer\n<< /Size {size} /Root 1 0 R /Info 22 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    OUT.write_bytes(out)
    print(f"wrote {OUT} ({len(out)} bytes)")


if __name__ == "__main__":
    main()
