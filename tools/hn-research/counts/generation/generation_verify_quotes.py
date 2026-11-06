"""Check every quote in generation.md: exact substring of its comment's text, at most 40 words, and the date shown
matches the comment's date. Quotes are lines of the form  > "QUOTE" — [YYYY-MM-DD, ID](link)
Usage: python3 -I generation_verify_quotes.py <hn_dir> [<markdown file>]
"""
import json, re, sys
root = sys.argv[1]
md = sys.argv[2] if len(sys.argv) > 2 else __file__.rsplit("/", 1)[0] + "/generation.md"
LINE = re.compile(r'^\s*> "(.*)" — \[(\d{4}-\d{2}-\d{2}), (\d+)\]\(https://news\.ycombinator\.com/item\?id=(\d+)\)')
quotes = []
for n, line in enumerate(open(md), 1):
    m = LINE.match(line)
    if m:
        quotes.append((n, m.group(1), m.group(2), int(m.group(3)), int(m.group(4))))
    elif line.lstrip().startswith('> "'):
        quotes.append((n, None, None, None, None))
ids = {q[3] for q in quotes if q[3]}
full = {}
for src in ("data/threads.jsonl", "data/comment_hits.jsonl"):
    for line in open(f"{root}/{src}"):
        c = json.loads(line)
        i = int(c["id"])
        if i in ids and i not in full:
            full[i] = ((c.get("created_at") or "")[:10], c.get("text") or "")
bad = 0
for n, q, d, i, link in quotes:
    if q is None:
        print(f"BAD line {n}: quote line not in the expected format"); bad += 1; continue
    date, text = full.get(i, ("?", ""))
    problems = [p for p, ok in (("not verbatim", q in text), ("> 40 words", len(q.split()) <= 40),
                                ("date mismatch", d == date), ("link id mismatch", i == link)) if not ok]
    if problems:
        bad += 1
        print(f"BAD line {n} id {i}: {', '.join(problems)}")
print(f"{len(quotes)} quotes checked, {bad} bad")
