#!/usr/bin/env python3
"""Facts about PDF files from pypdf, an independent reader (ADR 0018 tiers 4 and 6).

Never computed by lamina: lamina's tests compare their results against these.

    facts.py corpus                 rewrite fixtures/corpus/expected.json
    facts.py dir <in> <out.json>    write facts for every file in <in> to <out.json>
                                    (cargo xtask differential --update uses this for mutants)

`corpus` reads and writes only fixtures/corpus/ (encrypted fixtures and their facts live in
fixtures/encrypted/, owned by B2b). Requires the pinned versions in requirements.txt.
"""

import io
import json
import logging
import re
import sys
from pathlib import Path

import pypdf

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / "fixtures" / "corpus"
PINNED = "6.21.3"


class Collect(logging.Handler):
    """Collects pypdf's warnings: they are how it reports a repaired cross-reference table."""

    def __init__(self):
        super().__init__(logging.WARNING)
        self.messages = []

    def emit(self, record):
        self.messages.append(record.getMessage())


def outline_titles(reader):
    """Every outline title, depth-first (each item before its children)."""
    out = []
    stack = [iter(reader.outline)]
    while stack:
        item = next(stack[-1], None)
        if item is None:
            stack.pop()
        elif isinstance(item, list):
            stack.append(iter(item))
        else:
            out.append(str(item.title))
    return out


def read(data):
    """(reader, warnings) for `data`; the reader is None when pypdf cannot open it."""
    handler = Collect()
    logger = logging.getLogger("pypdf")
    logger.setLevel(logging.WARNING)
    logger.propagate = False
    logger.addHandler(handler)
    try:
        reader = pypdf.PdfReader(io.BytesIO(data), strict=False)
        if reader.is_encrypted:
            reader.decrypt("")
        return reader, handler.messages
    finally:
        logger.removeHandler(handler)


def corpus_facts(data):
    """The facts fixtures/corpus/expected.json holds for one file."""
    reader, _ = read(data)
    header = re.match(rb"%PDF-(\d\.\d)", data)
    root = reader.trailer["/Root"]
    return {
        "size": len(data),
        "header_version": header.group(1).decode() if header else None,
        "page_count": len(reader.pages),
        "encrypted": reader.is_encrypted,
        "revisions": data.count(b"%%EOF"),
        "has_xref_stream": re.search(rb"/Type\s*/XRef\b", data) is not None,
        "first_page_mediabox": [float(v) for v in reader.pages[0].mediabox],
        "root_keys": sorted(root.keys()),
        "outline_titles": outline_titles(reader),
    }


REPAIR_HINTS = ("xref", "startxref", "trailer", "eof")


def outcome_facts(data):
    """Open/repair/fail outcome, page count and outline titles (the differential's facts)."""
    try:
        reader, warnings = read(data)
        pages = len(reader.pages)
        titles = outline_titles(reader)
    except Exception:  # noqa: BLE001 - any failure to read is the "failed" outcome
        return {"outcome": "failed", "page_count": None, "outline_titles": None}
    repaired = any(hint in w.lower() for w in warnings for hint in REPAIR_HINTS)
    return {
        "outcome": "repaired" if repaired else "opened",
        "page_count": pages,
        "outline_titles": titles,
    }


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def main(argv):
    if pypdf.__version__ != PINNED:
        sys.exit(f"pypdf {pypdf.__version__} found; requirements.txt pins {PINNED}")
    if argv == ["corpus"]:
        facts = {p.name: corpus_facts(p.read_bytes()) for p in sorted(CORPUS.glob("*.pdf"))}
        write_json(CORPUS / "expected.json", facts)
        print(f"wrote facts for {len(facts)} files to {CORPUS / 'expected.json'}")
    elif len(argv) == 3 and argv[0] == "dir":
        src, out = Path(argv[1]), Path(argv[2])
        facts = {p.name: outcome_facts(p.read_bytes()) for p in sorted(src.iterdir()) if p.is_file()}
        write_json(out, facts)
        print(f"wrote facts for {len(facts)} files to {out}")
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    logging.basicConfig(level=logging.ERROR)
    main(sys.argv[1:])
