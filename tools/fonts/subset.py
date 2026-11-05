#!/usr/bin/env python3
"""Writes @laminapdf/fonts (todo U6b): subset WOFF2 files, fonts.css and the lock's outputs.

The sources are three npm tarballs, listed with the SHA-256 of each file this script reads in
packages/fonts/fonts.lock.json ("sources"). The script never touches the network: fetch the
tarballs first, then point it at their folder.

    python3 -m venv .venv && .venv/bin/pip install -r tools/fonts/requirements.txt
    (cd <dir> && npm pack source-sans@3.52.0 @fontsource-variable/newsreader@5.3.0 @fontsource/caveat@5.3.0)
    .venv/bin/python tools/fonts/subset.py <dir>            # rewrite files/, fonts.css, the lock's outputs
    .venv/bin/python tools/fonts/subset.py <dir> --check    # fail unless they are already up to date

Each output limits the source's variable axes to the weights the design language uses, keeps
Latin or Latin Extended (the same ranges as Google Fonts' "latin" and "latin-ext" subsets, so the
browser fetches Latin Extended only for text that needs it), keeps the OpenType features text
needs (FEATURES), drops hinting, and is compressed as WOFF2. Newsreader's optical-size axis is
pinned at its default, 18: with the axis its Latin file is 90 KB, without it 37 KB. Output is
deterministic: no timestamp is recalculated and Brotli is pinned.

Source Sans 3's copyright notice reserves the font name "Source" (SIL OFL 1.1 §3), and a subset
is a Modified Version, so its subsets are renamed "Lamina Sans"; the copyright notice and the
licence stay as they are. Newsreader and Caveat reserve no name.
"""

import argparse
import hashlib
import io
import json
import sys
import tarfile
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

PACKAGE = Path(__file__).resolve().parents[2] / "packages" / "fonts"
LOCK = PACKAGE / "fonts.lock.json"
CSS = PACKAGE / "fonts.css"

# Google Fonts' latin and latin-ext ranges, plus the UI's shortcut and arrow symbols in latin
# (the shortcut chips of design language §7); a symbol a font lacks is simply not kept.
LATIN = (
    "U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,"
    "U+2000-206F,U+20AC,U+2122,U+2190-2193,U+21E7,U+2212,U+2215,U+2318,U+2325,U+232B,U+2713,"
    "U+FEFF,U+FFFD"
)
LATIN_EXT = (
    "U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,"
    "U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,"
    "U+A720-A7FF"
)
RANGES = {"latin": LATIN, "latin-ext": LATIN_EXT}

# Kerning, marks, ligatures, contextual alternates, case forms and figure styles. Fractions and
# the like are left out: they cost Lamina Sans' preloaded file 1.2 KB, over its 30 KB budget.
FEATURES = ["rvrn", "ccmp", "locl", "mark", "mkmk", "kern", "liga", "calt", "rlig", "case",
            "tnum", "lnum", "pnum"]

RENAMED = "Lamina Sans"
RENAME_NOTE = (
    "Lamina Sans is Source Sans 3 by Adobe, subset for Lamina PDF and renamed because the font's "
    "licence reserves the name 'Source'."
)

# (family, style, weight range for CSS, source id, member, axis limits, ranges, rename)
FACES = [
    ("Lamina Sans", "normal", "400 700", "source-sans-3", "package/VF/SourceSans3VF-Upright.ttf",
     {"wght": (400, 400, 700)}, ("latin", "latin-ext"), True),
    ("Lamina Sans", "italic", "400 700", "source-sans-3", "package/VF/SourceSans3VF-Italic.ttf",
     {"wght": (400, 400, 700)}, ("latin", "latin-ext"), True),
    ("Newsreader", "normal", "400 600", "newsreader",
     "package/files/newsreader-{range}-opsz-normal.woff2", {"wght": (400, 400, 600), "opsz": 18},
     ("latin", "latin-ext"), False),
    ("Newsreader", "italic", "400", "newsreader",
     "package/files/newsreader-{range}-opsz-italic.woff2", {"wght": 400, "opsz": 18},
     ("latin", "latin-ext"), False),
    ("Caveat", "normal", "500", "caveat", "package/files/caveat-latin-500-normal.woff2",
     {}, ("latin",), False),
]

# The fallback face is Arial, whose advance widths match Helvetica's AFM (tools/std14-gen/data):
# size-adjust scales it to Lamina Sans' average width in English text, and the overrides give it
# Lamina Sans' own vertical metrics, so swapping fonts moves no line.
HELVETICA_WIDTHS = {  # per 1000 em, from Adobe's Helvetica AFM
    " ": 278, "a": 556, "b": 556, "c": 500, "d": 556, "e": 556, "f": 278, "g": 556, "h": 556,
    "i": 222, "j": 222, "k": 500, "l": 222, "m": 833, "n": 556, "o": 556, "p": 556, "q": 556,
    "r": 333, "s": 500, "t": 278, "u": 556, "v": 500, "w": 722, "x": 500, "y": 500, "z": 500,
}
# Relative frequencies of the characters of English text (space included), per 1000.
FREQUENCY = {
    " ": 182, "a": 65, "b": 12, "c": 22, "d": 34, "e": 102, "f": 18, "g": 16, "h": 49, "i": 57,
    "j": 1, "k": 6, "l": 33, "m": 20, "n": 57, "o": 62, "p": 15, "q": 1, "r": 50, "s": 53,
    "t": 75, "u": 23, "v": 8, "w": 17, "x": 1, "y": 16, "z": 1,
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_sources(download: Path, lock: dict) -> dict:
    """Every source file the lock lists, read from its tarball and checked against its hash."""
    files = {}
    for source in lock["sources"]:
        tarball = download / source["tarball"]
        if not tarball.is_file():
            sys.exit(f"missing {tarball}: npm pack {source['package']}@{source['version']}")
        with tarfile.open(tarball) as tar:
            for member, expected in source["files"].items():
                data = tar.extractfile(member).read()
                if sha256(data) != expected:
                    sys.exit(f"{source['tarball']}:{member}: SHA-256 differs from fonts.lock.json")
                files[(source["id"], member)] = data
    return files


def rename(font: TTFont, style: str) -> None:
    """Gives a Source Sans 3 subset the name Lamina Sans (OFL §3), keeping the copyright notice.

    Every name but the copyright (0) and the licence (13, 14) is rewritten, the instance and axis
    names the variable tables point at included, so no name the font presents says "Source".
    """
    suffix = "Italic" if style == "italic" else "Regular"
    table = font["name"]
    fixed = {
        1: RENAMED, 2: suffix, 3: f"LaminaSans-{suffix};{table.getDebugName(5)}",
        4: f"{RENAMED} {suffix}" if style == "italic" else RENAMED, 6: f"LaminaSans-{suffix}",
        10: RENAME_NOTE, 16: RENAMED, 17: suffix, 25: "LaminaSans",
    }
    for record in table.names:
        if record.nameID in (0, 13, 14) or record.nameID in fixed:
            continue
        text = record.toUnicode()
        for old, new in (("Source Sans 3", RENAMED), ("SourceSans3VF", "LaminaSans"),
                         ("SourceSans3", "LaminaSans")):
            text = text.replace(old, new)
        record.string = text
    for name_id in fixed:
        table.removeNames(nameID=name_id)
    for name_id, value in fixed.items():
        table.setName(value, name_id, 3, 1, 0x409)
    leftover = [r.nameID for r in table.names if r.nameID not in (0, 10, 13, 14)
                and "Source" in r.toUnicode()]
    if leftover:
        sys.exit(f"name IDs {leftover} still say 'Source' after renaming")


def subset_face(data: bytes, limits: dict, unicodes: str, renamed: bool, style: str) -> TTFont:
    font = TTFont(io.BytesIO(data), recalcTimestamp=False)
    if limits:
        # Saved and reloaded: the subsetter needs the instanced tables compiled, not lazy.
        buffer = io.BytesIO()
        instancer.instantiateVariableFont(font, limits).save(buffer)
        font = TTFont(io.BytesIO(buffer.getvalue()), recalcTimestamp=False)
    options = subset.Options()
    options.flavor = "woff2"
    options.layout_features = FEATURES
    options.hinting = False
    options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 10, 13, 14, 16, 17, 25]
    options.notdef_outline = True
    options.recalc_timestamp = False
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=subset.parse_unicodes(unicodes))
    subsetter.subset(font)
    if renamed:
        rename(font, style)
    font.flavor = "woff2"
    return font


def coverage(font: TTFont) -> str:
    """The font's mapped code points as U+ ranges."""
    points = sorted(font.getBestCmap())
    ranges, start = [], None
    for i, point in enumerate(points):
        if start is None:
            start = point
        if i + 1 == len(points) or points[i + 1] != point + 1:
            ranges.append(f"U+{start:04X}" if start == point else f"U+{start:04X}-{point:04X}")
            start = None
    return ",".join(ranges)


def fallback_metrics(font: TTFont) -> dict:
    """size-adjust and overrides that make Arial take Lamina Sans' space (the fallback face)."""
    cmap, hmtx, upm = font.getBestCmap(), font["hmtx"], font["head"].unitsPerEm
    ours = sum(FREQUENCY[c] * hmtx[cmap[ord(c)]][0] / upm for c in FREQUENCY)
    arial = sum(FREQUENCY[c] * HELVETICA_WIDTHS[c] / 1000 for c in FREQUENCY)
    size = ours / arial
    hhea = font["hhea"]
    return {
        "sizeAdjust": round(size * 100, 2),
        "ascentOverride": round(hhea.ascent / upm / size * 100, 2),
        "descentOverride": round(-hhea.descent / upm / size * 100, 2),
        "lineGapOverride": round(hhea.lineGap / upm / size * 100, 2),
    }


def css(outputs: list, fallback: dict) -> str:
    lines = [
        "/* @generated by tools/fonts/subset.py from fonts.lock.json: do not edit. */",
        f"/* {RENAME_NOTE} */",
        "",
    ]
    for out in outputs:
        lines += [
            "@font-face {",
            f'  font-family: "{out["family"]}";',
            f"  font-style: {out['style']};",
            f"  font-weight: {out['weight']};",
            "  font-display: swap;",
            f'  src: url("./{out["file"]}") format("woff2");',
            "  unicode-range:",  # on its own line, as Biome formats a long value
            f"    {out['range'].replace(',', ', ')};",
            "}",
            "",
        ]
    lines += [
        "/* Arial (or a metric twin), sized to take Lamina Sans' space while the font loads. */",
        "@font-face {",
        '  font-family: "Lamina Sans Fallback";',
        '  src: local("Arial"), local("ArialMT"), local("Liberation Sans"), local("Arimo");',
        f"  size-adjust: {fallback['sizeAdjust']:g}%;",
        f"  ascent-override: {fallback['ascentOverride']:g}%;",
        f"  descent-override: {fallback['descentOverride']:g}%;",
        f"  line-gap-override: {fallback['lineGapOverride']:g}%;",
        "}",
    ]
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("download", type=Path, help="folder holding the npm tarballs")
    parser.add_argument("--check", action="store_true", help="fail unless the outputs are current")
    args = parser.parse_args()

    lock = json.loads(LOCK.read_text())
    sources = read_sources(args.download, lock)
    written, outputs, fallback = {}, [], None
    for family, style, weight, source, member, limits, ranges, renamed in FACES:
        slug = "lamina-sans" if renamed else family.lower()
        for name in ranges:
            data = sources[(source, member.format(range=name))]
            font = subset_face(data, limits, RANGES[name], renamed, style)
            if renamed and style == "normal" and name == "latin":
                fallback = fallback_metrics(font)
            buffer = io.BytesIO()
            font.save(buffer)
            axis = "wght-" if limits and isinstance(limits.get("wght"), tuple) else ""
            weight_tag = "" if axis else f"{weight}-"
            file = f"files/{slug}-{name}-{axis}{weight_tag}{style}.woff2"
            written[file] = buffer.getvalue()
            outputs.append({
                "file": file, "family": family, "style": style, "weight": weight,
                "range": RANGES[name], "coverage": coverage(font),
            })

    lock["outputs"] = {
        out["file"]: {
            "sha256": sha256(written[out["file"]]),
            "bytes": len(written[out["file"]]),
            "family": out["family"], "style": out["style"], "weight": out["weight"],
            "coverage": out["coverage"],
        }
        for out in outputs
    }
    lock["fallback"] = {"family": "Lamina Sans Fallback", "for": "Lamina Sans", **fallback}
    new_lock = json.dumps(lock, indent=2, ensure_ascii=False) + "\n"
    new_css = css(outputs, fallback)

    def current(path: Path, data: bytes) -> bool:
        return path.is_file() and path.read_bytes() == data

    stale = [p for p, data in written.items() if not current(PACKAGE / p, data)]
    stale += [f"files/{p.name} (not an output)" for p in (PACKAGE / "files").glob("*.woff2")
              if f"files/{p.name}" not in written]
    if LOCK.read_text() != new_lock:
        stale.append("fonts.lock.json")
    if not CSS.is_file() or CSS.read_text() != new_css:
        stale.append("fonts.css")
    if args.check:
        if stale:
            sys.exit("stale: " + ", ".join(str(s) for s in stale))
        print("packages/fonts is up to date")
        return
    (PACKAGE / "files").mkdir(exist_ok=True)
    for old in (PACKAGE / "files").glob("*.woff2"):
        if f"files/{old.name}" not in written:
            old.unlink()
    for path, data in written.items():
        (PACKAGE / path).write_bytes(data)
    LOCK.write_text(new_lock)
    CSS.write_text(new_css)
    for path, data in written.items():
        print(f"{path}: {len(data)} bytes")


if __name__ == "__main__":
    main()
