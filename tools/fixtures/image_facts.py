#!/usr/bin/env python3
"""Writes fixtures/images/extract-facts.json: `pdfimages -list` of fixtures/images/extract.pdf, of
every fixtures/corpus/*.pdf and of every fixtures/golden/**/*.pdf, the outside reference image extraction is checked against
(crates/laminapdf-extract/tests/images.rs). A file pdfimages can't read is recorded with its error.
Poppler runs only as an outside tool (ADR 0019), at the version tools/xtask/src/doctor.rs pins.

    python3 tools/fixtures/image_facts.py
"""

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "fixtures"
PDF = FIXTURES / "images" / "extract.pdf"
OUT = ROOT / "fixtures" / "images" / "extract-facts.json"
POPPLER = "26.10.0"


def rows_of(pdf):
    """pdfimages -list of `pdf` as rows, or {"error": …} when it can't read the file."""
    run = subprocess.run(["pdfimages", "-list", str(pdf)], capture_output=True, text=True)
    if run.returncode != 0:
        return {"error": run.stderr.strip().splitlines()[-1] if run.stderr.strip() else "failed"}
    rows = []
    for line in run.stdout.splitlines()[2:]:
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
    return rows


def main():
    version = subprocess.run(["pdfimages", "-v"], capture_output=True, text=True).stderr
    found = re.search(r"pdfimages version (\S+)", version)
    assert found and found.group(1) == POPPLER, f"pdfimages {POPPLER} is needed, found: {version.strip()}"
    files = {}
    for pdf in [PDF, *sorted((FIXTURES / "corpus").glob("*.pdf")), *sorted((FIXTURES / "golden").rglob("*.pdf"))]:
        files[str(pdf.relative_to(FIXTURES))] = rows_of(pdf)
    facts = {
        "comment": "pdfimages -list of extract.pdf, the corpus and the goldens (tools/fixtures/image_facts.py), by "
        "file under fixtures/; page 0-based, object null for an inline image",
        "poppler": POPPLER,
        "files": files,
    }
    OUT.write_text(json.dumps(facts, indent=2) + "\n")


if __name__ == "__main__":
    main()
