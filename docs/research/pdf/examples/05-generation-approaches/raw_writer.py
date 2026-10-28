# Minimal dependency-free PDF writer: one page, title, paragraph, rectangle.
import sys, zlib

def esc(s):  # escape a PDF literal string
    return s.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")

TITLE = "Hello from lamina"
PARA = ["This one-page document was produced by hand-emitting PDF objects:",
        "a catalog, a page tree, one page, a font and a content stream."]

# 1. Content stream: graphics + text operators (see chapters 02/03).
ops = ["0.2 0.4 0.8 RG 2 w 72 600 468 120 re S",           # stroked rectangle
       "BT /F2 24 Tf 90 690 Td (%s) Tj ET" % esc(TITLE)]  # title
ops += ["BT /F1 12 Tf 14 TL 90 660 Td"] + ["(%s) '" % esc(l) for l in PARA] + ["ET"]
content = zlib.compress("\n".join(ops).encode("latin-1"))

# 2. Objects, numbered 1..n. Standard-14 fonts need no embedding (not PDF/A!).
objs = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
    b"/Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R >>",
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>",
    b"<< /Length %d /Filter /FlateDecode >>\nstream\n" % len(content)
        + content + b"\nendstream",
    b"<< /Title (%s) /Producer (raw_writer.py) >>" % esc(TITLE).encode(),
]

# 3. Serialize: header, objects (recording byte offsets), xref, trailer.
out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offsets = []
for i, body in enumerate(objs, 1):
    offsets.append(len(out))
    out += b"%d 0 obj\n" % i + body + b"\nendobj\n"
xref = len(out)
out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
out += b"".join(b"%010d 00000 n \n" % o for o in offsets)  # 20-byte entries
out += (b"trailer\n<< /Size %d /Root 1 0 R /Info 7 0 R >>\nstartxref\n%d\n%%%%EOF\n"
        % (len(objs) + 1, xref))
open(sys.argv[1] if len(sys.argv) > 1 else "raw.pdf", "wb").write(out)
