#!/usr/bin/env python3
"""Writes fixtures/images/extract-facts.json: `pdfimages -list` of fixtures/images/extract.pdf, the
outside reference image extraction is checked against (crates/laminapdf-extract/tests/images.rs).
Poppler runs only as an outside tool (ADR 0019), at the version tools/xtask/src/doctor.rs pins.

    python3 tools/fixtures/image_facts.py
"""

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PDF = ROOT / "fixtures" / "images" / "extract.pdf"
OUT = ROOT / "fixtures" / "images" / "extract-facts.json"
POPPLER = "26.10.0"


def main():
    version = subprocess.run(["pdfimages", "-v"], capture_output=True, text=True).stderr
    found = re.search(r"pdfimages version (\S+)", version)
    assert found and found.group(1) == POPPLER, f"pdfimages {POPPLER} is needed, found: {version.strip()}"
    listing = subprocess.run(["pdfimages", "-list", str(PDF)], capture_output=True, text=True, check=True).stdout
    rows = []
    for line in listing.splitlines()[2:]:
        cols = line.split()
        inline = cols[10] == "[inline]"
        rows.append(
            {
                "page": int(cols[0]) - 1,
                "type": cols[2],
                "width": int(cols[3]),
                "height": int(cols[4]),
                "color": cols[5],
                "bpc": int(cols[7]),
                "enc": cols[8],
                "object": None if inline else int(cols[10]),
            }
        )
    facts = {
        "comment": "pdfimages -list of extract.pdf (tools/fixtures/image_facts.py), page 0-based, "
        "object null for an inline image",
        "poppler": POPPLER,
        "rows": rows,
    }
    OUT.write_text(json.dumps(facts, indent=2) + "\n")


if __name__ == "__main__":
    main()
