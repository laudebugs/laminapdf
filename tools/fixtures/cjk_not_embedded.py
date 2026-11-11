#!/usr/bin/env python3
"""Writes fixtures/desktop/cjk-not-embedded.pdf (no dependencies): one page of Chinese text in a
font the file doesn't embed, so a renderer has to find a system font for it. The desktop engine's
tests render it inside the sandbox, where the font comes from the main process (ADR 0008 §5).

The font is a Type 0 font named STSong-Light, one of Adobe's standard CJK names that PDF writers
use without embedding, with the predefined CMap UniGB-UCS2-H (ISO 32000-1 §9.7.5.2) and
Adobe-GB1 as its character collection. The text is six common characters in UCS-2. The page has
no other ink, so every dark pixel is a glyph.

    python3 tools/fixtures/cjk_not_embedded.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "desktop" / "cjk-not-embedded.pdf"

TEXT = "中文字体测试"
CONTENT = "BT /F1 40 Tf 20 30 Td <" + TEXT.encode("utf-16-be").hex().upper() + "> Tj ET"

OBJECTS = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 100] /Contents 4 0 R"
    " /Resources << /Font << /F1 5 0 R >> >> >>",
    f"<< /Length {len(CONTENT)} >>\nstream\n{CONTENT}\nendstream",
    "<< /Type /Font /Subtype /Type0 /BaseFont /STSong-Light /Encoding /UniGB-UCS2-H"
    " /DescendantFonts [6 0 R] >>",
    "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /STSong-Light"
    " /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 2 >>"
    " /FontDescriptor 7 0 R /DW 1000 >>",
    "<< /Type /FontDescriptor /FontName /STSong-Light /Flags 6 /FontBBox [-25 -254 1000 880]"
    " /ItalicAngle 0 /Ascent 880 /Descent -120 /CapHeight 880 /StemV 93 >>",
]


def main():
    out = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for number, body in enumerate(OBJECTS, start=1):
        offsets.append(len(out))
        out += f"{number} 0 obj\n{body}\nendobj\n".encode("latin-1")
    xref = len(out)
    out += f"xref\n0 {len(OBJECTS) + 1}\n0000000000 65535 f \n".encode()
    for offset in offsets:
        out += f"{offset:010} 00000 n \n".encode()
    out += f"trailer\n<< /Size {len(OBJECTS) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(bytes(out))
    print(f"wrote {OUT.relative_to(OUT.parents[2])} ({len(out)} bytes)")


if __name__ == "__main__":
    main()
