"""Collect HN comments about PDF software (Nov 2016 to Nov 2026) from the HN Algolia API.

1. data/threads.jsonl: every comment of each story in data/stories.jsonl with >= MIN_COMMENTS
   comments (the items API's full tree), flattened.
2. data/comment_hits.jsonl: comments anywhere on HN matching the TERMS below (typo tolerance off),
   searched one year at a time; data/comment_counts.json records nbHits per term and year, so a
   capped window (over 1,000 hits) shows up as a sample.
"""

import html
import json
import re
import ssl
import sys
import time
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path

DATA = Path(sys.argv[1])
MIN_COMMENTS = 20
CTX = ssl.create_default_context()  # honours SSL_CERT_FILE behind a proxy
START = int(datetime(2016, 11, 1, tzinfo=timezone.utc).timestamp())
END = int(datetime(2026, 11, 7, tzinfo=timezone.utc).timestamp())

TERMS = [
    "pdf reader", "pdf viewer", "pdf editor", "acrobat", "pdf library", "pdf parsing", "parse pdf",
    "pdf extraction", "extract text pdf", "pdf tables", "pdf generation", "generate pdf", "html to pdf",
    "pdf forms", "fill pdf", "sign pdf", "redact pdf", "annotate pdf", "pdf ocr", "pdf markdown",
    "pdf.js", "pdfium", "mupdf", "pymupdf", "poppler", "pdfbox", "itext", "wkhtmltopdf", "weasyprint",
    "puppeteer pdf", "ghostscript", "qpdf", "pdftk", "foxit", "sumatrapdf", "okular", "zathura",
    "preview pdf", "typst", "docling", "pdfplumber", "stirling pdf",
]


def fetch(url: str):
    for attempt in range(6):
        try:
            with urllib.request.urlopen(url, context=CTX, timeout=90) as r:
                return json.load(r)
        except Exception as e:
            if attempt == 5:
                print("giving up", url, e, file=sys.stderr)
                return None
            time.sleep(2 ** attempt)


TAGS = re.compile(r"<[^>]+>")


def clean(text: str | None) -> str:
    if not text:
        return ""
    t = text.replace("<p>", "\n\n")
    return html.unescape(TAGS.sub("", t)).strip()


def flatten(node: dict, story: dict, depth: int, out: list) -> None:
    for c in node.get("children") or []:
        if c.get("type") == "comment" and c.get("text"):
            out.append({
                "id": c["id"], "story_id": story["objectID"], "story_title": story["title"],
                "parent_id": c.get("parent_id"), "author": c.get("author"), "created_at": c.get("created_at"),
                "depth": depth, "text": clean(c.get("text")),
            })
        flatten(c, story, depth + 1, out)


def thread(story: dict) -> list | None:
    """The story's comments, or None when the request failed after its retries."""
    tree = fetch(f"https://hn.algolia.com/api/v1/items/{story['objectID']}")
    if not tree:
        return None
    out: list = []
    flatten(tree, story, 0, out)
    return out


def years():
    for y in range(2016, 2027):
        a = max(int(datetime(y, 1, 1, tzinfo=timezone.utc).timestamp()), START)
        b = min(int(datetime(y + 1, 1, 1, tzinfo=timezone.utc).timestamp()), END)
        if a < b:
            yield y, a, b


def search(term: str, y: int, a: int, b: int):
    params = dict(query=term, tags="comment", typoTolerance="false", restrictSearchableAttributes="comment_text",
                  numericFilters=f"created_at_i>={a},created_at_i<{b}", hitsPerPage=1000)
    d = fetch("https://hn.algolia.com/api/v1/search?" + urllib.parse.urlencode(params))
    if not d:
        return term, y, None, []  # failed after its retries: recorded as null, and main() fails
    rows = [{
        "id": int(h["objectID"]), "story_id": h.get("story_id"), "story_title": h.get("story_title"),
        "parent_id": h.get("parent_id"), "author": h.get("author"), "created_at": h.get("created_at"),
        "text": clean(h.get("comment_text")), "_term": term,
    } for h in d["hits"]]
    return term, y, d["nbHits"], rows


def main() -> None:
    stories = [json.loads(l) for l in open(DATA / "stories.jsonl")]
    big = [s for s in stories if (s.get("num_comments") or 0) >= MIN_COMMENTS]
    failed: list[str] = []
    with ThreadPoolExecutor(8) as pool, open(DATA / "threads.jsonl", "w") as f:
        n = 0
        for story, rows in zip(big, pool.map(thread, big)):
            if rows is None:
                failed.append(f"thread {story['objectID']}")
                continue
            for r in rows:
                f.write(json.dumps(r) + "\n")
                n += 1
    print("threads:", len(big), "comments:", n, file=sys.stderr)

    jobs = [(t, y, a, b) for t in TERMS for (y, a, b) in years()]
    counts: dict[str, dict[int, int]] = {}
    seen: set[int] = set()
    with ThreadPoolExecutor(6) as pool, open(DATA / "comment_hits.jsonl", "w") as f:
        for term, y, nb, rows in pool.map(lambda j: search(*j), jobs):
            counts.setdefault(term, {})[y] = nb
            if nb is None:
                failed.append(f"search {term!r} {y}")
            for r in rows:
                if r["id"] in seen:
                    continue
                seen.add(r["id"])
                f.write(json.dumps(r) + "\n")
    (DATA / "comment_counts.json").write_text(json.dumps(counts, indent=1))
    capped = [(t, y, n) for t, ys in counts.items() for y, n in ys.items() if n is not None and n > 1000]
    print("comment hits (unique):", len(seen), "capped windows:", capped, file=sys.stderr)
    if failed:
        print(f"{len(failed)} request(s) failed after retries; run again:", *failed, sep="\n  ", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
