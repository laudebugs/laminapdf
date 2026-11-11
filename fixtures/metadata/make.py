#!/usr/bin/env python3
"""Writes fixtures/metadata/pdf20-xmp-only.pdf (no dependencies), for the metadata ops (todo E1c).

A one-page PDF 2.0 file whose metadata lives in the XMP packet only, as ISO 32000-2 §14.3.3
recommends: the Info dictionary holds just /CreationDate and /ModDate, the keys PDF 2.0 does
not deprecate. The catalog's /Metadata is an unfiltered XMP stream with dc:title, dc:creator
(two authors), dc:description, pdf:Keywords, pdf:Producer, xmp:CreatorTool, the three xmp:
dates and the xmpMM: IDs, wrapped in an <?xpacket?> with about 2 KB of padding and end="w".

The file is byte-reproducible: every value, including the trailer /ID, is fixed.

    python3 fixtures/metadata/make.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parent / "pdf20-xmp-only.pdf"

XMP = """<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
        xmlns:dc="http://purl.org/dc/elements/1.1/"
        xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
        xmlns:xmp="http://ns.adobe.com/xap/1.0/"
        xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/">
      <dc:format>application/pdf</dc:format>
      <dc:title>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">Quarterly figures</rdf:li>
        </rdf:Alt>
      </dc:title>
      <dc:creator>
        <rdf:Seq>
          <rdf:li>Ada Lovelace</rdf:li>
          <rdf:li>Charles Babbage</rdf:li>
        </rdf:Seq>
      </dc:creator>
      <dc:description>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">Revenue and costs for the third quarter</rdf:li>
        </rdf:Alt>
      </dc:description>
      <pdf:Keywords>finance; quarterly; figures</pdf:Keywords>
      <pdf:Producer>Lamina PDF fixtures/metadata/make.py</pdf:Producer>
      <xmp:CreatorTool>Analytical Engine 1.0</xmp:CreatorTool>
      <xmp:CreateDate>2026-03-12T09:30:00+01:00</xmp:CreateDate>
      <xmp:ModifyDate>2026-03-14T16:45:10+01:00</xmp:ModifyDate>
      <xmp:MetadataDate>2026-03-14T16:45:10+01:00</xmp:MetadataDate>
      <xmpMM:DocumentID>uuid:6f1d2c3b-4a5e-4f60-8b71-92a3b4c5d6e7</xmpMM:DocumentID>
      <xmpMM:InstanceID>uuid:0a1b2c3d-4e5f-4061-8273-8495a6b7c8d9</xmpMM:InstanceID>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
"""
# About 2 KB of whitespace so the packet can be edited in place (XMP Part 1 §7.3.3).
PADDING = ("                                                                   \n" * 30)
PACKET = (XMP + PADDING + '<?xpacket end="w"?>').encode("utf-8")

CONTENT = b"BT /F1 24 Tf 72 720 Td (Quarterly figures) Tj ET"
FILE_ID = "<6F1D2C3B4A5E4F608B7192A3B4C5D6E7>"

OBJECTS = {
    1: b"<< /Type /Catalog /Pages 2 0 R /Metadata 6 0 R >>",
    2: b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    3: b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R"
    b" /Resources << /Font << /F1 5 0 R >> >> >>",
    4: b"<< /Length %d >>\nstream\n%s\nendstream" % (len(CONTENT), CONTENT),
    5: b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    6: b"<< /Type /Metadata /Subtype /XML /Length %d >>\nstream\n%s\nendstream" % (len(PACKET), PACKET),
    7: b"<< /CreationDate (D:20260312093000+01'00') /ModDate (D:20260314164510+01'00') >>",
}


def main():
    out = bytearray(b"%PDF-2.0\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    for num, body in OBJECTS.items():
        offsets[num] = len(out)
        out += b"%d 0 obj\n%s\nendobj\n" % (num, body)
    size = len(OBJECTS) + 1
    xref = len(out)
    out += f"xref\n0 {size}\n0000000000 65535 f\r\n".encode()
    for num in range(1, size):
        out += f"{offsets[num]:010} 00000 n\r\n".encode()
    out += (
        f"trailer\n<< /Size {size} /Root 1 0 R /Info 7 0 R /ID [{FILE_ID} {FILE_ID}] >>\n"
        f"startxref\n{xref}\n%%EOF\n"
    ).encode()
    OUT.write_bytes(out)
    print(f"wrote {OUT} ({len(out)} bytes)")


if __name__ == "__main__":
    main()
