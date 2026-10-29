#!/usr/bin/env python3
"""Writes the shaping goldens fixtures/shaping/<script>.json (todo B4b) with HarfBuzz's
`hb-shape`: for every case in fixtures/shaping/cases.json, the glyphs HarfBuzz produces
(glyph ID, cluster as a UTF-8 byte offset, advances and offsets in font units, no scaling).
crates/lamina-text/tests/shaping_goldens.rs shapes the same cases with HarfRust and compares
number for number; cases HarfRust is known to differ on go in
fixtures/shaping/exceptions.json with an upstream link, never silently.

    python3 tools/shaping-goldens/goldens.py     # needs hb-shape on PATH, at HB_VERSION

`cargo xtask golden --update` runs it when hb-shape is installed. Standard library only.
"""

import json
import subprocess
import sys
from pathlib import Path

# The HarfBuzz the goldens are written with (also checked by shaping_goldens.rs). Every case
# matches HarfRust 0.8 exactly at this version; a future difference goes in exceptions.json.
HB_VERSION = "14.3.0"

ROOT = Path(__file__).resolve().parents[2]
SHAPING = ROOT / "fixtures" / "shaping"
FONTS = ROOT / "fixtures" / "fonts"


def hb_version():
    out = subprocess.run(["hb-shape", "--version"], capture_output=True, text=True, check=True)
    return out.stdout.split()[2]


def shape(case):
    args = [
        "hb-shape",
        "--output-format=json",
        "--no-glyph-names",
        "--utf8-clusters",
        f"--direction={case['direction']}",
        f"--script={case['iso15924']}",
        f"--language={case['language']}",
        str(FONTS / case["font"]),
        f"--text={case['text']}",
    ]
    if case["features"]:
        args.append(f"--features={','.join(case['features'])}")
    out = subprocess.run(args, capture_output=True, text=True, check=True)
    return [
        {"g": g["g"], "cl": g["cl"], "ax": g["ax"], "ay": g["ay"], "dx": g["dx"], "dy": g["dy"]}
        for g in json.loads(out.stdout)
    ]


def main():
    version = hb_version()
    if version != HB_VERSION:
        sys.exit(f"hb-shape is HarfBuzz {version}; the goldens are pinned to {HB_VERSION}")
    cases = json.loads((SHAPING / "cases.json").read_text(encoding="utf-8"))["cases"]
    by_script = {}
    for case in cases:
        by_script.setdefault(case["script"], []).append({"name": case["name"], "glyphs": shape(case)})
    for script, results in sorted(by_script.items()):
        path = SHAPING / f"{script}.json"
        # One glyph per line: diffs stay readable.
        out = [f'{{\n  "harfbuzz": "{version}",\n  "cases": [']
        for i, result in enumerate(results):
            glyphs = ",\n".join("        " + json.dumps(g, separators=(",", ":")) for g in result["glyphs"])
            comma = "," if i + 1 < len(results) else ""
            out.append(f'    {{\n      "name": {json.dumps(result["name"])},\n      "glyphs": [\n{glyphs}\n      ]\n    }}{comma}')
        out.append("  ]\n}\n")
        path.write_text("\n".join(out), encoding="utf-8")
        print(path)


if __name__ == "__main__":
    main()
