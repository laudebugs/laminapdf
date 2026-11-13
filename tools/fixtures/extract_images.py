#!/usr/bin/env python3
"""Writes fixtures/images/extract.pdf for image extraction's tests (todo E3i): standard library
only, plus macOS `sips` for the JPEG 2000 copy of photo-rgb.jpg.

Two pages. Page 1 draws one image of each kind extraction handles:

* JPEGs kept as stored: RGB, CMYK with an inverted /Decode, and grey with a soft mask;
* a JPEG 2000 (the photo through `sips`);
* decoded to PNG: Flate RGB, Indexed (2 bits), CMYK, 16-bit grey, grey with /Decode [1 0], Lab,
  RGB with a /Mask stencil, RGB with a colour-key /Mask, a stencil (/ImageMask), LZW,
  ASCII85 then Flate, RunLength, and a CCITT fax image encoded here by T.4's code tables;
* an inline image (ASCIIHex) and a Form XObject that runs another, which draws its own image
  and the Flate RGB image again;
* soft masks on decoded images (one with /Matte, one half the image's size), an ICCBased
  image, a Separation image, a two-colourant DeviceN image (skipped), a CCITT image with
  /BlackIs1 true, and a second inline image.

Page 2 draws the Flate RGB image once more: an image is extracted once, on its first page.

    python3 tools/fixtures/extract_images.py
"""

import base64
import subprocess
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IMAGES = ROOT / "fixtures" / "images"
OUT = IMAGES / "extract.pdf"


def stream(entries: bytes, data: bytes) -> bytes:
    return b"<< " + entries + b" /Length " + str(len(data)).encode() + b" >>\nstream\n" + data + b"\nendstream"


def image(entries: bytes, data: bytes) -> bytes:
    return stream(b"/Type /XObject /Subtype /Image " + entries, data)


def lzw(data: bytes) -> bytes:
    """LZW with /EarlyChange 1 (§7.4.4): 9- to 12-bit codes, MSB first, starting with Clear."""
    table = {bytes([i]): i for i in range(256)}
    nxt, width, out_bits, codes = 258, 9, [], [(256, 9)]
    w = b""
    for byte in data:
        wc = w + bytes([byte])
        if wc in table:
            w = wc
            continue
        codes.append((table[w], width))
        table[wc] = nxt
        nxt += 1
        if nxt + 1 > (1 << width) and width < 12:
            width += 1
        w = bytes([byte])
    if w:
        codes.append((table[w], width))
    codes.append((257, width))
    for code, bits in codes:
        out_bits.extend((code >> (bits - 1 - i)) & 1 for i in range(bits))
    out_bits.extend([0] * (-len(out_bits) % 8))
    return bytes(int("".join(map(str, out_bits[i : i + 8])), 2) for i in range(0, len(out_bits), 8))


def run_length(data: bytes) -> bytes:
    """RunLength (§7.4.5): literal runs of up to 128 bytes, then the end marker."""
    out = bytearray()
    for i in range(0, len(data), 128):
        chunk = data[i : i + 128]
        out += bytes([len(chunk) - 1]) + chunk
    return bytes(out + b"\x80")


def ccitt_g3() -> bytes:
    """An 8 x 2 Group 3 1-D image (T.4 terminating codes): row 1 all white (white 8: 10011);
    row 2 four white, four black (white 4: 1011, black 4: 011)."""
    bits = "10011" + "1011" + "011"
    bits += "0" * (-len(bits) % 8)
    return bytes(int(bits[i : i + 8], 2) for i in range(0, len(bits), 8))


def rgb_gradient(width: int, height: int) -> bytes:
    return bytes(v for y in range(height) for x in range(width) for v in (x * 255 // (width - 1), y * 255 // (height - 1), 128))


def jpeg2000() -> bytes:
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "photo.jp2"
        subprocess.run(
            ["sips", "-s", "format", "jp2", str(IMAGES / "photo-rgb.jpg"), "--out", str(out)],
            check=True,
            capture_output=True,
        )
        return out.read_bytes()


RGB = rgb_gradient(16, 16)
GRAY = bytes(range(0, 256, 16)) * 16  # 16 x 16, each row a ramp
GRAY16 = b"".join((v * 4369).to_bytes(2, "big") for v in range(16)) * 4  # 16 x 4, 16 bits
CMYK = bytes([0, 255, 255, 0] * 32 + [255, 0, 0, 0] * 32)  # 8 x 8: red rows, then cyan
INDEXED = bytes([0b00011011] * 8)  # 4 x 8 at 2 bits: indexes 0, 1, 2, 3 in every row
LAB = bytes([255, 128, 128] * 16)  # 4 x 4 white, with a* and b* at their middles
MASK_1BIT = bytes([0b11110000] * 16)  # 8 x 16 stencil: a 1 masks, so the left half is hidden
STENCIL = bytes([0b10101010] * 8)  # 8 x 8 stripes
SMASK = bytes(range(0, 256, 16)) * 16

PAGE1 = b"""q
/Jpg Do /Cmyk Do /Soft Do /Jpx Do
/Flate Do /Idx Do /Cmyk8 Do /Gray16 Do /Inv Do /Lab Do /Masked Do /Keyed Do
/Lzw Do /A85 Do /Rl Do /Fax Do
0 0 1 rg /Stencil Do
BI /W 4 /H 2 /BPC 8 /CS /G /F /AHx ID 00 40 80 C0 FF C0 80 40 > EI
/Outer Do
/Matted Do /SoftSmall Do /Icc Do /Sep Do /DevN Do /FaxInv Do
BI /W 2 /H 1 /BPC 8 /CS /RGB ID \xff\x00\x00\x00\x00\xff EI
Q"""

PAGE2 = b"/Flate Do"

OBJECTS = {
    1: b"<< /Type /Catalog /Pages 2 0 R >>",
    2: b"<< /Type /Pages /Kids [3 0 R 30 0 R] /Count 2 >>",
    3: b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources 5 0 R >>",
    4: stream(b"", PAGE1),
    5: b"""<< /XObject << /Jpg 10 0 R /Cmyk 11 0 R /Soft 12 0 R /Jpx 14 0 R /Flate 15 0 R /Idx 16 0 R
      /Cmyk8 17 0 R /Gray16 18 0 R /Inv 19 0 R /Lab 20 0 R /Masked 21 0 R /Keyed 23 0 R /Lzw 24 0 R
      /A85 25 0 R /Rl 26 0 R /Fax 27 0 R /Stencil 28 0 R /Outer 40 0 R /Matted 50 0 R /SoftSmall 52 0 R
      /Icc 54 0 R /Sep 56 0 R /DevN 58 0 R /FaxInv 59 0 R >> >>""",
    10: image(b"/Width 192 /Height 192 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode",
              (IMAGES / "photo-rgb.jpg").read_bytes()),
    11: image(b"/Width 192 /Height 192 /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /DCTDecode"
              b" /Decode [1 0 1 0 1 0 1 0]", (IMAGES / "photo-cmyk.jpg").read_bytes()),
    12: image(b"/Width 192 /Height 192 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /DCTDecode"
              b" /SMask 13 0 R", (IMAGES / "photo-gray.jpg").read_bytes()),
    13: image(b"/Width 16 /Height 16 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode",
              zlib.compress(SMASK)),
    14: image(b"/Width 192 /Height 192 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /JPXDecode", jpeg2000()),
    15: image(b"/Width 16 /Height 16 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode",
              zlib.compress(RGB)),
    16: image(b"/Width 4 /Height 8 /ColorSpace [/Indexed /DeviceRGB 3 <FF000000FF000000FFFFFFFF>]"
              b" /BitsPerComponent 2 /Filter /FlateDecode", zlib.compress(INDEXED)),
    17: image(b"/Width 8 /Height 8 /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /FlateDecode",
              zlib.compress(CMYK)),
    18: image(b"/Width 16 /Height 4 /ColorSpace /DeviceGray /BitsPerComponent 16 /Filter /FlateDecode",
              zlib.compress(GRAY16)),
    19: image(b"/Width 16 /Height 16 /ColorSpace /DeviceGray /BitsPerComponent 8 /Decode [1 0]", GRAY),
    20: image(b"/Width 4 /Height 4 /ColorSpace [/Lab << /WhitePoint [0.9505 1 1.089] /Range [-128 127 -128 127] >>]"
              b" /BitsPerComponent 8", LAB),
    21: image(b"/Width 16 /Height 16 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask 22 0 R", RGB),
    22: image(b"/Width 8 /Height 16 /ImageMask true", MASK_1BIT),
    23: image(b"/Width 16 /Height 16 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask [0 0 0 255 128 128]", RGB),
    24: image(b"/Width 16 /Height 16 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /LZWDecode", lzw(RGB)),
    25: image(b"/Width 16 /Height 16 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter [/ASCII85Decode /FlateDecode]",
              base64.a85encode(zlib.compress(GRAY)) + b"~>"),
    26: image(b"/Width 16 /Height 16 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /RunLengthDecode",
              run_length(GRAY)),
    27: image(b"/Width 8 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 1 /Filter /CCITTFaxDecode"
              b" /DecodeParms << /K 0 /Columns 8 /Rows 2 /EndOfBlock false >>", ccitt_g3()),
    28: image(b"/Width 8 /Height 8 /ImageMask true", STENCIL),
    30: b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 31 0 R"
        b" /Resources << /XObject << /Flate 15 0 R >> >> >>",
    31: stream(b"", PAGE2),
    40: stream(b"/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << /XObject << /Inner 41 0 R >> >>",
               b"/Inner Do"),
    41: stream(b"/Type /XObject /Subtype /Form /BBox [0 0 100 100]"
               b" /Resources << /XObject << /Deep 42 0 R /Flate 15 0 R >> >>", b"/Deep Do /Flate Do"),
    42: image(b"/Width 2 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 8", b"\x00\xff\xff\x00"),
    # Red blended with black at half opacity (premultiplied, /Matte [0 0 0]): extraction takes the
    # matte out, giving red at half opacity.
    50: image(b"/Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 51 0 R", bytes([128, 0, 0, 255, 0, 0])),
    51: image(b"/Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 /Matte [0 0 0]", bytes([128, 255])),
    # A decoded image with a soft mask of half its size, which is resampled.
    52: image(b"/Width 16 /Height 16 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 53 0 R", RGB),
    53: image(b"/Width 8 /Height 8 /ColorSpace /DeviceGray /BitsPerComponent 8", bytes(range(0, 256, 4))),
    54: image(b"/Width 16 /Height 16 /ColorSpace [/ICCBased 55 0 R] /BitsPerComponent 8", GRAY),
    55: stream(b"/N 1", (IMAGES / "gray-gamma22.icc").read_bytes()),
    56: image(b"/Width 4 /Height 1 /ColorSpace [/Separation /Spot /DeviceCMYK 57 0 R] /BitsPerComponent 8",
              bytes([0, 64, 128, 255])),
    57: b"<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 0 0] /N 1 >>",
    # DeviceN with two colourants: its tint transform isn't evaluated, so it is skipped.
    58: image(b"/Width 2 /Height 1 /ColorSpace [/DeviceN [/A /B] /DeviceCMYK 57 0 R] /BitsPerComponent 8",
              bytes([0, 0, 255, 255])),
    59: image(b"/Width 8 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 1 /Filter /CCITTFaxDecode"
              b" /DecodeParms << /K 0 /Columns 8 /Rows 2 /EndOfBlock false /BlackIs1 true >>", ccitt_g3()),
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
        if num in offsets:
            out += f"{offsets[num]:010} 00000 n\r\n".encode()
        else:
            out += b"0000000000 65535 f\r\n"
    out += f"trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    OUT.write_bytes(bytes(out))


if __name__ == "__main__":
    main()
