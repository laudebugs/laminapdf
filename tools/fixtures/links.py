#!/usr/bin/env python3
"""Writes fixtures/links/links.pdf (no dependencies): three pages whose links cover every kind
the web reader's link layer handles (C6b), each labelled on the page with what it is.

Page 1 holds, top to bottom: an explicit /XYZ destination, a named destination from the
/Names /Dests tree (a string) and one from the catalog's /Dests (a name, through a GoTo
action), URIs (https with /Contents, mailto, an international host written in UTF-8, and the
schemes a reader must refuse: javascript:, file:, data:), GoToR to another file, the named
actions NextPage and LastPage, Launch, JavaScript and SubmitForm actions, a link with
/QuadPoints, and a destination naming a page that isn't in the file. It also has two links a
reader leaves out: one hidden (/F 2) and one without a usable /Rect. Page 2 has a FirstPage
link and is the named destination's target; page 3 has a PrevPage link. fixtures/links/
expected.json holds pypdf's view of the links (tools/fixtures/link_facts.py).

    python3 tools/fixtures/links.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "links" / "links.pdf"

# Objects: 1 catalog, 2 pages, 3 font, 4..6 pages, 7..9 their contents, 10 the /Dests tree,
# 20 onwards page 1's links, 60 onwards the other pages' links.
PAGES = (4, 5, 6)
CONTENTS = (7, 8, 9)

# Page 1's links: (label drawn beside it, annotation entries after /Rect).
LINKS = [
    ("Explicit destination: page 3", "/Dest [6 0 R /XYZ 72 700 0]"),
    ("Named destination (string): page 2", "/Dest (chapter)"),
    ("GoTo a named destination (name): page 3", "/A << /S /GoTo /D /appendix >>"),
    ("https, with /Contents", "/Contents (The project site) /A << /S /URI /URI (https://example.com/a?b=c#d) >>"),
    ("mailto", "/A << /S /URI /URI (mailto:someone@example.com) >>"),
    ("International host", "/A << /S /URI /URI (https://b\\303\\274cher.example/) >>"),
    ("javascript: (refused)", "/A << /S /URI /URI (javascript:alert\\(1\\)) >>"),
    ("file: (refused)", "/A << /S /URI /URI (file:///etc/passwd) >>"),
    ("data: (refused)", "/A << /S /URI /URI (data:text/html,hi) >>"),
    ("GoToR: another file", "/A << /S /GoToR /F << /Type /Filespec /F (budget-2026.pdf) /UF (budget-2026.pdf) >> /D [0 /Fit] >>"),
    ("Named action: NextPage", "/A << /S /Named /N /NextPage >>"),
    ("Named action: LastPage", "/A << /S /Named /N /LastPage >>"),
    ("Launch (never run)", "/A << /S /Launch /F (calc.exe) >>"),
    ("JavaScript (never run)", "/A << /S /JavaScript /JS (app.alert\\(1\\)) >>"),
    ("SubmitForm (never run)", "/A << /S /SubmitForm /F << /FS /URL /F (https://example.com/) >> >>"),
    ("QuadPoints: page 2", "QUADS /Dest [5 0 R /Fit]"),
    ("Broken destination", "/Dest [99 0 R /Fit]"),
]
TOP, STEP, HEIGHT = 740, 36, 18


def link_rect(i):
    y = TOP - i * STEP
    return 72, y - 4, 400, y + HEIGHT - 4


def page1():
    lines = []
    annots = []
    for i, (label, entries) in enumerate(LINKS):
        x0, y0, x1, y1 = link_rect(i)
        lines.append(f"BT /F1 12 Tf 76 {y0 + 5} Td ({label}) Tj ET")
        if entries.startswith("QUADS"):
            # Two areas inside the rectangle, corners in the order Acrobat writes them.
            mid = (x0 + x1) // 2
            quads = f"[{x0} {y1} {mid} {y1} {x0} {y0} {mid} {y0} {mid} {y1} {x1} {y1} {mid} {y0} {x1} {y0}]"
            entries = f"/QuadPoints {quads}" + entries[len("QUADS"):]
        annots.append(f"<< /Type /Annot /Subtype /Link /Rect [{x0} {y0} {x1} {y1}] /Border [0 0 1] {entries} >>")
    # Two a reader leaves out.
    annots.append("<< /Type /Annot /Subtype /Link /Rect [72 80 400 98] /F 2 /Dest [5 0 R /Fit] >>")
    annots.append("<< /Type /Annot /Subtype /Link /Rect [72 80] /Dest [5 0 R /Fit] >>")
    return "\n".join(lines), annots


OBJECTS = {}
OBJECTS[1] = "<< /Type /Catalog /Pages 2 0 R /Names << /Dests 10 0 R >> /Dests << /appendix [6 0 R /Fit] >> >>"
OBJECTS[2] = "<< /Type /Pages /Kids [4 0 R 5 0 R 6 0 R] /Count 3 /MediaBox [0 0 612 792] >>"
OBJECTS[3] = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"
OBJECTS[10] = "<< /Names [(chapter) [5 0 R /FitH 600]] >>"
STREAMS = set(CONTENTS)


def add_page(page, contents, text, annots, first):
    refs = []
    for offset, annot in enumerate(annots):
        OBJECTS[first + offset] = annot
        refs.append(f"{first + offset} 0 R")
    OBJECTS[page] = (
        f"<< /Type /Page /Parent 2 0 R /Contents {contents} 0 R /Resources << /Font << /F1 3 0 R >> >>"
        f" /Annots [{' '.join(refs)}] >>"
    )
    OBJECTS[contents] = text


def build():
    text, annots = page1()
    add_page(4, 7, text, annots, 20)
    add_page(5, 8, "BT /F1 24 Tf 72 720 Td (Page 2: chapter) Tj ET\nBT /F1 12 Tf 76 641 Td (Named action: FirstPage) Tj ET",
             ["<< /Type /Annot /Subtype /Link /Rect [72 636 400 654] /A << /S /Named /N /FirstPage >> >>"], 60)
    add_page(6, 9, "BT /F1 24 Tf 72 720 Td (Page 3: appendix) Tj ET\nBT /F1 12 Tf 76 641 Td (Named action: PrevPage) Tj ET",
             ["<< /Type /Annot /Subtype /Link /Rect [72 636 400 654] /A << /S /Named /N /PrevPage >> >>"], 61)


def main():
    build()
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
    out += f"trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    OUT.parent.mkdir(exist_ok=True)
    OUT.write_bytes(out)
    print(f"wrote {OUT} ({len(out)} bytes)")


if __name__ == "__main__":
    main()
