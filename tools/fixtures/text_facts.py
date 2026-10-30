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

import json

import pypdf
from facts_common import ROOT, qpdf_check, qpdf_version, run_pdfjs, sha256

OUT = ROOT / "fixtures" / "text" / "extracted.json"


def facts(path):
    reader = pypdf.PdfReader(path, strict=True)
    return {
        "sha256": sha256(path),
        "pypdf": reader.pages[0].extract_text().split("\n"),
        "qpdf_check": qpdf_check(path),
    }


def main():
    goldens = sorted((ROOT / "fixtures" / "golden").glob("text-*.pdf"))
    js = run_pdfjs("pdfjs_text.mjs", goldens)
    files = {p.name: facts(p) for p in goldens}
    for name, lines in js["files"].items():
        files[name]["pdfjs"] = lines
    result = {
        "pypdf_version": pypdf.__version__,
        "pdfjs_version": js["pdfjs_version"],
        "qpdf_version": qpdf_version(),
        "files": files,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(OUT)


if __name__ == "__main__":
    main()
