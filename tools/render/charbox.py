#!/usr/bin/env python3
"""PDFium's character boxes for the quad test (todo B9c, ADR 0018 tier 6).

Writes, for every page of every given PDF, PDFium's loose character boxes
(`FPDFText_GetLooseCharBox`: the font's ascent to descent, as lamina's quads are) with each
character's text, in PDF user space (origin bottom left), plus the page's CropBox and /Rotate.
`cargo xtask quads --update` runs it into fixtures/text/charboxes.json;
crates/lamina/tests/quads.rs compares lamina's glyph quads with it.

    python3 tools/render/charbox.py <out.json> <repo-relative pdf>...
"""

from __future__ import annotations

import json
import sys

import pypdfium2 as pdfium
import pypdfium2.version as pdfium_version

PINNED = "5.14.0"


def rounded(values):
    return [round(v, 2) for v in values]


def page_boxes(page) -> dict:
    text = page.get_textpage()
    chars = []
    for i in range(text.count_chars()):
        chars.append([text.get_text_range(i, 1), *rounded(text.get_charbox(i, loose=True))])
    return {
        "cropbox": rounded(page.get_cropbox()),
        "rotation": page.get_rotation(),
        "chars": chars,
    }


def dumps(out: dict) -> str:
    """The JSON with one character per line (a reviewable diff at a third of indent=1's size)."""
    compact = lambda v: json.dumps(v, ensure_ascii=False, separators=(",", ":"))  # noqa: E731
    lines = ["{", f' "pypdfium2": {compact(out["pypdfium2"])},', f' "pdfium": {compact(out["pdfium"])},', ' "files": {']
    files = list(out["files"].items())
    for i, (name, pages) in enumerate(files):
        lines.append(f"  {compact(name)}: [")
        for j, page in enumerate(pages):
            chars = ",\n".join(f"     {compact(c)}" for c in page["chars"])
            body = f'    {{"cropbox": {compact(page["cropbox"])}, "rotation": {page["rotation"]}, "chars": [\n{chars}\n    ]}}'
            lines.append(body + ("," if j + 1 < len(pages) else ""))
        lines.append("  ]" + ("," if i + 1 < len(files) else ""))
    lines += [" }", "}"]
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    version = str(pdfium_version.PYPDFIUM_INFO)
    if version != PINNED:
        print(f"pypdfium2 {version}, expected {PINNED} (tools/render/requirements.txt)", file=sys.stderr)
        return 1
    out = {"pypdfium2": version, "pdfium": str(pdfium_version.PDFIUM_INFO), "files": {}}
    for path in argv[2:]:
        doc = pdfium.PdfDocument(path)
        out["files"][path] = [page_boxes(doc[i]) for i in range(len(doc))]
    with open(argv[1], "w", encoding="utf-8") as f:
        f.write(dumps(out))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
