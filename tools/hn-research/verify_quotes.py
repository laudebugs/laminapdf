"""Check every quote of a Hacker News comment in the given Markdown files against the comment it cites.

A quote is a line that starts with a quoted passage ('- "…"', '> "…"'), or text in double quotes
("…" or “…”) inside a line, followed within the next 100 characters (and, inside a line, before the
next quote) by the comment's ID: a news.ycombinator.com/item?id= link or a bare 7-9 digit ID.
Each must appear word for word in that comment (whitespace, curly versus straight quote marks and
'…'/'...' between pieces in order aside), run 40 words or fewer and, when a YYYY-MM-DD date follows
it, carry the comment's date. An ID that names neither a collected comment nor a collected story is
a problem; an ID that names a story is skipped, since quoted titles aren't comments.

Usage: python3 -I tools/hn-research/verify_quotes.py <data-root> [FILE.md|DIR]...
<data-root> holds data/ from collect_*.py. DIRs are searched for *.md. With no paths it checks the
research, the trust and proof plan and the todos.
"""

import json
import re
import sys
from pathlib import Path

H = Path(sys.argv[1])
DEFAULT = ["docs/research/hacker-news", "docs/plans/trust-and-proof.md", "docs/todos"]
text: dict[int, str] = {}
date: dict[int, str] = {}
for src in ("threads.jsonl", "comment_hits.jsonl"):
    for line in open(H / "data" / src):
        c = json.loads(line)
        i = int(c["id"])
        text.setdefault(i, c.get("text") or "")
        date.setdefault(i, (c.get("created_at") or "")[:10])
stories = {int(s["objectID"]) for s in map(json.loads, open(H / "data" / "stories.jsonl"))}

TYPO = str.maketrans({"’": "'", "‘": "'", "“": '"', "”": '"'})
QUOTE = re.compile(r'(?<!\\)["“](?P<q>[^"“”\n]{12,}?)(?<!\\)["”]')
# A line that starts with a quote ('- "', '> "', '1. "') is read whole, so a quote that holds
# straight quotes of its own is checked in full.
LINE = re.compile(r'^\s*(?:[-*]|>|\d+\.)\s+["“](?P<q>.+)["”]\s*(?:\(|—|–|-)')
CITE = re.compile(r"item\?id=(\d+)|(?<![\d#])(\d{7,9})(?!\d)")
DATE = re.compile(r"\b(20\d\d-\d\d-\d\d)\b")
SPLIT = re.compile(r"\s*(?:…|\.\.\.|\[…\]|\[\.\.\.\])\s*")


def norm(s: str) -> str:
    return re.sub(r"\s+", " ", s.replace('\\"', '"').translate(TYPO)).strip()


def problems(q: str, i: int, tail: str) -> list[str]:
    src, out, pos = norm(text[i]), [], 0
    for piece in (norm(p) for p in SPLIT.split(q)):
        if not piece:
            continue
        k = src.find(piece, pos)
        if k < 0:
            out.append("not verbatim")
            break
        pos = k + len(piece)
    if len(q.split()) > 40:
        out.append(f"{len(q.split())} words")
    d = DATE.search(tail)
    if d and d.group(1) != date[i]:
        out.append(f"date {d.group(1)} vs {date[i]}")
    return out


def main() -> None:
    targets = [Path(a) for a in sys.argv[2:]] or [Path(p) for p in DEFAULT]
    files = sorted(f for t in targets for f in ([t] if t.is_file() else t.rglob("*.md")))
    total = bad = skipped = 0
    for f in files:
        n = fails = 0
        for ln in f.read_text().splitlines():
            whole = LINE.match(ln)
            for m in [whole] if whole else QUOTE.finditer(ln):
                rest = ln[m.end():m.end() + 100]
                nxt = re.search(r'(?<!\\)["“]', rest)
                tail = rest[: nxt.start()] if nxt else rest
                cite = CITE.search(tail)
                if not cite:
                    continue
                i = int(cite.group(1) or cite.group(2))
                if i not in text:
                    if i in stories:
                        skipped += 1
                    else:
                        fails += 1
                        print(f"{f}: {i}: ID names no collected comment or story: {m['q'][:80]}")
                    continue
                n += 1
                found = problems(m["q"], i, tail[cite.end():cite.end() + 40])
                if found:
                    fails += 1
                    print(f"{f}: {i}: {', '.join(found)}: {m['q'][:100]}")
        total += n
        bad += fails
        if n or fails:
            print(f"== {f}: {n} quotes checked, {fails} problems")
    print(f"TOTAL {total} quotes, {bad} problems ({skipped} cite stories, not comments)")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
