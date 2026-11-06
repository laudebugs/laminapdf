"""Check every quote line in the given Markdown files (a line starting with '- "' or '> "') against
the comment it cites: verbatim (whitespace-insensitive; '…'/'...' allowed between pieces that appear
in order), at most 40 words, date matching the comment's.

Usage: python3 -I tools/hn-research/verify_quotes.py <data-root> [FILE.md|DIR]...
<data-root> holds data/ from collect_*.py; DIRs are searched for *.md (default: docs/research/hacker-news)."""
import json, re, sys
from pathlib import Path
H = Path(sys.argv[1])
text, date = {}, {}
for src in ("threads.jsonl", "comment_hits.jsonl"):
    for line in open(H / "data" / src):
        c = json.loads(line); i = int(c["id"])
        text.setdefault(i, c.get("text") or ""); date.setdefault(i, (c.get("created_at") or "")[:10])
stories = {int(s["objectID"]): s for s in map(json.loads, open(H / "data" / "stories.jsonl"))}
norm = lambda s: re.sub(r"\s+", " ", s.replace('\\"', '"')).strip()
LINE = re.compile(r'^\s*(?:[-*]|>|\d+\.)\s+"(?P<q>.+)"\s*(?:\(|—|–|-)')
total = bad = skipped = 0
targets = [Path(a) for a in sys.argv[2:]] or [Path("docs/research/hacker-news")]
files = sorted(f for t in targets for f in ([t] if t.is_file() else t.rglob("*.md")))
for f in files:
    n = fails = 0
    for ln in f.read_text().splitlines():
        m = LINE.match(ln)
        ids = re.findall(r"item\?id=(\d+)", ln)
        if not m or not ids:
            continue
        i = int(ids[0])
        if i not in text:
            skipped += 1
            if i not in stories: print(f"{f.name}: {i}: id not in data: {ln[:100]}")
            continue
        n += 1
        q = m["q"]
        src = norm(text[i])
        pieces = [norm(p) for p in re.split(r"\s*(?:…|\.\.\.|\[…\]|\[\.\.\.\])\s*", q) if norm(p)]
        pos, ok = 0, True
        for p in pieces:
            k = src.find(p, pos)
            if k < 0: ok = False; break
            pos = k + len(p)
        d = re.findall(r"\b(20\d\d-\d\d-\d\d)\b", ln)
        problems = []
        if not ok: problems.append("not verbatim")
        if len(q.split()) > 40: problems.append(f"{len(q.split())} words")
        if d and d[0] != date[i]: problems.append(f"date {d[0]} vs {date[i]}")
        if problems:
            fails += 1; print(f"{f.name}: {i}: {', '.join(problems)}: {q[:100]}")
    total += n; bad += fails
    print(f"== {f.name}: {n} quotes checked, {fails} problems")
print(f"TOTAL {total} quotes, {bad} problems ({skipped} lines cite stories, not comments)")
