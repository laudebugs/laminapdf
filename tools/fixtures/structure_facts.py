#!/usr/bin/env python3
"""Writes fixtures/build/structure.json (todo B8a): for each of laminapdf-build's tagged goldens,
its SHA-256, whether `qpdf --check` passes, and the structure tree pdf.js
(`page.getStructTree()`, tools/fixtures/pdfjs_structure.mjs) and PDFium
(`FPDF_StructTree_GetForPage`, through pypdfium2) read per page: each element's type, alternate
text and language, its child elements and its MCIDs. crates/laminapdf-build/tests/tagging.rs checks
them against the builder's input and the goldens' current bytes, so tests need neither Node,
Python nor qpdf.

    python3 -m venv .venv && .venv/bin/pip install -r tools/fixtures/requirements.txt
    (cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299)   # pdf.js: Apache-2.0
    PDFJS_DIST=<that dir>/node_modules/pdfjs-dist .venv/bin/python tools/fixtures/structure_facts.py
    # needs qpdf on PATH (pinned in doctor.rs) and node

Run it after `cargo xtask golden --update` changes one of these goldens.
"""

import ctypes
import json

import pypdfium2
import pypdfium2.raw as pdfium
from facts_common import ROOT, qpdf_check, qpdf_version, run_pdfjs, sha256

OUT = ROOT / "fixtures" / "build" / "structure.json"
GOLDENS = ["tagged-report.pdf"]


def utf16(getter, elem):
    """A PDFium string getter's value (UTF-16LE with a terminating NUL), or None when empty."""
    size = getter(elem, None, 0)
    if size <= 2:
        return None
    buf = ctypes.create_string_buffer(size)
    getter(elem, buf, size)
    return buf.raw[: size - 2].decode("utf-16-le")


def element(elem):
    out = {"type": utf16(pdfium.FPDF_StructElement_GetType, elem)}
    alt = utf16(pdfium.FPDF_StructElement_GetAltText, elem)
    if alt is not None:
        out["alt"] = alt
    lang = utf16(pdfium.FPDF_StructElement_GetLang, elem)
    if lang is not None:
        out["lang"] = lang
    count = pdfium.FPDF_StructElement_GetMarkedContentIdCount(elem)
    # Kids that aren't marked content (child elements, OBJRs) count as -1.
    mcids = [pdfium.FPDF_StructElement_GetMarkedContentIdAtIndex(elem, i) for i in range(max(count, 0))]
    out["mcids"] = [m for m in mcids if m >= 0]
    children = []
    for i in range(pdfium.FPDF_StructElement_CountChildren(elem)):
        child = pdfium.FPDF_StructElement_GetChildAtIndex(elem, i)
        if child:
            children.append(element(child))
    out["children"] = children
    return out


def pdfium_pages(path):
    doc = pypdfium2.PdfDocument(str(path))
    pages = []
    for i in range(len(doc)):
        page = doc[i]
        tree = pdfium.FPDF_StructTree_GetForPage(page.raw)
        roots = []
        for j in range(pdfium.FPDF_StructTree_CountChildren(tree)):
            child = pdfium.FPDF_StructTree_GetChildAtIndex(tree, j)
            if child:
                roots.append(element(child))
        pdfium.FPDF_StructTree_Close(tree)
        pages.append(roots)
        page.close()
    doc.close()
    return pages


def main():
    paths = [ROOT / "fixtures" / "golden" / name for name in GOLDENS]
    js = run_pdfjs("pdfjs_structure.mjs", paths)
    files = {}
    for p in paths:
        files[p.name] = {
            "sha256": sha256(p),
            "qpdf_check": qpdf_check(p),
            "pdfjs": js["files"][p.name],
            "pdfium": {"pages": pdfium_pages(p)},
        }
    result = {
        "pdfjs_version": js["pdfjs_version"],
        "pypdfium2_version": str(pypdfium2.version.PYPDFIUM_INFO),
        "qpdf_version": qpdf_version(),
        "files": files,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(OUT)


if __name__ == "__main__":
    main()
