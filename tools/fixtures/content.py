#!/usr/bin/env python3
"""Writes fixtures/content/operators.pdf (no dependencies): one page whose content uses every
kind of content-stream construct laminapdf-content interprets (B9a, ADR 0023), so its operator
counts can be compared with pdf.js's (tools/corpus-facts/pdfjs-operators.mjs).

The page draws: graphics state operators (w J j M d ri i gs, q Q cm), paths with every
painting operator and both clip rules, colour in device, named and pattern spaces
(CS cs SC SCN sc scn G g RG rg K k), a shading (sh), text with every state, positioning and
showing operator (Tc Tw Tz TL Tf Tr Ts Td TD Tm T* Tj TJ ' "), an inline image, an image
XObject, a Form XObject that draws a nested form and the image (with its own /Matrix and
resources), marked content (BMC BDC EMC MP DP, inline and named property lists) and a
compatibility section (BX EX) around an unknown operator.

    python3 tools/fixtures/content.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "content" / "operators.pdf"

PAGE = b"""q
2 w 1 J 1 j 4 M [3 1] 0 d /Perceptual ri 1 i /GS1 gs
1 0 0 1 10 10 cm
0 0 m 10 0 l 10 10 20 20 30 0 c 40 10 50 0 v 60 10 70 0 y h S
0 20 m 10 20 l s
0 40 10 10 re f
20 40 10 10 re F
40 40 10 10 re f*
60 40 10 10 re B
80 40 10 10 re B*
100 40 10 10 re b
120 40 10 10 re b*
0 60 10 10 re W n
20 60 10 10 re W* n
Q
q /DeviceRGB CS 0.1 0.2 0.3 SC /CS1 cs 0.5 sc /CS1 CS 0.25 SCN 0 0 m 5 5 l S Q
q /Pattern cs /P1 scn 0 100 50 50 re f Q
q 0.5 G 0.5 g 1 0 0 RG 0 1 0 rg 0 0 0 1 K 0 0 0 1 k 0 200 m 10 200 l B Q
q 0 300 100 50 re W n /Sh1 sh Q
BT
/F1 12 Tf 0.5 Tc 1 Tw 90 Tz 14 TL 0 Tr 2 Ts
72 700 Td (Td) Tj
0 -14 TD (TD) Tj
T* [(T) -120 (J) 40 (array)] TJ
1 0 0 1 72 600 Tm (quote) '
2 0.5 (double) "
ET
/Span <</MCID 0>> BDC BT /F1 10 Tf 72 500 Td (marked) Tj ET EMC
/Artifact BMC /Note MP /Note <</Key 1>> DP /OC /MC1 BDC EMC EMC
BX 1 2 unknownop EX
q 20 0 0 20 300 300 cm BI /W 2 /H 2 /BPC 8 /CS /G ID \x00\x80\x80\xff EI Q
q 30 0 0 30 350 300 cm /Im1 Do Q
q /Fm1 Do Q
/GS2 gs
q 150 0 0 60 0 0 cm BI /W 150 /H 60 /BPC 1 /CS /G ID BIGDATA EI Q
q 8 0 0 2 0 0 cm BI /IM true /W 8 /H 2 ID \x0f\xf0 EI Q
q 8 0 0 2 0 0 cm /Mask8 Do /Mask1 Do /Soft Do Q
q /Grouped Do /Optional Do Q
"""

INNER = b"0 0 1 1 re f BT /F1 8 Tf (inner) Tj ET"
FORM = b"q 5 0 0 5 0 0 cm /Fm2 Do Q /Im1 Do /P <</MCID 1>> BDC BT /F1 9 Tf (form) Tj ET EMC"


# 150 x 60 at 1 bit per pixel: 19 bytes per row (§8.9.7: inline images over 4 KB are not
# allowed, so this one stays at 1,140 bytes); w + h >= 200, which pdf.js does not inline.
PAGE = PAGE.replace(b"BIGDATA", b"\x55" * (19 * 60))


def stream(entries, data):
    return b"<< " + entries + b" /Length " + str(len(data)).encode() + b" >>\nstream\n" + data + b"\nendstream"


OBJECTS = {
    1: b"<< /Type /Catalog /Pages 2 0 R >>",
    2: b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    3: b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources 5 0 R >>",
    4: stream(b"", PAGE),
    5: b"""<< /Font << /F1 6 0 R >> /XObject << /Im1 7 0 R /Fm1 8 0 R /Mask8 14 0 R /Mask1 15 0 R /Soft 16 0 R
     /Grouped 17 0 R /Optional 18 0 R >>
   /ExtGState << /GS1 << /Type /ExtGState /CA 0.5 /ca 0.5 /LW 1 >> /GS2 << /Type /ExtGState /OP true >> >>
   /ColorSpace << /CS1 [/Indexed /DeviceRGB 1 <000000FFFFFF>] >>
   /Pattern << /P1 10 0 R >> /Shading << /Sh1 11 0 R >>
   /Properties << /MC1 13 0 R >> >>""",
    6: b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    7: stream(b"/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 8",
              b"\x00\x80\x80\xff"),
    8: stream(b"/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Matrix [1 0 0 1 400 400]"
              b" /Resources << /Font << /F1 6 0 R >> /XObject << /Fm2 9 0 R /Im1 7 0 R >> >>", FORM),
    9: stream(b"/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << /Font << /F1 6 0 R >> >>", INNER),
    10: b"<< /PatternType 2 /Shading 11 0 R >>",
    11: b"<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] /Function 12 0 R >>",
    12: b"<< /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >>",
    13: b"<< /Type /OCG /Name (Layer) >>",
    14: stream(b"/Type /XObject /Subtype /Image /Width 8 /Height 2 /ImageMask true", b"\x0f\xf0"),
    15: stream(b"/Type /XObject /Subtype /Image /Width 1 /Height 1 /ImageMask true", b"\x80"),
    16: stream(b"/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 8"
               b" /SMask 7 0 R", b"\x00\x80\x80\xff"),
    17: stream(b"/Type /XObject /Subtype /Form /BBox [0 0 10 10]"
               b" /Group << /Type /Group /S /Transparency >>", b"0 0 5 5 re f"),
    18: stream(b"/Type /XObject /Subtype /Form /BBox [0 0 10 10] /OC 13 0 R", b"0 0 5 5 re f"),
}


def main():
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    for num in sorted(OBJECTS):
        offsets[num] = len(out)
        out += f"{num} 0 obj\n".encode() + OBJECTS[num] + b"\nendobj\n"
    xref = len(out)
    size = max(OBJECTS) + 1
    out += f"xref\n0 {size}\n0000000000 65535 f\r\n".encode()
    for num in range(1, size):
        out += f"{offsets[num]:010} 00000 n\r\n".encode()
    out += f"trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(bytes(out))


if __name__ == "__main__":
    main()
