"""Collect HN stories about developer documentation, 2015-01-01 to 2026-11-12 (HN Algolia API).

Title search per query, stories only, num_comments >= 50. Writes data/stories_raw.jsonl
(every hit, deduplicated, with the queries that found it). The relevance pass is select.py.

Usage: python3 -I collect_stories.py <workdir>
"""

import json
import ssl
import sys
import time
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

OUT = Path(sys.argv[1]) / "data"
CTX = ssl.create_default_context()
START = int(datetime(2015, 1, 1, tzinfo=timezone.utc).timestamp())
END = int(datetime(2026, 11, 12, tzinfo=timezone.utc).timestamp())
MIN_COMMENTS = 50

QUERIES = [
    "documentation", "docs", "developer docs", "API documentation", "API docs", "Diátaxis", "Diataxis",
    "Divio documentation", "docs site", "documentation site", "Stripe docs", "good documentation",
    "bad documentation", "README", "docs as code", "llms.txt", "Starlight", "Docusaurus", "Mintlify",
    "Read the Docs", "ReadTheDocs", "man pages", "man page", "manpages", "examples", "interactive docs",
    "playground", "technical writing", "technical writer", "documenting", "document your code",
    "reference docs", "tutorial", "docs.rs", "MDN", "rustdoc", "Sphinx", "MkDocs", "GitBook",
    "write docs", "writing docs", "writing documentation", "devdocs", "javadoc", "docstrings",
    "changelog", "error messages", "tldr", "cheat sheet", "Rust book", "Arch wiki", "FreeBSD handbook",
    "docs for agents", "AGENTS.md", "developer experience", "onboarding docs", "manual",
]


def get(params: dict) -> dict:
    url = "https://hn.algolia.com/api/v1/search?" + urllib.parse.urlencode(params)
    for attempt in range(5):
        try:
            with urllib.request.urlopen(url, context=CTX, timeout=60) as r:
                return json.load(r)
        except Exception as e:
            if attempt == 4:
                raise
            time.sleep(2 ** attempt)
            print("retry", e, file=sys.stderr)
    raise RuntimeError


KEEP = ("objectID", "title", "url", "author", "points", "num_comments", "created_at", "created_at_i", "story_text")


def years():
    for y in range(2015, 2027):
        a = max(int(datetime(y, 1, 1, tzinfo=timezone.utc).timestamp()), START)
        b = min(int(datetime(y + 1, 1, 1, tzinfo=timezone.utc).timestamp()), END)
        yield a, b


def main() -> None:
    seen: dict[str, dict] = {}
    for q in QUERIES:
        n = 0
        for a, b in years():
            d = get(dict(query=q, tags="story", restrictSearchableAttributes="title", typoTolerance="false",
                         numericFilters=f"created_at_i>={a},created_at_i<{b},num_comments>={MIN_COMMENTS}",
                         hitsPerPage=1000))
            if d["nbHits"] > 1000:
                print("capped:", q, a, d["nbHits"], file=sys.stderr)
            for h in d["hits"]:
                n += 1
                s = seen.setdefault(h["objectID"], {k: h.get(k) for k in KEEP} | {"queries": []})
                s["queries"].append(q)
            time.sleep(0.15)
        print(f"{q}: {n}", file=sys.stderr)
    OUT.mkdir(parents=True, exist_ok=True)
    with open(OUT / "stories_raw.jsonl", "w") as f:
        for s in sorted(seen.values(), key=lambda s: s["created_at_i"]):
            f.write(json.dumps(s) + "\n")
    print("raw:", len(seen), file=sys.stderr)


if __name__ == "__main__":
    main()
