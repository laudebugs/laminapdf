"""Collect Hacker News stories about PDF software, Nov 2016 to Nov 2026, from the HN Algolia API.

Writes data/stories_raw.jsonl (every hit, deduplicated by objectID) and data/stories.jsonl
(the ones whose subject is PDF software: "pdf" outside a "[pdf]"/"(PDF, …)" link tag, or a
PDF tool's name in the title).
"""

import json
import re
import ssl
import sys
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

OUT = Path(sys.argv[1])
CTX = ssl.create_default_context()  # honours SSL_CERT_FILE behind a proxy
START = int(datetime(2016, 11, 1, tzinfo=timezone.utc).timestamp())
END = int(datetime(2026, 11, 7, tzinfo=timezone.utc).timestamp())

# Tools whose names Algolia doesn't split into "pdf" (or that lack it), searched in titles by year.
TOOLS = [
    "mupdf", "pymupdf", "poppler", "ghostscript", "acrobat", "pdfbox", "itext", "wkhtmltopdf",
    "weasyprint", "docling", "pdfplumber", "pdftk", "qpdf", "okular", "sumatrapdf", "zathura",
    "typst", "llamaparse", "olmocr", "pdfium", "pdfjs", "mistral ocr", "paged.js", "prince xml",
    "foxit", "preview.app", "pdfkit", "reportlab", "unstructured.io", "tesseract", "ocrmypdf",
]


def get(params: dict) -> dict:
    url = "https://hn.algolia.com/api/v1/search?" + urllib.parse.urlencode(params)
    for attempt in range(5):
        try:
            with urllib.request.urlopen(url, context=CTX, timeout=60) as r:
                return json.load(r)
        except Exception as e:  # network hiccup: back off and retry
            if attempt == 4:
                raise
            time.sleep(2 ** attempt)
            print("retry", e, file=sys.stderr)
    raise RuntimeError("unreachable")


def months():
    y, m = 2016, 11
    while True:
        a = int(datetime(y, m, 1, tzinfo=timezone.utc).timestamp())
        y2, m2 = (y + 1, 1) if m == 12 else (y, m + 1)
        b = int(datetime(y2, m2, 1, tzinfo=timezone.utc).timestamp())
        if a >= END:
            return
        yield a, min(b, END)
        y, m = y2, m2


def years():
    for y in range(2016, 2027):
        a = max(int(datetime(y, 1, 1, tzinfo=timezone.utc).timestamp()), START)
        b = min(int(datetime(y + 1, 1, 1, tzinfo=timezone.utc).timestamp()), END)
        if a < b:
            yield a, b


KEEP = ("objectID", "title", "url", "author", "points", "num_comments", "created_at", "created_at_i", "story_text")
TAG = re.compile(r"[\[(][^\])]*\bpdfs?\b[^\])]*[\])]", re.I)


def is_about_pdf(title: str) -> bool:
    t = title or ""
    if re.search(r"\bpdf", TAG.sub(" ", t), re.I):
        return True
    low = t.lower()
    return any(tool in low for tool in TOOLS)


def main() -> None:
    seen: dict[str, dict] = {}
    for a, b in months():
        d = get(dict(query="pdf", tags="story", restrictSearchableAttributes="title",
                     numericFilters=f"created_at_i>={a},created_at_i<{b}", hitsPerPage=1000))
        if d["nbHits"] > 1000:
            print("window over 1000:", a, d["nbHits"], file=sys.stderr)
        for h in d["hits"]:
            h["_query"] = "pdf"
            seen.setdefault(h["objectID"], {k: h.get(k) for k in KEEP} | {"_query": "pdf"})
        time.sleep(0.2)
    print("pdf-title hits:", len(seen), file=sys.stderr)
    for tool in TOOLS:
        n = 0
        for a, b in years():
            d = get(dict(query=tool, tags="story", restrictSearchableAttributes="title",
                         numericFilters=f"created_at_i>={a},created_at_i<{b}", hitsPerPage=1000))
            for h in d["hits"]:
                n += 1
                seen.setdefault(h["objectID"], {k: h.get(k) for k in KEEP} | {"_query": tool})
            time.sleep(0.2)
        print(f"{tool}: {n}", file=sys.stderr)
    OUT.mkdir(parents=True, exist_ok=True)
    with open(OUT / "stories_raw.jsonl", "w") as f:
        for s in seen.values():
            f.write(json.dumps(s) + "\n")
    kept = [s for s in seen.values() if is_about_pdf(s.get("title") or "")]
    with open(OUT / "stories.jsonl", "w") as f:
        for s in sorted(kept, key=lambda s: s["created_at_i"]):
            f.write(json.dumps(s) + "\n")
    print("raw:", len(seen), "kept:", len(kept), file=sys.stderr)


if __name__ == "__main__":
    main()
