#!/usr/bin/env python3
"""Writes fixtures/pages/set-labels/facts.json (todo E2q): for each file SetPageLabels wrote into
fixtures/pages/set-labels/ (crates/laminapdf-save/tests/labels.rs writes them with
UPDATE_GOLDEN=1), its SHA-256, whether `qpdf --check` passes, and the label of every page as
pdf.js (`getPageLabels()`, tools/fixtures/pdfjs_labels.mjs) and PDFium (`FPDF_GetPageLabel`,
through pypdfium2) show it, null for a page without one. The test checks the op's labels and the
files' current bytes against these facts, so tests need neither Node, Python nor qpdf.

    python3 -m venv .venv && .venv/bin/pip install -r tools/fixtures/requirements.txt
    (cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299)   # pdf.js: Apache-2.0
    PDFJS_DIST=<that dir>/node_modules/pdfjs-dist .venv/bin/python tools/fixtures/label_facts.py
    # needs qpdf on PATH (pinned in doctor.rs) and node
"""

import ctypes
import json

import pypdfium2
import pypdfium2.raw as pdfium
from facts_common import ROOT, qpdf_check, qpdf_version, run_pdfjs, sha256

DIR = ROOT / "fixtures" / "pages" / "set-labels"
OUT = DIR / "facts.json"


def pdfium_labels(path):
    """Each page's label as PDFium reads it (UTF-16LE with a terminating NUL), None when empty."""
    doc = pypdfium2.PdfDocument(str(path))
    labels = []
    for index in range(len(doc)):
        size = pdfium.FPDF_GetPageLabel(doc.raw, index, None, 0)
        if size <= 2:
            labels.append(None)
            continue
        buf = ctypes.create_string_buffer(size)
        pdfium.FPDF_GetPageLabel(doc.raw, index, buf, size)
        labels.append(buf.raw[: size - 2].decode("utf-16-le"))
    doc.close()
    return labels


def main():
    paths = sorted(DIR.glob("*.pdf"))
    js = run_pdfjs("pdfjs_labels.mjs", paths)
    files = {}
    for p in paths:
        files[p.name] = {
            "sha256": sha256(p),
            "qpdf_check": qpdf_check(p),
            "pdfjs": js["files"][p.name],
            "pdfium": pdfium_labels(p),
        }
    result = {
        "pdfjs_version": js["pdfjs_version"],
        "pypdfium2_version": str(pypdfium2.version.PYPDFIUM_INFO),
        "qpdf_version": qpdf_version(),
        "files": files,
    }
    OUT.write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(OUT)


if __name__ == "__main__":
    main()
