#!/usr/bin/env python3
"""Writes fixtures/text/extracted.json (todo B4a): for every fixtures/golden/text-*.pdf, its
SHA-256, the text pypdf extracts from page 1 (one line per text line) and whether
`qpdf --check` passes. crates/lamina-text/tests/extraction.rs checks the facts against the
samples' input text and the goldens' current bytes, so tests need neither Python nor qpdf.

    python3 -m venv .venv && .venv/bin/pip install -r tools/fixtures/requirements.txt
    .venv/bin/python tools/fixtures/text_facts.py      # needs qpdf on PATH (pinned in doctor.rs)

Run it after `cargo xtask golden --update` changes a text golden.
"""

import hashlib
import json
import subprocess
from pathlib import Path

import pypdf

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "text" / "extracted.json"


def facts(path):
    reader = pypdf.PdfReader(path, strict=True)
    check = subprocess.run(["qpdf", "--check", str(path)], capture_output=True, text=True)
    return {
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "pypdf": reader.pages[0].extract_text().split("\n"),
        "qpdf_check": check.returncode == 0,
    }


def main():
    goldens = sorted((ROOT / "fixtures" / "golden").glob("text-*.pdf"))
    qpdf = subprocess.run(["qpdf", "--version"], capture_output=True, text=True, check=True)
    result = {
        "pypdf_version": pypdf.__version__,
        "qpdf_version": qpdf.stdout.split()[2],
        "files": {p.name: facts(p) for p in goldens},
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(OUT)


if __name__ == "__main__":
    main()
