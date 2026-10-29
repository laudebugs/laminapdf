#!/usr/bin/env python3
"""Writes the font fixtures in fixtures/fonts/ (todo B4a) from upstream Noto fonts.

Every source is an OFL-1.1 Noto font without a Reserved Font Name, so subsetting and
redistributing it under its original name is allowed (OFL §3, §4: the licence travels with it
as fixtures/fonts/OFL.txt). The downloads are checked against the SHA-256 values below, then
pre-subset with fontTools to small test ranges so the fixtures stay small. Layout tables
(GSUB/GPOS/GDEF) and hinting are kept: B4b shapes with these files. Variation tables are kept
in the variable font. Output is deterministic (no timestamps are recalculated).

    python3 -m venv .venv && .venv/bin/pip install -r tools/fixtures/requirements.txt
    .venv/bin/python tools/fixtures/fonts.py <download-dir>

<download-dir> holds the upstream files (fetched with curl from the URLs below); the script
never touches the network itself.
"""

import hashlib
import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTCollection, TTFont

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "fonts"

NOTO = "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts"
SOURCES = {
    "NotoSans-Regular.ttf": (
        f"{NOTO}/NotoSans/hinted/ttf/NotoSans-Regular.ttf",
        "478c558ea716033cd60c03438f628dfa75694dcf6b5f6d505a2f05fd2b4f3823",
    ),
    "NotoSans-Bold.ttf": (
        f"{NOTO}/NotoSans/hinted/ttf/NotoSans-Bold.ttf",
        "1df075a380fc7cb898acf64c1f7b3b4dd780de3caa860178bf929de35817a913",
    ),
    "NotoSans-Regular.otf": (
        f"{NOTO}/NotoSans/unhinted/otf/NotoSans-Regular.otf",
        "bfb44de2c2031c391ff856943fe8f105ebb5d725cb9a04d493d211d5bbd5c21b",
    ),
    "NotoSans-VF.ttf": (
        "https://raw.githubusercontent.com/google/fonts/main/ofl/notosans/NotoSans%5Bwdth%2Cwght%5D.ttf",
        "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d",
    ),
    "NotoSansArabic-Regular.ttf": (
        f"{NOTO}/NotoSansArabic/hinted/ttf/NotoSansArabic-Regular.ttf",
        "bdff3e5659d67e67def05b33f749683b9376ae819d65d3dd62ac4640b3aaef48",
    ),
    "NotoSansHebrew-Regular.ttf": (
        f"{NOTO}/NotoSansHebrew/hinted/ttf/NotoSansHebrew-Regular.ttf",
        "cdefaf8efd47045f6820928eba84db5bed7557539328952b5f828315485e02ee",
    ),
    "NotoSansDevanagari-Regular.ttf": (
        f"{NOTO}/NotoSansDevanagari/hinted/ttf/NotoSansDevanagari-Regular.ttf",
        "4e3c66638958c3e2ab5d37f47a8deb89fffeb7be9985c665a519bbc7ba762313",
    ),
    "NotoSansMath-Regular.ttf": (
        f"{NOTO}/NotoSansMath/hinted/ttf/NotoSansMath-Regular.ttf",
        "d51afd5739c7ba6c44fcab35a88160e25dfb69a2d4ad0bd99533f8d894af1f96",
    ),
    "NotoSansSC-Regular.otf": (
        "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf",
        "faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9",
    ),
    "OFL.txt": (
        "https://raw.githubusercontent.com/google/fonts/main/ofl/notosans/OFL.txt",
        "cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a",
    ),
}

ASCII = range(0x20, 0x7F)
LATIN = [*ASCII, *range(0xA0, 0x180), *range(0x2010, 0x2028), 0x2030, 0x2039, 0x203A, 0x20AC, *range(0xFB00, 0xFB05)]
CJK_TEXT = "你好世界，这是中文字体子集测试。汉字排版与文本提取"
DEVANAGARI_TEXT = "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण"

# Output name → (source, code points). Code points are the cmap entries kept; GSUB closure adds
# the glyphs those characters can shape into (ligatures, contextual forms, conjuncts).
FIXTURES = {
    "NotoSans-Regular.ttf": ("NotoSans-Regular.ttf", LATIN),
    "NotoSans-Regular.otf": ("NotoSans-Regular.otf", LATIN),
    "NotoSans-VF.ttf": ("NotoSans-VF.ttf", [*ASCII, *range(0xA0, 0x100), *range(0x2018, 0x201F)]),
    "NotoSansArabic-Regular.ttf": ("NotoSansArabic-Regular.ttf", [*ASCII, *range(0x600, 0x700)]),
    "NotoSansHebrew-Regular.ttf": ("NotoSansHebrew-Regular.ttf", [*ASCII, *range(0x590, 0x600)]),
    "NotoSansDevanagari-Regular.ttf": ("NotoSansDevanagari-Regular.ttf", [*ASCII, *(ord(c) for c in DEVANAGARI_TEXT)]),
    "NotoSansMath-Regular.ttf": ("NotoSansMath-Regular.ttf", [*ASCII, *range(0x1D400, 0x1D434)]),
    "NotoSansSC-Regular.otf": ("NotoSansSC-Regular.otf", [*ASCII, *(ord(c) for c in CJK_TEXT)]),
}
# A collection of two members: Regular (index 0) and Bold (index 1), Latin subsets.
TTC = ("NotoSans-RegularBold.ttc", ["NotoSans-Regular.ttf", "NotoSans-Bold.ttf"], [*ASCII, *range(0xA0, 0x100)])


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def subset_font(src, codepoints):
    options = subset.Options()
    options.layout_features = ["*"]
    options.name_IDs = ["*"]
    options.name_languages = ["*"]
    options.notdef_outline = True
    options.glyph_names = True
    options.hinting = True
    options.legacy_kern = True
    options.recalc_timestamp = False
    font = TTFont(src, recalcTimestamp=False)
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=codepoints)
    subsetter.subset(font)
    return font


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    downloads = Path(sys.argv[1])
    for name, (url, digest) in SOURCES.items():
        path = downloads / name
        if not path.is_file() or sha256(path) != digest:
            sys.exit(f"{path}: missing or wrong SHA-256; fetch it with: curl -fL -o {path} '{url}'")
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "OFL.txt").write_bytes((downloads / "OFL.txt").read_bytes())
    for out, (src, codepoints) in FIXTURES.items():
        subset_font(downloads / src, codepoints).save(OUT / out)
    name, members, codepoints = TTC
    collection = TTCollection()
    collection.fonts = [subset_font(downloads / m, codepoints) for m in members]
    collection.save(OUT / name)
    for path in sorted(OUT.iterdir()):
        print(f"{path.name}  {path.stat().st_size}  {sha256(path)}")


if __name__ == "__main__":
    main()
