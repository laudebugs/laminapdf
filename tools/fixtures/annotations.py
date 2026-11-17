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
    review.pdf         a review (todo E5b): a highlight over text with two replies (one answering
                       the other) and four states set on it (review states as strings, as names,
                       and dated out of document order; a marked state), its Popup, a second note
                       whose state has no /StateModel, a reply to a missing object, a ring of two
                       replies, a strike-out grouped with a caret, a reply to the highlight's
                       Popup, and on page 2 a reply to the highlight
    flatten.pdf        flattening (todo E5e), two pages of a tagged file with optional content:
                       a translucent square, a stamp whose /AS picks one of two states, a hidden
                       square, a print-only one, a screen-only one, one in a layer that is off,
                       a square without an appearance, a file attachment without one, a widget,
                       a note with its Popup and a reply, an appearance turned by its /Matrix,
                       and on page 2 a square tagged as an Annot element beside marked text
    flatten-cases.pdf  flattening's edge cases: squares in a layer by membership dictionaries (one
                       shown, one hidden), one that multiplies (/BM), one whose appearance has no
                       /Subtype, a locked one, one with /CA and /ca, a note whose reply has no
                       appearance, a square whose reply is locked, and on a page turned a quarter
                       a NoRotate square

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


def review():
    font = "<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>"
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        **page(
            3,
            "[10 0 R 23 0 R 11 0 R 12 0 R 13 0 R 14 0 R 15 0 R 16 0 R 24 0 R 17 0 R 18 0 R 19 0 R 20 0 R 21 0 R 25 0 R]",
            "/Contents 40 0 R ",
            font,
        ),
        **page(4, "[30 0 R]"),
        10: "<< /Type /Annot /Subtype /Highlight /Rect [18 246 98 264] /QuadPoints [18 264 98 264 18 246 98 246] "
        "/Contents (Check this) /T (Ada) /NM (c1) /CreationDate (D:20261101100000Z) /Popup 23 0 R /F 4 >>",
        23: "<< /Type /Annot /Subtype /Popup /Rect [120 200 220 260] /Parent 10 0 R >>",
        11: "<< /Type /Annot /Subtype /Text /Rect [18 246 98 264] /IRT 10 0 R /RT /R /Contents (Agreed) /T (Ben) "
        "/NM (r1) /CreationDate (D:20261101110000Z) /F 28 >>",
        12: "<< /Type /Annot /Subtype /Text /Rect [18 246 98 264] /IRT 11 0 R /Contents (Thanks) /T (Ada) "
        "/NM (r2) /CreationDate (D:20261101113000Z) /F 28 >>",
        13: "<< /Type /Annot /Subtype /Text /Rect [18 246 98 264] /IRT 10 0 R /StateModel (Review) /State (Accepted) "
        "/T (Ben) /NM (s1) /Contents (Accepted set by Ben) /CreationDate (D:20261102090000Z) /F 28 >>",
        14: "<< /Type /Annot /Subtype /Text /Rect [18 246 98 264] /IRT 10 0 R /StateModel /Review /State /Rejected "
        "/T (Cy) /NM (s2) /CreationDate (D:20261101120000Z) /F 28 >>",
        15: "<< /Type /Annot /Subtype /Text /Rect [18 246 98 264] /IRT 10 0 R /StateModel /Marked /State /Marked "
        "/T (Ada) /NM (s3) /F 28 >>",
        16: "<< /Type /Annot /Subtype /Text /Rect [200 260 220 280] /Contents (Second) /T (Ben) /NM (c2) /F 28 >>",
        24: "<< /Type /Annot /Subtype /Text /Rect [200 260 220 280] /IRT 16 0 R /State (Completed) /T (Ada) /F 28 >>",
        17: "<< /Type /Annot /Subtype /Text /Rect [200 200 220 220] /IRT 99 0 R /Contents (Orphan) /T (Cy) /F 28 >>",
        18: "<< /Type /Annot /Subtype /Text /Rect [200 150 220 170] /IRT 19 0 R /Contents (Ring A) /F 28 >>",
        19: "<< /Type /Annot /Subtype /Text /Rect [200 150 220 170] /IRT 18 0 R /Contents (Ring B) /F 28 >>",
        20: "<< /Type /Annot /Subtype /StrikeOut /Rect [18 96 98 114] /QuadPoints [18 114 98 114 18 96 98 96] "
        "/Contents (Struck) /T (Ben) /F 4 >>",
        21: "<< /Type /Annot /Subtype /Caret /Rect [96 96 104 114] /IRT 20 0 R /RT /Group /Contents (better) /F 4 >>",
        25: "<< /Type /Annot /Subtype /Text /Rect [200 100 220 120] /IRT 23 0 R /Contents (To the Popup) /F 28 >>",
        30: "<< /Type /Annot /Subtype /Text /Rect [20 260 40 280] /IRT 10 0 R /Contents (From page 2) /T (Cy) /F 28 >>",
    }
    streams = {
        40: (
            "",
            b"BT /F1 12 Tf 20 250 Td (Quoted words) Tj ET BT /F1 12 Tf 20 100 Td (Struck text) Tj ET",
        )
    }
    write("review.pdf", objects, streams)


def form(color, w, h, extra=""):
    return (
        f"/Type /XObject /Subtype /Form /BBox [0 0 {w} {h}] /Resources << >> {extra}",
        f"{color} rg 0 0 {w} {h} re f".encode(),
    )


def flatten():
    font = "<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>"
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        **page(
            3,
            "[10 0 R 11 0 R 12 0 R 13 0 R 14 0 R 15 0 R 16 0 R 17 0 R 18 0 R 19 0 R 20 0 R 21 0 R 22 0 R]",
            "/Contents 80 0 R ",
            font,
        ),
        **page(4, "[60 0 R]", "/Contents 81 0 R /StructParents 0 /Tabs /S ", font),
        10: "<< /Type /Annot /Subtype /Square /Rect [20 220 70 270] /CA 0.5 /F 4 /AP << /N 32 0 R >> >>",
        11: "<< /Type /Annot /Subtype /Stamp /Rect [90 220 140 270] /F 4 /AS /On "
        "/AP << /N << /On 33 0 R /Off 34 0 R >> >> >>",
        12: "<< /Type /Annot /Subtype /Square /Rect [160 220 210 270] /F 6 /AP << /N 35 0 R >> >>",
        13: "<< /Type /Annot /Subtype /Square /Rect [230 220 280 270] /F 36 /AP << /N 36 0 R >> >>",
        14: "<< /Type /Annot /Subtype /Square /Rect [20 150 70 200] /AP << /N 37 0 R >> >>",
        15: "<< /Type /Annot /Subtype /Square /Rect [90 150 140 200] /F 4 /OC 50 0 R /AP << /N 38 0 R >> >>",
        16: "<< /Type /Annot /Subtype /Square /Rect [160 150 210 200] /F 4 /C [0 0 1] >>",
        17: "<< /Type /Annot /Subtype /FileAttachment /Rect [230 150 250 170] /F 4 /Contents (Attached) >>",
        18: "<< /Type /Annot /Subtype /Widget /Rect [20 80 120 100] /FT /Tx /T (name) /F 4 /AP << /N 39 0 R >> >>",
        19: "<< /Type /Annot /Subtype /Text /Rect [150 80 170 100] /Contents (Note) /Popup 20 0 R /F 28 "
        "/AP << /N 41 0 R >> >>",
        20: "<< /Type /Annot /Subtype /Popup /Rect [180 40 280 100] /Parent 19 0 R >>",
        # A reply draws nothing, as Lamina PDF's replies don't.
        21: "<< /Type /Annot /Subtype /Text /Rect [150 80 170 100] /IRT 19 0 R /Contents (Reply) /F 28 "
        "/AP << /N 43 0 R >> >>",
        22: "<< /Type /Annot /Subtype /Square /Rect [200 110 280 140] /F 4 /AP << /N 42 0 R >> >>",
        50: "<< /Type /OCG /Name (Hidden layer) >>",
        60: "<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] /F 4 /StructParent 1 /Contents (Tagged square) "
        "/AP << /N 61 0 R >> >>",
        70: "<< /Type /StructTreeRoot /K 71 0 R /ParentTree 75 0 R /ParentTreeNextKey 2 >>",
        71: "<< /Type /StructElem /S /Document /P 70 0 R /K [72 0 R 73 0 R] >>",
        72: "<< /Type /StructElem /S /P /P 71 0 R /Pg 4 0 R /K 0 >>",
        73: "<< /Type /StructElem /S /Annot /P 71 0 R /Pg 4 0 R /K << /Type /OBJR /Obj 60 0 R >> >>",
        75: "<< /Nums [0 [72 0 R] 1 73 0 R] >>",
    }
    streams = {
        32: form("1 0 0", 50, 50),
        33: form("0 0.6 0", 50, 50),
        34: form("0 0 1", 50, 50),
        35: form("0.5 0.5 0.5", 50, 50),
        36: form("1 0.5 0", 50, 50),
        37: form("0.5 0 0.5", 50, 50),
        38: form("0 1 1", 50, 50),
        39: form("0.9 0.9 0.9", 100, 20),
        41: form("1 0.9 0", 20, 20),
        # Turned a quarter: the 30 x 80 box fills the 80 x 30 rectangle, magenta on the right.
        42: (
            "/Type /XObject /Subtype /Form /BBox [0 0 30 80] /Matrix [0 1 -1 0 0 0] /Resources << >>",
            b"1 0 1 rg 0 0 30 40 re f 0 1 1 rg 0 40 30 40 re f",
        ),
        43: ("/Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >>", b""),
        61: form("0 0.5 0", 60, 60),
        80: ("", b"BT /F1 12 Tf 20 280 Td (Flatten me) Tj ET"),
        81: ("", b"/P << /MCID 0 >> BDC BT /F1 12 Tf 20 250 Td (Tagged text) Tj ET EMC"),
    }
    write(
        "flatten.pdf",
        objects,
        streams,
        "/MarkInfo << /Marked true >> /StructTreeRoot 70 0 R "
        "/OCProperties << /OCGs [50 0 R] /D << /OFF [50 0 R] >> >> ",
    )


def flatten_cases():
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        **page(3, "[10 0 R 11 0 R 12 0 R 13 0 R 14 0 R 15 0 R 16 0 R 17 0 R 18 0 R 19 0 R]"),
        **page(4, "[20 0 R]", "/Rotate 90 "),
        10: "<< /Type /Annot /Subtype /Square /Rect [20 220 70 270] /F 4 /OC 50 0 R /AP << /N 30 0 R >> >>",
        11: "<< /Type /Annot /Subtype /Square /Rect [90 220 140 270] /F 4 /OC 52 0 R /AP << /N 31 0 R >> >>",
        12: "<< /Type /Annot /Subtype /Square /Rect [160 220 210 270] /F 4 /BM /Multiply /AP << /N 32 0 R >> >>",
        13: "<< /Type /Annot /Subtype /Square /Rect [230 220 280 270] /F 4 /AP << /N 33 0 R >> >>",
        14: "<< /Type /Annot /Subtype /Square /Rect [20 150 70 200] /F 132 /AP << /N 34 0 R >> >>",
        15: "<< /Type /Annot /Subtype /Text /Rect [90 180 110 200] /F 4 /Contents (Note) /AP << /N 35 0 R >> >>",
        16: "<< /Type /Annot /Subtype /Text /Rect [90 180 110 200] /F 4 /IRT 15 0 R /Contents (Reply) >>",
        17: "<< /Type /Annot /Subtype /Square /Rect [160 150 210 200] /F 4 /CA 0.5 /ca 0.25 /AP << /N 36 0 R >> >>",
        18: "<< /Type /Annot /Subtype /Square /Rect [230 150 280 200] /F 4 /AP << /N 37 0 R >> >>",
        19: "<< /Type /Annot /Subtype /Text /Rect [230 150 250 170] /F 132 /IRT 18 0 R /Contents (Locked reply) >>",
        20: "<< /Type /Annot /Subtype /Square /Rect [20 20 70 70] /F 20 /AP << /N 38 0 R >> >>",
        50: "<< /Type /OCMD /OCGs 51 0 R /P /AllOff >>",
        51: "<< /Type /OCG /Name (Off layer) >>",
        52: "<< /Type /OCMD /OCGs [51 0 R] /P /AnyOn >>",
    }
    streams = {
        30: form("1 0 0", 50, 50),
        31: form("0 1 0", 50, 50),
        32: form("0 0 1", 50, 50),
        33: ("/BBox [0 0 50 50] /Resources << >>", b"1 1 0 rg 0 0 50 50 re f"),
        34: form("0.5 0.5 0.5", 50, 50),
        35: form("1 0.9 0", 20, 20),
        36: form("1 0 1", 50, 50),
        37: form("0 1 1", 50, 50),
        38: form("1 0.5 0", 50, 50),
    }
    write("flatten-cases.pdf", objects, streams, "/OCProperties << /OCGs [51 0 R] /D << /OFF [51 0 R] >> >> ")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    every_subtype()
    threads()
    flags()
    tagged()
    review()
    flatten()
    flatten_cases()


if __name__ == "__main__":
    main()
