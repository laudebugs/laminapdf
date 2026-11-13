#!/usr/bin/env python3
"""Writes fixtures/images/ for laminapdf-build's image and colour tests (todo B5a); standard library
only, plus macOS `sips` for the JPEG conversions.

* `photo-rgb.jpg`: the krilla-fit spike's baseline JPEG (`spikes/krilla-fit/assets/photo.jpg`,
  a generated 192x192 pattern encoded by `sips`), with its APP1 (Exif), APP2 (ICC) and APP13
  (Photoshop) segments removed (it has no APP2; the same filter runs on every JPEG).
* `photo-cmyk.jpg`, `photo-gray.jpg`: the same photo converted by `sips` with macOS's Generic
  CMYK and Generic Gray profiles. `sips` writes CMYK as Photoshop does: inverted samples and an
  Adobe APP14 marker. APP1, APP2 (the embedded Apple ICC profile) and APP13 are removed, so no
  third-party profile data is committed.
* `photo-wide.jpg`: `photo-rgb.jpg` cropped by `sips` to 192x96, so a quarter turn shows; and
  `photo-wide-exif2.jpg` to `photo-wide-exif8.jpg`, the same bytes with an APP1 Exif segment
  after the SOI holding only IFD0's Orientation, 2 to 8, for images to PDF's orientation tests.
* `gray-gamma22.icc`, `srgb-like.icc`: minimal ICC v2 profiles written here from first principles
  (a gamma 2.2 gray TRC; sRGB primaries adapted to D50 with gamma 2.2 TRCs), licensed
  `MIT OR Apache-2.0` like the rest of lamina (their copyright tag says so).

    python3 tools/fixtures/images.py
"""

import shutil
import struct
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "images"
PHOTO = ROOT / "spikes" / "krilla-fit" / "assets" / "photo.jpg"
PROFILES = Path("/System/Library/ColorSync/Profiles")


def strip_app_segments(data: bytes, drop=(0xE1, 0xE2, 0xED)) -> bytes:
    """The JPEG with the APPn segments in `drop` removed (everything before SOS is segments)."""
    assert data[:2] == b"\xff\xd8"
    out, i = bytearray(data[:2]), 2
    while True:
        assert data[i] == 0xFF
        marker = data[i + 1]
        if marker == 0xDA:  # start of scan: the rest is entropy-coded data
            out += data[i:]
            return bytes(out)
        length = struct.unpack(">H", data[i + 2 : i + 4])[0]
        if marker not in drop:
            out += data[i : i + 2 + length]
        i += 2 + length


def s15f16(v: float) -> bytes:
    return struct.pack(">i", round(v * 65536))


def xyz(x, y, z) -> bytes:
    return b"XYZ " + b"\0" * 4 + s15f16(x) + s15f16(y) + s15f16(z)


def curve(gamma: float) -> bytes:
    return b"curv" + b"\0" * 4 + struct.pack(">I", 1) + struct.pack(">H", round(gamma * 256)) + b"\0\0"


def text(s: str) -> bytes:
    return b"text" + b"\0" * 4 + s.encode("ascii") + b"\0"


def desc(s: str) -> bytes:
    ascii_ = s.encode("ascii") + b"\0"
    return (
        b"desc" + b"\0" * 4 + struct.pack(">I", len(ascii_)) + ascii_
        + b"\0" * 8  # Unicode: language code, count 0
        + b"\0" * 3 + b"\0" * 67  # ScriptCode: code, count, 67 bytes
    )


def profile(cls: bytes, space: bytes, tags) -> bytes:
    """An ICC v2.1 profile with `tags` [(signature, data)], each tag aligned to 4 bytes."""
    count = len(tags)
    offset = 128 + 4 + 12 * count
    table, body = b"", b""
    for sig, data in tags:
        table += sig + struct.pack(">II", offset + len(body), len(data))
        body += data + b"\0" * (-len(data) % 4)
    size = offset + len(body)
    header = (
        struct.pack(">I", size) + b"none" + bytes([2, 0x10, 0, 0]) + cls + space + b"XYZ "
        + b"\0" * 12  # date
        + b"acsp" + b"APPL" + b"\0" * 4 + b"none" + b"none" + b"\0" * 8 + b"\0" * 4
        + s15f16(0.9642) + s15f16(1.0) + s15f16(0.8249)  # D50 illuminant
        + b"none" + b"\0" * 44
    )
    assert len(header) == 128
    return header + struct.pack(">I", count) + table + body


def gray_profile() -> bytes:
    return profile(b"mntr", b"GRAY", [
        (b"desc", desc("lamina gray gamma 2.2")),
        (b"cprt", text("MIT OR Apache-2.0")),
        (b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        (b"kTRC", curve(2.2)),
    ])


def rgb_profile() -> bytes:
    # sRGB primaries, Bradford-adapted to D50 (the ICC's PCS white), as in IEC 61966-2-1.
    return profile(b"mntr", b"RGB ", [
        (b"desc", desc("lamina sRGB-like gamma 2.2")),
        (b"cprt", text("MIT OR Apache-2.0")),
        (b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        (b"rXYZ", xyz(0.4361, 0.2225, 0.0139)),
        (b"gXYZ", xyz(0.3851, 0.7169, 0.0971)),
        (b"bXYZ", xyz(0.1431, 0.0606, 0.7141)),
        (b"rTRC", curve(2.2)),
        (b"gTRC", curve(2.2)),
        (b"bTRC", curve(2.2)),
    ])


def crop(height: int, width: int, out: Path):
    """`photo-rgb.jpg` cropped around its centre by `sips` to `height` rows of `width` pixels (the
    order `sips -c` takes them), with the APP1, APP2 and APP13 segments removed like every JPEG
    here; APP0 (JFIF, 72 dpi) stays."""
    with tempfile.TemporaryDirectory() as tmp:
        dst = Path(tmp) / "out.jpg"
        subprocess.run(
            ["sips", "-c", str(height), str(width), str(OUT / "photo-rgb.jpg"), "--out", str(dst)],
            check=True,
            capture_output=True,
        )
        out.write_bytes(strip_app_segments(dst.read_bytes()))


def convert(profile_name: str, out: Path):
    with tempfile.TemporaryDirectory() as tmp:
        dst = Path(tmp) / "out.jpg"
        subprocess.run(["sips", "-m", str(PROFILES / profile_name), str(PHOTO), "--out", str(dst)],
                       check=True, capture_output=True)
        out.write_bytes(strip_app_segments(dst.read_bytes()))


def with_orientation(jpeg: bytes, orientation: int) -> bytes:
    """`jpeg` with an APP1 Exif segment, a big-endian TIFF holding only IFD0's Orientation (tag
    0x0112, SHORT), inserted right after its SOI."""
    tiff = b"MM\0*" + struct.pack(">I", 8) + struct.pack(">H", 1)
    tiff += struct.pack(">HHIHH", 0x0112, 3, 1, orientation, 0) + struct.pack(">I", 0)
    segment = b"Exif\0\0" + tiff
    return jpeg[:2] + b"\xff\xe1" + struct.pack(">H", len(segment) + 2) + segment + jpeg[2:]


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "photo-rgb.jpg").write_bytes(strip_app_segments(PHOTO.read_bytes()))
    if shutil.which("sips"):
        crop(96, 192, OUT / "photo-wide.jpg")
        convert("Generic CMYK Profile.icc", OUT / "photo-cmyk.jpg")
        convert("Generic Gray Profile.icc", OUT / "photo-gray.jpg")
    else:
        print("sips not found (macOS only): photo-cmyk.jpg, photo-gray.jpg and photo-wide.jpg left as they are")
    wide = (OUT / "photo-wide.jpg").read_bytes()
    for orientation in range(2, 9):
        (OUT / f"photo-wide-exif{orientation}.jpg").write_bytes(with_orientation(wide, orientation))
    (OUT / "gray-gamma22.icc").write_bytes(gray_profile())
    (OUT / "srgb-like.icc").write_bytes(rgb_profile())
    for f in sorted(OUT.iterdir()):
        print(f"{f.relative_to(ROOT)} ({f.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
