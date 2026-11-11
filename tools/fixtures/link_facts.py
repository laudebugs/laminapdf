#!/usr/bin/env python3
"""Writes fixtures/links/expected.json: pypdf's view of every link annotation in fixtures/links/
(ADR 0018 tier 4). Never computed by Lamina PDF: crates/laminapdf/tests/links.rs compares its
links against these facts.

For each page, each /Link annotation in /Annots order, with:
  hidden   /F sets Hidden or NoView (a reader leaves it out)
  rect     /Rect as four numbers, or null when it isn't (a reader leaves it out)
  page     the 0-based page a destination goes to (null for a broken one); or
  uri      a URI action's address; or
  file     a GoToR action's file; or
  named    a Named action's name; or
  action   any other action's type

Needs pypdf at the version pinned in tools/fixtures/requirements.txt.

    python3 tools/fixtures/link_facts.py
"""

import json
import sys
from pathlib import Path

import pypdf
from pypdf.generic import ArrayObject, NameObject

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "links"
PINNED = "6.21.3"
HIDDEN = (1 << 1) | (1 << 5)


def page_of(reader, dest):
    """The page index of an explicit destination array, or None."""
    if not isinstance(dest, ArrayObject) or not dest:
        return None
    first = dest[0]
    for index, page in enumerate(reader.pages):
        if page.indirect_reference == first:
            return index
    return None


def named_destinations(reader):
    """Both sources of named destinations. pypdf 6.21 reads the catalog's /Dests or else the
    /Names /Dests tree, never both, so the tree is read through its own walker too."""
    names = dict(reader.named_destinations)
    tree = reader.root_object.get("/Names", {}).get("/Dests")
    if tree is not None:
        for key, dest in reader._get_named_destinations(tree=tree.get_object(), retval={}).items():  # noqa: SLF001
            names.setdefault(key, dest)
    return names


def destination(reader, value):
    """The page a /Dest or /D value goes to: explicit, or by name through pypdf's lookup."""
    value = value.get_object()
    if isinstance(value, ArrayObject):
        return page_of(reader, value)
    named = named_destinations(reader).get(str(value))
    if named is None:
        return None
    return reader.get_destination_page_number(named)


def target(reader, annot):
    if "/Dest" in annot:
        return {"page": destination(reader, annot["/Dest"])}
    action = annot.get("/A")
    if action is None:
        return {}
    action = action.get_object()
    kind = str(action.get("/S"))
    if kind == "/GoTo":
        return {"page": destination(reader, action["/D"])}
    if kind == "/URI":
        return {"uri": bytes(action["/URI"].get_original_bytes()).decode("utf-8")}
    if kind == "/GoToR":
        spec = action["/F"].get_object()
        return {"file": str(spec.get("/UF", spec.get("/F")) if isinstance(spec, dict) else spec)}
    if kind == "/Named":
        return {"named": str(action["/N"]).lstrip("/")}
    return {"action": kind.lstrip("/")}


def rect(annot):
    value = annot.get("/Rect")
    if not isinstance(value, ArrayObject) or len(value) != 4:
        return None
    return [float(v) for v in value]


def links(reader, page):
    out = []
    for annot in page.get("/Annots") or []:
        annot = annot.get_object()
        if annot.get("/Subtype") != NameObject("/Link"):
            continue
        out.append(
            {
                "hidden": int(annot.get("/F", 0)) & HIDDEN != 0,
                "rect": rect(annot),
                **target(reader, annot),
            }
        )
    return out


def main():
    if pypdf.__version__ != PINNED:
        sys.exit(f"pypdf {pypdf.__version__} found; requirements.txt pins {PINNED}")
    facts = {"pypdf_version": pypdf.__version__, "files": {}}
    for path in sorted(DIR.glob("*.pdf")):
        reader = pypdf.PdfReader(path, strict=False)
        facts["files"][path.name] = [links(reader, page) for page in reader.pages]
    out = DIR / "expected.json"
    out.write_text(json.dumps(facts, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
