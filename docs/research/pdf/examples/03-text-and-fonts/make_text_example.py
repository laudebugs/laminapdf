#!/usr/bin/env python3
"""Build a small, hand-written PDF that demonstrates:
  * a simple font (standard-14 Helvetica, WinAnsiEncoding) shown with Tj/TJ
  * a Type 0 / Identity-H composite font wrapping a subset CIDFontType2
    (DejaVu Sans), with /W widths, CIDToGIDMap /Identity and a ToUnicode CMap
    (including a ligature glyph mapped to two code points)
  * invisible text (render mode 3), as used by OCR layers
  * minimal Tagged PDF: StructTreeRoot, P elements, MCIDs, ParentTree, /Lang
Only dependency: fontTools (pip install fonttools).
"""
import io, sys, zlib
from fontTools.ttLib import TTFont
from fontTools import subset

FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
OUT = sys.argv[1] if len(sys.argv) > 1 else "text-example.pdf"

# ---------------------------------------------------------------- font prep
src = TTFont(FONT)
upem = src["head"].unitsPerEm
fs_type = src["OS/2"].fsType
assert fs_type & 0x0002 == 0, "Restricted License embedding: must not embed"

# Text runs for the composite font. U+FB01 (fi ligature) is used directly here
# to show a ligature glyph whose ToUnicode maps to TWO code points ("fi").
# In a real engine the ligature comes from shaping (GSUB 'liga'), not from input.
runs = ["Ελληνικά · Кириллица · ﬁnal", "Hidden OCR-style text layer"]
cmap = src.getBestCmap()
chars = sorted({ord(ch) for r in runs for ch in r})
glyph_names = [cmap[c] for c in chars]

opts = subset.Options()
opts.layout_features = []        # we are not shaping here
opts.name_IDs = []               # PDF does not need the name table
opts.notdef_outline = True
opts.glyph_names = False
opts.retain_gids = False         # compact GIDs -> CIDToGIDMap /Identity still works
opts.drop_tables += ["GSUB", "GPOS", "GDEF", "kern", "MATH", "FFTM", "post"]
sub = subset.Subsetter(opts)
sub.populate(glyphs=glyph_names)
font = TTFont(FONT)
sub.subset(font)
order = font.getGlyphOrder()                 # new GID == index in order
gid_of = {name: i for i, name in enumerate(order)}
buf = io.BytesIO(); font.save(buf); font_bytes = buf.getvalue()

def gid(ch):
    return gid_of[cmap[ord(ch)]]

def hexrun(s):
    return "<" + "".join(f"{gid(ch):04X}" for ch in s) + ">"

# /W array: one c [w ...] block per contiguous GID run, widths in 1/1000 em
hmtx = font["hmtx"].metrics
widths = {i: round(hmtx[n][0] * 1000 / upem) for i, n in enumerate(order)}
W = "[ 0 [" + " ".join(str(widths[i]) for i in range(len(order))) + "] ]"

# ToUnicode: one bfchar per used GID (UTF-16BE hex). Ligature -> 2 code points.
lines = []
for c in chars:
    g = gid_of[cmap[c]]
    uni = "00660069" if c == 0xFB01 else f"{c:04X}"
    lines.append(f"<{g:04X}> <{uni}>")
tounicode = (
    "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n"
    "/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n"
    "/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n"
    "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
    f"{len(lines)} beginbfchar\n" + "\n".join(lines) + "\nendbfchar\n"
    "endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
)

TAG = "LAMINA+"  # six uppercase letters + '+' marks a subset
base = TAG + "DejaVuSans"
head = font["head"]; os2 = font["OS/2"]; hhea = font["hhea"]
s = 1000 / upem
src['glyf'][cmap[ord('H')]].recalcBounds(src['glyf'])
cap_height = round(src['glyf'][cmap[ord('H')]].yMax * s)  # OS/2 v1 has no sCapHeight
bbox = [round(v * s) for v in (head.xMin, head.yMin, head.xMax, head.yMax)]

# ---------------------------------------------------------------- content
content = f"""/P <</MCID 0>> BDC
BT
/F1 24 Tf
72 720 Td
(Hello, PDF text!) Tj
0 -30 Td
[(W) 120 (AVE) -250 (kerned via TJ)] TJ
ET
EMC
/P <</MCID 1>> BDC
BT
/F2 18 Tf
1 0 0 1 72 640 Tm
{hexrun(runs[0])} Tj
ET
EMC
/Artifact BMC
BT
3 Tr
/F2 12 Tf
72 600 Td
{hexrun(runs[1])} Tj
ET
EMC
"""

def stream(d, data, compress=True):
    if isinstance(data, str):
        data = data.encode("latin-1")
    if compress:
        data = zlib.compress(data)
        d = d + " /Filter /FlateDecode"
    return f"<< {d} /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream"

objs = {
 1: "<< /Type /Catalog /Pages 2 0 R /Lang (en-US) /MarkInfo << /Marked true >>"
    " /StructTreeRoot 9 0 R /ViewerPreferences << /DisplayDocTitle true >>"
    " /Metadata 13 0 R >>",
 2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
 3: "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R"
    " /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /StructParents 0 /Tabs /S >>",
 4: stream("", content),
 5: "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
 6: f"<< /Type /Font /Subtype /Type0 /BaseFont /{base} /Encoding /Identity-H"
    f" /DescendantFonts [7 0 R] /ToUnicode 12 0 R >>",
 7: f"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{base}"
    f" /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>"
    f" /FontDescriptor 8 0 R /DW 1000 /W {W} /CIDToGIDMap /Identity >>",
 8: f"<< /Type /FontDescriptor /FontName /{base} /Flags 4"
    f" /FontBBox [{' '.join(map(str, bbox))}] /ItalicAngle 0"
    f" /Ascent {round(hhea.ascent*s)} /Descent {round(hhea.descent*s)}"
    f" /CapHeight {cap_height} /StemV 80"
    f" /FontFile2 11 0 R >>",
 9: "<< /Type /StructTreeRoot /K [10 0 R] /ParentTree 14 0 R /ParentTreeNextKey 1 >>",
 10: "<< /Type /StructElem /S /Document /P 9 0 R /K [15 0 R 16 0 R] >>",
 11: stream(f"/Length1 {len(font_bytes)}", font_bytes),
 12: stream("", tounicode),
 13: None,  # XMP metadata, filled below
 14: "<< /Nums [0 [15 0 R 16 0 R]] >>",
 15: "<< /Type /StructElem /S /P /P 10 0 R /Pg 3 0 R /K 0 >>",
 16: "<< /Type /StructElem /S /P /P 10 0 R /Pg 3 0 R /K 1 >>",
}
xmp = """<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:title><rdf:Alt><rdf:li xml:lang="x-default">laminapdf text example</rdf:li></rdf:Alt></dc:title>
</rdf:Description></rdf:RDF></x:xmpmeta>
<?xpacket end="w"?>""".encode("utf-8")
objs[13] = (f"<< /Type /Metadata /Subtype /XML /Length {len(xmp)} >>\nstream\n").encode() + xmp + b"\nendstream"

out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offsets = {}
for n in sorted(objs):
    offsets[n] = len(out)
    body = objs[n] if isinstance(objs[n], bytes) else objs[n].encode("latin-1")
    out += f"{n} 0 obj\n".encode() + body + b"\nendobj\n"
xref = len(out)
size = max(objs) + 1
out += f"xref\n0 {size}\n0000000000 65535 f \n".encode()
for n in range(1, size):
    out += f"{offsets[n]:010d} 00000 n \n".encode()
out += (f"trailer\n<< /Size {size} /Root 1 0 R /Info << /Title (laminapdf text example) >> >>\n"
        f"startxref\n{xref}\n%%EOF\n").encode()
open(OUT, "wb").write(out)
print(f"wrote {OUT}: {len(out)} bytes; embedded subset {len(font_bytes)} bytes, "
      f"{len(order)} glyphs (orig {len(src.getGlyphOrder())}); fsType={fs_type:#06x}")
