import sys, zlib, struct
out = sys.argv[1]
content = b"BT /F1 24 Tf 72 720 Td (Hello, World) Tj ET"
inner = {
 1: b"<</Type/Catalog/Pages 2 0 R>>",
 2: b"<</Type/Pages/Kids[3 0 R]/Count 1>>",
 3: b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>",
 4: b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>",
}
hdr = b""; body = b""
for n, o in inner.items():
    hdr += b"%d %d " % (n, len(body)); body += o + b" "
objstm_raw = hdr + body
first = len(hdr)
objstm = zlib.compress(objstm_raw)
buf = bytearray(b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n")
off = {}
off[5] = len(buf)
buf += b"5 0 obj\n<</Length %d>>\nstream\n" % len(content) + content + b"\nendstream\nendobj\n"
off[6] = len(buf)
buf += b"6 0 obj\n<</Type/ObjStm/N 4/First %d/Filter/FlateDecode/Length %d>>\nstream\n" % (first, len(objstm)) + objstm + b"\nendstream\nendobj\n"
off[7] = len(buf)
rows = [struct.pack(">BHB", 0, 0, 255)]
for i, n in enumerate(inner):
    rows.append(struct.pack(">BHB", 2, 6, i))
rows.append(struct.pack(">BHB", 1, off[5], 0))
rows.append(struct.pack(">BHB", 1, off[6], 0))
rows.append(struct.pack(">BHB", 1, off[7], 0))
xs = zlib.compress(b"".join(rows))
buf += b"7 0 obj\n<</Type/XRef/Size 8/W[1 2 1]/Root 1 0 R/Filter/FlateDecode/Length %d>>\nstream\n" % len(xs) + xs + b"\nendstream\nendobj\n"
buf += b"startxref\n%d\n%%%%EOF\n" % off[7]
open(out, "wb").write(buf)
print("objstm header:", objstm_raw[:first], "First", first, "size", len(buf))
