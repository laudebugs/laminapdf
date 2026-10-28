import sys
out = sys.argv[1]
content = b"BT\n/F1 24 Tf\n72 720 Td\n(Hello, World) Tj\nET"
objs = [
 b"<< /Type /Catalog /Pages 2 0 R >>",
 b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
 b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\n   /Resources << /Font << /F1 4 0 R >> >>\n   /Contents 5 0 R >>",
 b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
 b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
]
buf = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offs = []
for i, o in enumerate(objs, 1):
    offs.append(len(buf))
    buf += b"%d 0 obj\n" % i + o + b"\nendobj\n"
xref = len(buf)
buf += b"xref\n0 %d\n" % (len(objs)+1)
buf += b"0000000000 65535 f\r\n"
for off in offs:
    buf += b"%010d 00000 n\r\n" % off
buf += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs)+1, xref)
open(out, "wb").write(buf)
print(offs, xref, len(buf))
