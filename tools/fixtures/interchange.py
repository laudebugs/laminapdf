#!/usr/bin/env python3
"""Writes fixtures/interchange/ (no dependencies): XFDF and FDF files for laminapdf-interchange's
readers and importer (todo E5d), each made for corpus/hello.pdf (one Letter page).

    review.xfdf          every kind an import makes: a highlight with its Popup, a reply to it
                         and a review status, a note, an underline, a strike-out grouped with a
                         caret, a text box, ink, a line with an arrow, a square, a circle, a
                         polygon, a polyline, a stamp and an attached file
    review.fdf           the same comments as FDF, its objects in another order than they are
                         listed and without a cross-reference table, as FDF files often are
    other-document.xfdf  one note, made for another PDF (its <f href> and <ids> differ)
    skipped.xfdf         what an import leaves out: a sound, a link, a comment for page 8, a
                         reply to a comment that isn't there, and two form fields
    malformed.xfdf       an element closed with the wrong name on line 5
    dtd.xfdf             a DTD declaring an entity on line 2, which must be refused

Run from the repository root: python3 tools/fixtures/interchange.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "interchange"

HEAD = '<?xml version="1.0" encoding="UTF-8"?>\n<xfdf xmlns="http://ns.adobe.com/xfdf/" xml:space="preserve">\n'
FOOT = '<f href="hello.pdf"/>\n</xfdf>\n'

REVIEW = """<annots>
<highlight page="0" rect="72,700,180,714" name="hl" title="Ada" subject="Highlight" color="#FFFF00" opacity="0.6" flags="print" creationdate="D:20261101100000Z" date="D:20261101100000Z" coords="72,714,180,714,72,700,180,700">
<contents>Check this figure</contents>
<popup open="no" page="0" rect="200,620,380,700"/>
</highlight>
<text page="0" rect="72,700,92,720" name="hl-reply" title="Ben" inreplyto="hl" replyType="reply" creationdate="D:20261101110000Z" date="D:20261101110000Z" icon="Comment"><contents>It matches the appendix</contents></text>
<text page="0" rect="72,700,92,720" name="hl-state" title="Ben" inreplyto="hl" state="Accepted" statemodel="Review" creationdate="D:20261101120000Z" date="D:20261101120000Z"><contents>Accepted set by Ben</contents></text>
<text page="0" rect="400,700,420,720" name="note" title="Ada" color="#FFD700" flags="print,nozoom,norotate" icon="Help" date="D:20261101100000Z"><contents>Why is this blank?</contents><popup open="yes" page="0" rect="430,620,590,720"/></text>
<underline page="0" rect="72,650,180,664" name="ul" color="#0000FF" flags="print" coords="72,664,180,664,72,650,180,650"/>
<strikeout page="0" rect="72,600,180,614" name="so" color="#FF0000" flags="print" coords="72,614,180,614,72,600,180,600"><contents>Replace with "net"</contents></strikeout>
<caret page="0" rect="176,596,186,606" name="so-caret" inreplyto="so" replyType="group" color="#FF0000" flags="print" symbol="none"/>
<freetext page="0" rect="300,500,460,540" name="box" title="Ada" flags="print" justification="left" width="1"><contents>Totals are in thousands</contents><defaultappearance>/Helv 12 Tf 0 g</defaultappearance></freetext>
<ink page="0" rect="100,400,200,460" name="ink" color="#0000FF" flags="print" width="2"><inklist><gesture>100,400;150,460;200,400</gesture></inklist></ink>
<line page="0" rect="90,340,210,380" name="line" color="#FF0000" flags="print" width="1" start="100,350" end="200,370" head="None" tail="OpenArrow"/>
<square page="0" rect="300,400,380,440" name="square" color="#FF0000" interior-color="#FFFFCC" flags="print" width="2"/>
<circle page="0" rect="400,400,480,440" name="circle" color="#00AA00" flags="print" width="1" style="dash" dashes="3,2"/>
<polygon page="0" rect="300,300,400,380" name="polygon" color="#0000FF" flags="print" width="1"><vertices>300,300,400,300,350,380</vertices></polygon>
<polyline page="0" rect="420,300,520,380" name="polyline" color="#0000FF" flags="print" width="1" head="None" tail="ClosedArrow"><vertices>420,300,470,380,520,300</vertices></polyline>
<stamp page="0" rect="72,200,252,250" name="stamp" title="Ada" flags="print" icon="Approved" color="#00AA00"/>
<fileattachment page="0" rect="40,40,60,60" name="file" title="Ada" flags="print" icon="Paperclip" file="readings.csv" mimetype="text/csv"><contents>The raw readings</contents><data MODE="raw" encoding="hex" length="25">6461792C72656164696E670A312C31322E350A322C31332E310A</data></fileattachment>
</annots>
"""

# The same comments as FDF dictionaries, by /NM: (subtype, entries written as PDF syntax,
# the comment it answers, its Popup, an embedded file).
FDF_COMMENTS = [
    ("Highlight", "/Rect [72 700 180 714] /T (Ada) /Subj (Highlight) /C [1 1 0] /CA 0.6 /F 4 "
     "/CreationDate (D:20261101100000Z) /M (D:20261101100000Z) "
     "/QuadPoints [72 714 180 714 72 700 180 700] /Contents (Check this figure)", "hl", None,
     ("[200 620 380 700]", "false"), None),
    ("Text", "/Rect [72 700 92 720] /T (Ben) /RT /R /CreationDate (D:20261101110000Z) "
     "/M (D:20261101110000Z) /Name /Comment /Contents (It matches the appendix)", "hl-reply", "hl", None, None),
    ("Text", "/Rect [72 700 92 720] /T (Ben) /State (Accepted) /StateModel (Review) "
     "/CreationDate (D:20261101120000Z) /M (D:20261101120000Z) /Contents (Accepted set by Ben)",
     "hl-state", "hl", None, None),
    ("Text", "/Rect [400 700 420 720] /T (Ada) /C [1 0.8431 0] /F 28 /Name /Help /M (D:20261101100000Z) "
     "/Contents (Why is this blank?)", "note", None, ("[430 620 590 720]", "true"), None),
    ("Underline", "/Rect [72 650 180 664] /C [0 0 1] /F 4 /QuadPoints [72 664 180 664 72 650 180 650]",
     "ul", None, None, None),
    ("StrikeOut", "/Rect [72 600 180 614] /C [1 0 0] /F 4 /QuadPoints [72 614 180 614 72 600 180 600] "
     "/Contents (Replace with \"net\")", "so", None, None, None),
    ("Caret", "/Rect [176 596 186 606] /C [1 0 0] /F 4 /RT /Group /Sy /None", "so-caret", "so", None, None),
    ("FreeText", "/Rect [300 500 460 540] /T (Ada) /F 4 /Q 0 /BS << /W 1 >> /Contents (Totals are in thousands) "
     "/DA (/Helv 12 Tf 0 g)", "box", None, None, None),
    ("Ink", "/Rect [100 400 200 460] /C [0 0 1] /F 4 /BS << /W 2 >> /InkList [[100 400 150 460 200 400]]",
     "ink", None, None, None),
    ("Line", "/Rect [90 340 210 380] /C [1 0 0] /F 4 /BS << /W 1 >> /L [100 350 200 370] /LE [/None /OpenArrow]",
     "line", None, None, None),
    ("Square", "/Rect [300 400 380 440] /C [1 0 0] /IC [1 1 0.8] /F 4 /BS << /W 2 >>", "square", None, None, None),
    ("Circle", "/Rect [400 400 480 440] /C [0 0.6667 0] /F 4 /BS << /W 1 /S /D /D [3 2] >>",
     "circle", None, None, None),
    ("Polygon", "/Rect [300 300 400 380] /C [0 0 1] /F 4 /BS << /W 1 >> /Vertices [300 300 400 300 350 380]",
     "polygon", None, None, None),
    ("PolyLine", "/Rect [420 300 520 380] /C [0 0 1] /F 4 /BS << /W 1 >> /Vertices [420 300 470 380 520 300] "
     "/LE [/None /ClosedArrow]", "polyline", None, None, None),
    ("Stamp", "/Rect [72 200 252 250] /T (Ada) /F 4 /Name /Approved /C [0 0.6667 0]", "stamp", None, None, None),
    ("FileAttachment", "/Rect [40 40 60 60] /T (Ada) /F 4 /Name /Paperclip /Contents (The raw readings)",
     "file", None, None, ("readings.csv", "text/csv", b"day,reading\n1,12.5\n2,13.1\n")),
]


def fdf() -> bytes:
    """review.fdf: each comment's object, its Popup and its file stream, numbered from 2, the
    catalog last, and no cross-reference table."""
    numbers = {name: 2 + i for i, (_, _, name, *_) in enumerate(FDF_COMMENTS)}
    extra = 2 + len(FDF_COMMENTS)
    objects = []
    for subtype, entries, name, parent, popup, file in FDF_COMMENTS:
        n = numbers[name]
        body = f"<< /Type /Annot /Subtype /{subtype} /Page 0 /NM ({name}) {entries}"
        if parent:
            body += f" /IRT {numbers[parent]} 0 R"
        if popup:
            rect, is_open = popup
            body += f" /Popup {extra} 0 R"
            objects.append((extra, f"<< /Type /Annot /Subtype /Popup /Page 0 /Rect {rect} /Open {is_open} "
                                   f"/Parent {n} 0 R >>".encode()))
            extra += 1
        if file:
            fname, mime, data = file
            body += (f" /FS << /Type /Filespec /F ({fname}) /UF ({fname}) /EF << /F {extra} 0 R >> >>")
            mime_name = mime.replace("/", "#2F")
            objects.append((extra, f"<< /Type /EmbeddedFile /Subtype /{mime_name} /Length {len(data)} >>\nstream\n".encode()
                            + data + b"\nendstream"))
            extra += 1
        objects.append((n, (body + " >>").encode()))
    refs = " ".join(f"{numbers[name]} 0 R" for _, _, name, *_ in FDF_COMMENTS)
    objects.append((1, f"<< /FDF << /F (hello.pdf) /Annots [{refs}] >> >>".encode()))
    # Written in reverse, so a reader can't rely on the objects' order.
    out = b"%FDF-1.2\n%\xe2\xe3\xcf\xd3\n"
    for n, body in reversed(objects):
        out += f"{n} 0 obj\n".encode() + body + b"\nendobj\n"
    return out + b"trailer\n<< /Root 1 0 R >>\n%%EOF\n"


OTHER = """<annots>
<text page="0" rect="72,700,92,720" name="other" title="Cy"><contents>From the Q2 report</contents></text>
</annots>
<f href="Q2-board-report.pdf"/>
<ids original="0123456789ABCDEF0123456789ABCDEF" modified="0123456789ABCDEF0123456789ABCDEF"/>
</xfdf>
"""

SKIPPED = """<annots>
<text page="0" rect="72,700,92,720" name="kept" title="Ada"><contents>Imported</contents></text>
<sound page="0" rect="100,100,120,120" name="sound"/>
<link page="0" rect="100,200,200,220" name="link"/>
<text page="7" rect="72,700,92,720" name="far"><contents>On page 8</contents></text>
<text page="0" rect="72,700,92,720" name="orphan" inreplyto="gone"><contents>Answers nothing here</contents></text>
</annots>
<fields>
<field name="Name"><value>Ada Lovelace</value></field>
<field name="Agreed"><value>Yes</value></field>
</fields>
"""

MALFORMED = """<annots>
<text page="0" rect="72,700,92,720" name="a"><contents>Fine</contents></text>
<text page="0" rect="72,650,92,670" name="b"><contents>Broken</content></text>
</annots>
"""

DTD = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE xfdf [<!ENTITY boom "boom">]>
<xfdf xmlns="http://ns.adobe.com/xfdf/" xml:space="preserve">
<annots><text page="0" rect="72,700,92,720" name="a"><contents>&boom;</contents></text></annots>
</xfdf>
"""


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    files = {
        "review.xfdf": (HEAD + REVIEW + FOOT).encode(),
        "review.fdf": fdf(),
        "other-document.xfdf": (HEAD + OTHER).encode(),
        "skipped.xfdf": (HEAD + SKIPPED + FOOT).encode(),
        "malformed.xfdf": (HEAD + MALFORMED + FOOT).encode(),
        "dtd.xfdf": DTD.encode(),
    }
    for name, data in files.items():
        (OUT / name).write_bytes(data)
        print(f"wrote fixtures/interchange/{name}")


if __name__ == "__main__":
    main()
