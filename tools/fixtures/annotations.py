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
    same-name.pdf      two filled squares with the same /NM, each with its appearance (red at
                       the left, blue at the right), and a third square that is already Hidden:
                       a render that leaves one out must name it by its place in /Annots
    rectless-first.pdf the red and blue squares after a note with no /Rect: an annotation without
                       a rectangle still takes a place in /Annots
    tagged.pdf         a tagged page: a square that is the only kid of an Annot element, a note
                       that shares its element with marked content, and a line whose OBJR is an
                       indirect object
    review.pdf         a review (todo E5b): a highlight over text with two replies (one answering
                       the other) and four states set on it (review states as strings, as names,
                       and dated out of document order; a marked state), its Popup, a second note
                       whose state has no /StateModel, a reply to a missing object, a ring of two
                       replies, a strike-out grouped with a caret, a reply to the highlight's
                       Popup, and on page 2 a reply to the highlight; its text is set in Noto
                       Sans, embedded (todo E5f's summary draws the page under PDF/UA-1)
    flatten.pdf        flattening (todo E5e), two pages of a tagged file with optional content:
                       a translucent square, a stamp whose /AS picks one of two states, a hidden
                       square, a print-only one, a screen-only one, one in a layer that is off,
                       a square without an appearance, a sound comment without one, a widget,
                       a note with its Popup and a reply, an appearance turned by its /Matrix,
                       and on page 2 a square tagged as an Annot element beside marked text
    flatten-cases.pdf  flattening's edge cases: squares in a layer by membership dictionaries (one
                       shown, one hidden), one that multiplies (/BM), one whose appearance has no
                       /Subtype, a locked one, one with /CA and /ca, a note whose reply has no
                       appearance, a square whose reply is locked, and on a page turned a quarter
                       a NoRotate square
    stamps.pdf         stamps (todo E5c): the fourteen standard names, each with an appearance,
                       another app's own name (SBSignHere), one with no /Name (read as Draft), an
                       image stamp named as Acrobat names one ("#" and a hash) whose appearance
                       draws a 2 x 2 image, and one turned a quarter (/Rotate 90)
    attachments.pdf    file attachments (todo E5c): one per icon (PushPin, Paperclip, Graph, Tag),
                       each holding a small text file with its MIME type and size; the Graph one
                       shares its file with the /EmbeddedFiles tree
    sound.pdf          sound comments (todo E5c): a 440 Hz tone, 0.05 s at 8000 samples a second,
                       as raw 8-bit, signed 16-bit stereo, mu-law, A-law, raw 4-bit, signed
                       16-bit Flate-compressed, and samples with /CO /DPCM that can't be played

    python3 tools/fixtures/annotations.py
"""

import math
import struct
import zlib
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


def same_name():
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        **page(3, "[10 0 R 11 0 R 12 0 R]"),
        10: "<< /Type /Annot /Subtype /Square /Rect [20 100 120 200] /NM (twin) /C [1 0 0] /IC [1 0 0] "
        "/F 4 /AP << /N 30 0 R >> >>",
        11: "<< /Type /Annot /Subtype /Square /Rect [180 100 280 200] /NM (twin) /C [0 0 1] /IC [0 0 1] "
        "/F 4 /AP << /N 31 0 R >> >>",
        12: "<< /Type /Annot /Subtype /Square /Rect [100 20 200 80] /NM (hidden) /C [0 1 0] /IC [0 1 0] "
        "/F 6 /AP << /N 32 0 R >> >>",
    }
    form = "/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >>"
    streams = {
        30: (form, b"1 0 0 rg 0 0 100 100 re f"),
        31: (form, b"0 0 1 rg 0 0 100 100 re f"),
        32: ("/Type /XObject /Subtype /Form /BBox [0 0 100 60] /Resources << >>", b"0 1 0 rg 0 0 100 60 re f"),
    }
    write("same-name.pdf", objects, streams)


def rectless_first():
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        **page(3, "[9 0 R 10 0 R 11 0 R]"),
        9: "<< /Type /Annot /Subtype /Text /Contents (No rectangle) /F 4 >>",
        10: "<< /Type /Annot /Subtype /Square /Rect [20 100 120 200] /C [1 0 0] /IC [1 0 0] /F 4 /AP << /N 30 0 R >> >>",
        11: "<< /Type /Annot /Subtype /Square /Rect [180 100 280 200] /C [0 0 1] /IC [0 0 1] /F 4 /AP << /N 31 0 R >> >>",
    }
    form = "/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >>"
    streams = {30: (form, b"1 0 0 rg 0 0 100 100 re f"), 31: (form, b"0 0 1 rg 0 0 100 100 re f")}
    write("rectless-first.pdf", objects, streams)


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


def truetype(path):
    """A simple TrueType font dictionary's entries for characters 32 to 126 of the font at path,
    WinAnsi-encoded, and its descriptor's: the widths from hmtx through the (3, 1) cmap, the box
    and vertical metrics from head, hhea and OS/2 (ISO 32000-2 §9.6.3, §9.8)."""
    data = path.read_bytes()
    count = struct.unpack(">H", data[4:6])[0]
    tables = {}
    for i in range(count):
        tag, _, offset, length = struct.unpack(">4sIII", data[12 + 16 * i : 28 + 16 * i])
        tables[tag.decode()] = data[offset : offset + length]
    head, hhea, hmtx, os2 = tables["head"], tables["hhea"], tables["hmtx"], tables["OS/2"]
    upem = struct.unpack(">H", head[18:20])[0]
    box = struct.unpack(">4h", head[36:44])
    ascent, descent = struct.unpack(">2h", hhea[4:8])
    metrics = struct.unpack(">H", hhea[34:36])[0]
    cap = struct.unpack(">h", os2[88:90])[0]

    def advance(gid):
        return struct.unpack(">H", hmtx[4 * min(gid, metrics - 1) : 4 * min(gid, metrics - 1) + 2])[0]

    cmap = tables["cmap"]
    glyphs = {}
    for i in range(struct.unpack(">H", cmap[2:4])[0]):
        platform, encoding, offset = struct.unpack(">HHI", cmap[4 + 8 * i : 12 + 8 * i])
        if (platform, encoding) != (3, 1) or struct.unpack(">H", cmap[offset : offset + 2])[0] != 4:
            continue
        sub = cmap[offset:]
        segs = struct.unpack(">H", sub[6:8])[0] // 2
        ends = struct.unpack(f">{segs}H", sub[14 : 14 + 2 * segs])
        starts = struct.unpack(f">{segs}H", sub[16 + 2 * segs : 16 + 4 * segs])
        deltas = struct.unpack(f">{segs}h", sub[16 + 4 * segs : 16 + 6 * segs])
        ranges_at = 16 + 6 * segs
        ranges = struct.unpack(f">{segs}H", sub[ranges_at : ranges_at + 2 * segs])
        for code in range(32, 127):
            for k in range(segs):
                if starts[k] <= code <= ends[k]:
                    if ranges[k] == 0:
                        glyphs[code] = (code + deltas[k]) & 0xFFFF
                    else:
                        at = ranges_at + 2 * k + ranges[k] + 2 * (code - starts[k])
                        gid = struct.unpack(">H", sub[at : at + 2])[0]
                        glyphs[code] = (gid + deltas[k]) & 0xFFFF if gid else 0
                    break
    scale = 1000 / upem
    widths = " ".join(str(round(advance(glyphs.get(c, 0)) * scale)) for c in range(32, 127))
    font = f"/FirstChar 32 /LastChar 126 /Widths [{widths}] /Encoding /WinAnsiEncoding"
    descriptor = (
        f"/FontBBox [{' '.join(str(round(v * scale)) for v in box)}] /ItalicAngle 0 "
        f"/Ascent {round(ascent * scale)} /Descent {round(descent * scale)} /CapHeight {round(cap * scale)} /StemV 80"
    )
    return font, descriptor, data


def review():
    # The page's font is embedded, so a comment summary, which is PDF/UA-1, can draw the page.
    entries, descriptor, program = truetype(OUT.parent / "fonts" / "NotoSans-Regular.ttf")
    font = (
        "<< /Font << /F1 << /Type /Font /Subtype /TrueType /BaseFont /NotoSans-Regular "
        f"{entries} /FontDescriptor 41 0 R >> >> >>"
    )
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
        41: f"<< /Type /FontDescriptor /FontName /NotoSans-Regular /Flags 32 {descriptor} /FontFile2 42 0 R >>",
    }
    streams = {
        40: (
            "",
            b"BT /F1 12 Tf 20 250 Td (Quoted words) Tj ET BT /F1 12 Tf 20 100 Td (Struck text) Tj ET",
        ),
        42: (f"/Length1 {len(program)}", program),
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
        17: "<< /Type /Annot /Subtype /Sound /Rect [230 150 250 170] /F 4 /Contents (Recorded) >>",
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


def stamps():
    # ISO 32000-2's fourteen names, then another app's own name, no name, an image stamp as
    # Acrobat names one (a "#" and a hash) and one turned a quarter.
    names = [
        "Approved", "Experimental", "NotApproved", "AsIs", "Expired", "NotForPublicRelease",
        "Confidential", "Final", "Sold", "Departmental", "ForComment", "TopSecret", "Draft",
        "ForPublicRelease",
    ]
    objects = {2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"}
    streams = {}
    refs = []
    for i, name in enumerate(names):
        x, y = 10 + 145 * (i % 2), 280 - 18 * (i // 2)
        objects[10 + i] = (
            f"<< /Type /Annot /Subtype /Stamp /Name /{name} /Rect [{x} {y - 15} {x + 140} {y}] /F 4 "
            f"/Contents ({name}) /AP << /N 30 0 R >> >>"
        )
        refs.append(f"{10 + i} 0 R")
    objects[40] = "<< /Type /Annot /Subtype /Stamp /Name /SBSignHere /Rect [10 20 70 40] /F 4 /AP << /N 31 0 R >> >>"
    objects[41] = "<< /Type /Annot /Subtype /Stamp /Rect [80 20 140 40] /F 4 >>"
    objects[42] = (
        "<< /Type /Annot /Subtype /Stamp /Name /#239A3F1C2B /Rect [150 10 190 50] /F 4 /AP << /N 32 0 R >> >>"
    )
    objects[43] = "<< /Type /Annot /Subtype /Stamp /Name /Final /Rotate 90 /Rect [200 10 230 90] /F 4 /AP << /N 34 0 R >> >>"
    refs += ["40 0 R", "41 0 R", "42 0 R", "43 0 R"]
    objects.update(page(3, "[" + " ".join(refs) + "]"))
    streams = {
        30: form("0 0.5 0", 140, 15),
        31: form("1 0.5 0", 60, 20),
        32: (
            "/Type /XObject /Subtype /Form /BBox [0 0 40 40] /Resources << /XObject << /Im0 33 0 R >> >>",
            b"q 40 0 0 40 0 0 cm /Im0 Do Q",
        ),
        33: (
            "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8",
            bytes([255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0]),
        ),
        34: form("0 0.5 0", 30, 80),
    }
    write("stamps.pdf", objects, streams)


def attachments():
    icons = ["PushPin", "Paperclip", "Graph", "Tag"]
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        **page(3, "[" + " ".join(f"{10 + i} 0 R" for i in range(4)) + "]"),
    }
    streams = {}
    for i, icon in enumerate(icons):
        x = 20 + 60 * i
        name = f"{icon.lower()}.txt"
        data = f"The {icon} file.\n".encode()
        objects[10 + i] = (
            f"<< /Type /Annot /Subtype /FileAttachment /Name /{icon} /Rect [{x} 260 {x + 20} 280] /F 28 "
            f"/FS {20 + i} 0 R /Contents ({name}) >>"
        )
        objects[20 + i] = (
            f"<< /Type /Filespec /F ({name}) /UF ({name}) /Desc (File {i + 1}) /EF << /F {30 + i} 0 R >> >>"
        )
        streams[30 + i] = (f"/Type /EmbeddedFile /Subtype /text#2Fplain /Params << /Size {len(data)} >>", data)
    objects[40] = "<< /Names [(graph.txt) 22 0 R] >>"
    write("attachments.pdf", objects, streams, "/Names << /EmbeddedFiles 40 0 R >> ")


def mu_law(sample):
    """ITU-T G.711 mu-law of a 16-bit sample."""
    sign = 0x80 if sample < 0 else 0
    magnitude = min(abs(sample), 32635) + 0x84
    exponent = max(0, magnitude.bit_length() - 8)
    mantissa = (magnitude >> (exponent + 3)) & 0x0F
    return ~(sign | (exponent << 4) | mantissa) & 0xFF


def a_law(sample):
    """ITU-T G.711 A-law of a 16-bit sample."""
    sign = 0x80 if sample >= 0 else 0
    magnitude = min(abs(sample), 32767) >> 3
    if magnitude < 32:
        code = magnitude >> 1
    else:
        exponent = magnitude.bit_length() - 5
        code = (exponent << 4) | ((magnitude >> exponent) & 0x0F)
    return (sign | code) ^ 0x55


def sound():
    rate, count = 8000, 400
    tone = [round(12000 * math.sin(2 * math.pi * 440 * i / rate)) for i in range(count)]
    raw8 = bytes((s >> 8) + 128 for s in tone)
    stereo16 = b"".join(struct.pack(">hh", s, -s) for s in tone)
    raw4 = bytes((((tone[i] >> 12) + 8) << 4) | ((tone[i + 1] >> 12) + 8) for i in range(0, count, 2))
    signed16 = b"".join(struct.pack(">h", s) for s in tone)
    sounds = [
        ("/R 8000 /B 8 /E /Raw", raw8),
        ("/R 8000 /C 2 /B 16 /E /Signed", stereo16),
        ("/R 8000 /B 8 /E /muLaw", bytes(mu_law(s) for s in tone)),
        ("/R 8000 /B 8 /E /ALaw", bytes(a_law(s) for s in tone)),
        ("/R 8000 /B 4", raw4),
        ("/R 8000 /B 16 /E /Signed /Filter /FlateDecode", zlib.compress(signed16, 9)),
        ("/R 8000 /B 8 /CO /DPCM", raw8),
    ]
    objects = {
        2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        **page(3, "[" + " ".join(f"{10 + i} 0 R" for i in range(len(sounds))) + "]"),
    }
    streams = {}
    for i, (entries, data) in enumerate(sounds):
        x = 20 + 36 * i
        objects[10 + i] = (
            f"<< /Type /Annot /Subtype /Sound /Name /Speaker /Rect [{x} 260 {x + 20} 280] /F 4 "
            f"/Sound {30 + i} 0 R /Contents (Sound {i + 1}) >>"
        )
        streams[30 + i] = (f"/Type /Sound {entries}", data)
    write("sound.pdf", objects, streams)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    every_subtype()
    threads()
    flags()
    same_name()
    rectless_first()
    tagged()
    review()
    flatten()
    flatten_cases()
    stamps()
    attachments()
    sound()


if __name__ == "__main__":
    main()
