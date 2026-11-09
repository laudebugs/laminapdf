#!/usr/bin/env python3
"""Writes fixtures/appearance/ for the page appearance tests (todo U2c); standard library only.

Five one-page files, 300 x 200 pt, text in the standard 14 Helvetica (not embedded, so every
renderer draws it from its own copy):

* `text.pdf`: black body text, a grey line and a light-grey line.
* `vector.pdf`: tinted filled boxes, a blue-grey bar chart and stroked rules.
* `photo.pdf`: an opaque 64 x 48 RGB gradient image (no `/SMask`, Normal blend) over most of
  the page, with a caption.
* `mixed.pdf`: text, a filled box, the opaque image, and a second image with an `/SMask`.
* `annotated.pdf`: text under a yellow Highlight annotation (Multiply) and a filled Square
  annotation, both with appearance streams.

Images are raw samples (no filter), so the files are deterministic and readable.

    python3 tools/fixtures/appearance.py
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "appearance"

WIDTH, HEIGHT = 300, 200


def stream(dict_body: str, data: bytes) -> bytes:
    return f"<< {dict_body} /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream"


def write_pdf(path: Path, objects: list[bytes]) -> None:
    """Objects are numbered from 1 in order; object 1 must be the catalog."""
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += f"{number} 0 obj\n".encode() + body + b"\nendobj\n"
    xref = len(out)
    out += f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode()
    for offset in offsets:
        out += f"{offset:010d} 00000 n \n".encode()
    out += f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    path.write_bytes(bytes(out))


def text(x: float, y: float, size: float, words: str, gray: float = 0.0) -> str:
    return f"BT /F1 {size} Tf {gray} g {x} {y} Td ({words}) Tj ET\n"


def gradient(width: int, height: int) -> bytes:
    """An opaque RGB gradient: red across, green down, blue fixed."""
    rows = []
    for y in range(height):
        for x in range(width):
            rows.append(bytes((x * 255 // (width - 1), y * 255 // (height - 1), 96)))
    return b"".join(rows)


def disc_alpha(size: int) -> bytes:
    """A soft mask: opaque inside a disc, transparent outside."""
    centre = (size - 1) / 2
    out = bytearray()
    for y in range(size):
        for x in range(size):
            inside = (x - centre) ** 2 + (y - centre) ** 2 <= centre**2
            out.append(255 if inside else 0)
    return bytes(out)


FONT = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"


def page_file(path: Path, content: str, resources: str, extra: list[bytes], annots: str = "") -> None:
    """Catalog 1, pages 2, page 3, content 4, font 5, then `extra` from 6."""
    annots_entry = f" /Annots [{annots}]" if annots else ""
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {WIDTH} {HEIGHT}] /Contents 4 0 R "
            f"/Resources << /Font << /F1 5 0 R >> {resources} >>{annots_entry} >>"
        ).encode(),
        stream("", content.encode()),
        FONT,
        *extra,
    ]
    write_pdf(path, objects)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    photo = stream(
        "/Type /XObject /Subtype /Image /Width 64 /Height 48 /ColorSpace /DeviceRGB /BitsPerComponent 8",
        gradient(64, 48),
    )

    page_file(
        OUT / "text.pdf",
        text(20, 160, 14, "Body text in black ink")
        + text(20, 130, 12, "Secondary text in mid grey", 0.45)
        + text(20, 100, 12, "Faint text in light grey", 0.75),
        "",
        [],
    )

    page_file(
        OUT / "vector.pdf",
        "1 0.95 0.75 rg 10 120 130 70 re f\n"
        "0.85 0.93 1 rg 160 120 130 70 re f\n"
        "0.306 0.353 0.388 rg 30 20 30 80 re f 80 20 30 55 re f 130 20 30 95 re f\n"
        "0 0 0 RG 1 w 20 18 m 280 18 l S\n"
        "0.8 0.1 0.1 RG 2 w 180 30 m 280 90 l S\n",
        "",
        [],
    )

    page_file(
        OUT / "photo.pdf",
        "q 240 0 0 150 30 40 cm /Im1 Do Q\n" + text(30, 20, 10, "A photo: its pixels never change"),
        "/XObject << /Im1 6 0 R >>",
        [photo],
    )

    soft = stream(
        "/Type /XObject /Subtype /Image /Width 32 /Height 32 /ColorSpace /DeviceGray /BitsPerComponent 8",
        disc_alpha(32),
    )
    masked = stream(
        "/Type /XObject /Subtype /Image /Width 32 /Height 32 /ColorSpace /DeviceRGB /BitsPerComponent 8 "
        "/SMask 8 0 R",
        bytes((40, 120, 200)) * (32 * 32),
    )
    page_file(
        OUT / "mixed.pdf",
        text(20, 175, 14, "Mixed page")
        + "0.9 0.9 0.9 rg 20 120 120 40 re f\n"
        + text(25, 135, 10, "Text in a grey box")
        + "q 100 0 0 75 180 100 cm /Im1 Do Q\n"
        + "q 60 0 0 60 40 30 cm /Im2 Do Q\n",
        "/XObject << /Im1 6 0 R /Im2 7 0 R >>",
        [photo, masked, soft],
    )

    highlight_ap = stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 150 18] /Resources << /ExtGState << /G0 << /BM /Multiply >> >> >>",
        b"q /G0 gs 1 0.9 0.2 rg 0 0 150 18 re f Q",
    )
    square_ap = stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 80 50]",
        b"0.2 0.6 0.3 rg 0 0 80 50 re f 0 0 0 RG 2 w 1 1 78 48 re S",
    )
    page_file(
        OUT / "annotated.pdf",
        text(20, 150, 14, "Highlighted words here") + text(20, 110, 12, "Plain text, recoloured"),
        "",
        [
            b"<< /Type /Annot /Subtype /Highlight /Rect [18 146 168 164] "
            b"/QuadPoints [18 164 168 164 18 146 168 146] /C [1 0.9 0.2] /F 4 /AP << /N 8 0 R >> >>",
            b"<< /Type /Annot /Subtype /Square /Rect [200 30 280 80] /C [0 0 0] /IC [0.2 0.6 0.3] "
            b"/F 4 /AP << /N 9 0 R >> >>",
            highlight_ap,
            square_ap,
        ],
        annots="6 0 R 7 0 R",
    )


if __name__ == "__main__":
    main()
