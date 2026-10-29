#!/usr/bin/env python3
"""Writes fixtures/text/extracted.json (todos B4a, B4b): for every fixtures/golden/text-*.pdf,
its SHA-256, the text pypdf and pdf.js extract from page 1 (one line per text line) and
whether `qpdf --check` passes. crates/lamina-text/tests/extraction.rs checks the facts against
the samples' input text and the goldens' current bytes, so tests need neither Python, Node
nor qpdf.

    python3 -m venv .venv && .venv/bin/pip install -r tools/fixtures/requirements.txt
    (cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299)   # pdf.js: Apache-2.0
    PDFJS_DIST=<that dir>/node_modules/pdfjs-dist .venv/bin/python tools/fixtures/text_facts.py
    # needs qpdf on PATH (pinned in doctor.rs) and node (tools/fixtures/pdfjs_text.mjs)

Run it after `cargo xtask golden --update` changes a text golden.
"""

import hashlib
import json
import os
import subprocess
import sys
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


def pdfjs(goldens):
    """pdf.js's lines for each golden, through tools/fixtures/pdfjs_text.mjs."""
    dist = os.environ.get("PDFJS_DIST")
    if not dist:
        sys.exit("set PDFJS_DIST to node_modules/pdfjs-dist (pdfjs-dist@6.4.299)")
    script = ROOT / "tools" / "fixtures" / "pdfjs_text.mjs"
    out = subprocess.run(["node", str(script), dist, *map(str, goldens)], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def main():
    goldens = sorted((ROOT / "fixtures" / "golden").glob("text-*.pdf"))
    qpdf = subprocess.run(["qpdf", "--version"], capture_output=True, text=True, check=True)
    js = pdfjs(goldens)
    files = {p.name: facts(p) for p in goldens}
    for name, lines in js["files"].items():
        files[name]["pdfjs"] = lines
    result = {
        "pypdf_version": pypdf.__version__,
        "pdfjs_version": js["pdfjs_version"],
        "qpdf_version": qpdf.stdout.split()[2],
        "files": files,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(OUT)


if __name__ == "__main__":
    main()
