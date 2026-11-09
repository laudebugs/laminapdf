#!/usr/bin/env python3
"""Facts about PDF files from pypdf, an independent reader (ADR 0018 tiers 4 and 6).

Never computed by lamina: lamina's tests compare their results against these.

    facts.py corpus                 rewrite fixtures/corpus/expected.json
    facts.py dir <in> <out.json>    write facts for every file in <in> to <out.json>
                                    (cargo xtask differential --update uses this for mutants)

`corpus` reads and writes only fixtures/corpus/ (encrypted fixtures and their facts live in
fixtures/encrypted/, owned by B2b). Requires the pinned versions in requirements.txt.
"""

import contextlib
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


def outline_pages(reader):
    """Each outline item's 0-based page index, in `outline_titles`' order: None for an item
    whose destination names no page of the file (a URI, another file, a broken reference)."""
    out = []
    stack = [iter(reader.outline)]
    while stack:
        item = next(stack[-1], None)
        if item is None:
            stack.pop()
        elif isinstance(item, list):
            stack.append(iter(item))
        else:
            try:
                page = reader.get_destination_page_number(item)
            except Exception:  # noqa: BLE001 - an item without a page is its own fact
                page = None
            out.append(page if isinstance(page, int) and page >= 0 else None)
    return out


def page_labels(reader, root):
    """Every page's label (ISO 32000-2 §12.4.2) for a file with /PageLabels, else None."""
    return list(reader.page_labels) if "/PageLabels" in root else None


@contextlib.contextmanager
def captured_warnings():
    """Collects pypdf's warnings for the whole block: pypdf parses pages and the outline
    lazily, so the handler stays attached until every fact has been read."""
    handler = Collect()
    logger = logging.getLogger("pypdf")
    saved = (logger.level, logger.propagate)
    logger.setLevel(logging.WARNING)
    logger.propagate = False
    logger.addHandler(handler)
    try:
        yield handler.messages
    finally:
        logger.removeHandler(handler)
        logger.setLevel(saved[0])
        logger.propagate = saved[1]


def open_reader(data):
    """A pypdf reader for `data`, decrypted with the empty password when encrypted."""
    reader = pypdf.PdfReader(io.BytesIO(data), strict=False)
    if reader.is_encrypted:
        reader.decrypt("")
    return reader


def corpus_facts(data):
    """The facts fixtures/corpus/expected.json holds for one file."""
    with captured_warnings():
        return corpus_facts_of(open_reader(data), data)


def corpus_facts_of(reader, data):
    header = re.match(rb"%PDF-(\d\.\d)", data)
    root = reader.trailer["/Root"]
    return {
        "size": len(data),
        "header_version": header.group(1).decode() if header else None,
        "page_count": len(reader.pages),
        "encrypted": reader.is_encrypted,
        "revisions": data.count(b"%%EOF"),
        "has_xref_stream": re.search(rb"/Type\s*/XRef\b", data) is not None,
        "first_page_mediabox": [float(v) for v in reader.pages[0].mediabox] if len(reader.pages) else None,
        "root_keys": sorted(root.keys()),
        "outline_titles": outline_titles(reader),
        "outline_pages": outline_pages(reader),
        "page_labels": page_labels(reader, root),
    }


REPAIR_HINTS = ("xref", "startxref", "trailer", "eof")


def outcome_facts(data):
    """Open/repair/fail outcome, page count and outline titles (the differential's facts).

    The outcome is about opening the file and counting its pages, as lamina's side of the
    comparison is (crates/laminapdf-model/tests/differential.rs): a file that opens but whose
    outline cannot be read keeps its outcome and page count, with `outline_titles` null."""
    with captured_warnings() as warnings:
        try:
            reader = open_reader(data)
            pages = len(reader.pages)
        except Exception:  # noqa: BLE001 - any failure to open is the "failed" outcome
            return {"outcome": "failed", "page_count": None, "outline_titles": None}
        try:
            titles = outline_titles(reader)
        except Exception:  # noqa: BLE001 - an unreadable outline is its own fact
            titles = None
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
