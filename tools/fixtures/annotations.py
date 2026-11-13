#!/usr/bin/env python3
"""Writes fixtures/annotations/*.pdf (no dependencies): small pages of annotations for
laminapdf-model's annotation views and the annotation ops (todo E4b). Every page is 300 x 300
points.

    every-subtype.pdf  one page with a highlight (quads in the standard's order, with an
                       appearance), an underline (quads in Acrobat's order, no appearance), a note
                       with its Popup, ink, a line, a polygon, a FreeText callout with rich text,
                       a widget, and a direct annotation dictionary in /Annots
    threads.pdf        two pages: a note with its Popup, a reply and a reply to the reply, an
                       /IRT cycle, a strike-out grouped with a caret, and on page 2 a reply to
                       the note on page 1; /Annots of page 2 is an indirect array
    flags.pdf          a Locked square, a LockedContents note, a square with private keys, and
                       two notes with the same /NM
    tagged.pdf         a tagged page: a square that is the only kid of an Annot element, a note
                       that shares its element with marked content, and a line whose OBJR is an
                       indirect object

    python3 tools/fixtures/annotations.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "annotations"

MEDIA = "/MediaBox [0 0 300 300]"


def write(name, objects, streams=None, catalog_extra=""):
    """objects: {num: body}; streams: {num: (dictionary entries, bytes)}. Object 1 is the
    catalog, object 2 the page tree."""
    streams = streams or {}
    objects = {1: f"<< /Type /Catalog /Pages 2 0 R {catalog_extra}>>", **objects}
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    size = max(list(objects) + list(streams)) + 1
    for num in range(1, size):
        if num in streams:
            offsets[num] = len(out)
            extra, data = streams[num]
            out += f"{num} 0 obj\n<< {extra} /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream\nendobj\n"
        elif num in objects:
            offsets[num] = len(out)
            out += f"{num} 0 obj\n{objects[num]}\nendobj\n".encode("latin-1")
    xref = len(out)
    out += f"xref\n0 {size}\n0000000000 65535 f\r\n".encode()
    for num in range(1, size):
        out += f"{offsets[num]:010} 00000 n\r\n".encode() if num in offsets else b"0000000000 65535 f\r\n"
    out += f"trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    (OUT / name).write_bytes(bytes(out))


def page(num, annots, extra="", resources="<< >>"):
    # Every page has /Resources (ISO 32000-2 §7.7.3.3), empty unless its content uses some, so
    # qpdf --check passes.
    return {num: f"<< /Type /Page /Parent 2 0 R {MEDIA} /Resources {resources} /Annots {annots} {extra}>>"}


def every_subtype():
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        **page(
            3,
            "[10 0 R 11 0 R 12 0 R 13 0 R 14 0 R 15 0 R 16 0 R 17 0 R 18 0 R "
            "<< /Type /Annot /Subtype /Square /Rect [200 200 220 220] >>]",
        ),
        10: "<< /Type /Annot /Subtype /Highlight /Rect [20 260 120 275] /NM (hl-1) /C [1 1 0] /CA 0.5 "
        "/QuadPoints [20 260 120 260 120 275 20 275] /T (Ada) /Subj (Highlight) "
        "/CreationDate (D:20261101090000Z) /M (D:20261102090000Z) /F 4 /AP << /N 30 0 R >> >>",
        11: "<< /Type /Annot /Subtype /Underline /Rect [20 240 120 255] /C [0 0 1] "
        "/QuadPoints [20 255 120 255 20 240 120 240] /F 4 >>",
        12: "<< /Type /Annot /Subtype /Text /Rect [150 260 170 280] /Contents (Check this) /T (Ada) "
        "/Name /Comment /Popup 13 0 R /F 28 >>",
        13: "<< /Type /Annot /Subtype /Popup /Rect [180 200 280 260] /Parent 12 0 R /Open false >>",
        14: "<< /Type /Annot /Subtype /Ink /Rect [20 150 120 200] /InkList [[20 150 70 200 120 150] [30 160 40 170]] "
        "/BS << /W 2 >> /C [1 0 0] >>",
        15: "<< /Type /Annot /Subtype /Line /Rect [20 100 120 140] /L [20 100 120 140] /LE [/None /OpenArrow] >>",
        16: "<< /Type /Annot /Subtype /Polygon /Rect [150 100 250 160] /Vertices [150 100 250 100 200 160] /IC [0 1 0] >>",
        17: "<< /Type /Annot /Subtype /FreeText /Rect [150 20 280 80] /Contents (Callout) /DA (/Helv 10 Tf 0 g) "
        "/IT /FreeTextCallout /CL [140 90 160 70 170 50] "
        "/RC (<?xml version=\"1.0\"?><body xmlns=\"http://www.w3.org/1999/xhtml\"><p><b>Callout</b></p></body>) >>",
        18: "<< /Type /Annot /Subtype /Widget /Rect [20 20 120 40] /FT /Tx /T (name) >>",
    }
    streams = {30: ("/Type /XObject /Subtype /Form /BBox [0 0 100 15] /Resources << >>", b"1 1 0 rg 0 0 100 15 re f")}
    write("every-subtype.pdf", objects, streams)


def threads():
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        **page(3, "[10 0 R 11 0 R 12 0 R 13 0 R 14 0 R 15 0 R 16 0 R 17 0 R]"),
        **page(4, "5 0 R"),
        5: "[20 0 R]",
        10: "<< /Type /Annot /Subtype /Text /Rect [20 260 40 280] /Contents (Parent) /T (Ada) /NM (parent) /Popup 11 0 R >>",
        11: "<< /Type /Annot /Subtype /Popup /Rect [50 200 150 260] /Parent 10 0 R >>",
        12: "<< /Type /Annot /Subtype /Text /Rect [20 260 40 280] /Contents (Reply) /T (Ben) /IRT 10 0 R /NM (reply) >>",
        13: "<< /Type /Annot /Subtype /Text /Rect [20 260 40 280] /Contents (Reply to the reply) /T (Ada) /IRT 12 0 R /RT /R >>",
        14: "<< /Type /Annot /Subtype /Text /Rect [200 260 220 280] /Contents (Cycle A) /IRT 15 0 R >>",
        15: "<< /Type /Annot /Subtype /Text /Rect [200 260 220 280] /Contents (Cycle B) /IRT 14 0 R >>",
        16: "<< /Type /Annot /Subtype /StrikeOut /Rect [20 100 120 115] /QuadPoints [20 100 120 100 120 115 20 115] "
        "/Contents (better) >>",
        17: "<< /Type /Annot /Subtype /Caret /Rect [118 100 126 115] /IRT 16 0 R /RT /Group >>",
        20: "<< /Type /Annot /Subtype /Text /Rect [20 260 40 280] /Contents (Reply on page 2) /IRT 10 0 R >>",
    }
    write("threads.pdf", objects)


def flags():
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        **page(3, "[10 0 R 11 0 R 12 0 R 13 0 R 14 0 R]"),
        10: "<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] /F 132 /Contents (Locked) >>",
        11: "<< /Type /Annot /Subtype /Text /Rect [100 20 120 40] /F 516 /Contents (Fixed text) >>",
        12: "<< /Type /Annot /Subtype /Square /Rect [150 20 210 80] /LaminaTestKey (kept) "
        "/AAPL:Private << /Kept true >> /F 4 >>",
        13: "<< /Type /Annot /Subtype /Text /Rect [20 200 40 220] /NM (same) >>",
        14: "<< /Type /Annot /Subtype /Text /Rect [60 200 80 220] /NM (same) >>",
    }
    write("flags.pdf", objects)


def tagged():
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        **page(
            3,
            "[10 0 R 11 0 R 12 0 R]",
            "/StructParents 0 /Tabs /S /Contents 30 0 R ",
            "<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>",
        ),
        10: "<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] /StructParent 1 /Contents (Square) >>",
        11: "<< /Type /Annot /Subtype /Text /Rect [100 200 120 220] /StructParent 2 /Contents (Note) >>",
        12: "<< /Type /Annot /Subtype /Line /Rect [150 150 250 250] /L [150 150 250 250] /StructParent 3 /Contents (Line) >>",
        20: "<< /Type /StructTreeRoot /K 21 0 R /ParentTree 25 0 R /ParentTreeNextKey 4 >>",
        21: "<< /Type /StructElem /S /Document /P 20 0 R /K [22 0 R 23 0 R 24 0 R] >>",
        22: "<< /Type /StructElem /S /Annot /P 21 0 R /Pg 3 0 R /K << /Type /OBJR /Obj 10 0 R >> >>",
        23: "<< /Type /StructElem /S /P /P 21 0 R /Pg 3 0 R /K [0 << /Type /OBJR /Obj 11 0 R >>] >>",
        24: "<< /Type /StructElem /S /Annot /P 21 0 R /Pg 3 0 R /K [26 0 R] >>",
        25: "<< /Nums [0 [23 0 R] 1 22 0 R 2 23 0 R 3 24 0 R] >>",
        26: "<< /Type /OBJR /Obj 12 0 R >>",
    }
    streams = {30: ("", b"/P << /MCID 0 >> BDC BT /F1 12 Tf 20 250 Td (Text) Tj ET EMC")}
    write("tagged.pdf", objects, streams, "/MarkInfo << /Marked true >> /StructTreeRoot 20 0 R ")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    every_subtype()
    threads()
    flags()
    tagged()


if __name__ == "__main__":
    main()
