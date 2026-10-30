#!/usr/bin/env python3
"""Writes fixtures/build/facts.json (todo B5b): for each of lamina-build's B5b goldens, its
SHA-256, whether `qpdf --check` passes, and what pypdf and pdf.js read from it: each page's
text lines, link annotations, the outline with each item's page, the named destinations, the
page labels, the Info dictionary and the XMP values. crates/lamina-build/tests/external.rs
checks the facts against the builder's input and the goldens' current bytes, so tests need
neither Python, Node nor qpdf.

    python3 -m venv .venv && .venv/bin/pip install -r tools/fixtures/requirements.txt
    (cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299)   # pdf.js: Apache-2.0
    PDFJS_DIST=<that dir>/node_modules/pdfjs-dist .venv/bin/python tools/fixtures/build_facts.py
    # needs qpdf on PATH (pinned in doctor.rs) and node (tools/fixtures/pdfjs_build.mjs)

Run it after `cargo xtask golden --update` changes one of these goldens.
"""

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import pypdf

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "build" / "facts.json"
GOLDENS = ["build-text-latin.pdf", "build-text-multiscript.pdf", "links-and-outlines.pdf", "metadata.pdf"]


def page_index(reader, dest):
    try:
        return reader.get_destination_page_number(dest)
    except Exception:  # noqa: BLE001 - a destination pypdf can't resolve is a fact too
        return None


def outline(reader, items, depth, out):
    for item in items:
        if isinstance(item, list):
            outline(reader, item, depth + 1, out)
            continue
        uri = None
        action = item.get("/A")
        if action is not None and action.get_object().get("/S") == "/URI":
            uri = str(action.get_object()["/URI"])
        out.append(
            {
                "title": str(item.title),
                "depth": depth,
                "page": page_index(reader, item) if item.page is not None and not isinstance(item.page, pypdf.generic.NullObject) else None,
                "url": uri,
            }
        )
    return out


def links(reader, page):
    out = []
    for annot in page.get("/Annots") or []:
        annot = annot.get_object()
        if annot.get("/Subtype") != "/Link":
            continue
        action = annot.get("/A")
        if action is not None and action.get_object().get("/S") == "/URI":
            out.append({"url": str(action.get_object()["/URI"])})
            continue
        dest = annot.get("/Dest")
        if isinstance(dest, (pypdf.generic.TextStringObject, pypdf.generic.ByteStringObject, str)):
            named = reader.named_destinations.get(str(dest))
            out.append({"page": page_index(reader, named) if named is not None else None})
        else:
            target = dest[0].get_object() if dest else None
            index = next((i for i, p in enumerate(reader.pages) if p.indirect_reference == dest[0]), None) if target else None
            out.append({"page": index})
    return out


def xmp_facts(reader):
    x = reader.xmp_metadata
    if x is None:
        return {}
    alt = lambda v: v.get("x-default") if v else None  # noqa: E731
    facts = {
        "dc:title": alt(x.dc_title),
        "dc:creator": "; ".join(x.dc_creator) if x.dc_creator else None,
        "dc:description": alt(x.dc_description),
        "pdf:keywords": x.pdf_keywords,
        "pdf:producer": x.pdf_producer,
        "xmp:creatortool": x.xmp_creator_tool,
        "xmp:createdate": x.xmp_create_date.isoformat() if x.xmp_create_date else None,
        "xmp:modifydate": x.xmp_modify_date.isoformat() if x.xmp_modify_date else None,
    }
    return {k: v for k, v in facts.items() if v is not None}


def facts(path):
    reader = pypdf.PdfReader(path, strict=True)
    check = subprocess.run(["qpdf", "--check", str(path)], capture_output=True, text=True)
    info = {k.lstrip("/"): str(v) for k, v in (reader.metadata or {}).items()}
    return {
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "qpdf_check": check.returncode == 0,
        "pypdf": {
            "pages": [[line for line in p.extract_text().split("\n") if line] for p in reader.pages],
            "links": [links(reader, p) for p in reader.pages],
            "outline": outline(reader, reader.outline, 0, []),
            "destinations": {str(k): page_index(reader, v) for k, v in sorted(reader.named_destinations.items())},
            "page_labels": list(reader.page_labels),
            "info": info,
            "xmp": xmp_facts(reader),
        },
    }


def pdfjs(paths):
    dist = os.environ.get("PDFJS_DIST")
    if not dist:
        sys.exit("set PDFJS_DIST to node_modules/pdfjs-dist (pdfjs-dist@6.4.299)")
    script = ROOT / "tools" / "fixtures" / "pdfjs_build.mjs"
    out = subprocess.run(["node", str(script), dist, *map(str, paths)], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def main():
    paths = [ROOT / "fixtures" / "golden" / name for name in GOLDENS]
    qpdf = subprocess.run(["qpdf", "--version"], capture_output=True, text=True, check=True)
    js = pdfjs(paths)
    files = {p.name: facts(p) for p in paths}
    for name, view in js["files"].items():
        files[name]["pdfjs"] = view
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
