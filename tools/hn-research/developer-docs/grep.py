"""Print comments matching a regex (case-insensitive), shortest first. Usage: grep.py <workdir> REGEX [MAXLEN] [N]"""
import json, re, sys
from pathlib import Path
H = Path(sys.argv[1]); rx = re.compile(sys.argv[2], re.I); mx = int(sys.argv[3]) if len(sys.argv) > 3 else 500; n = int(sys.argv[4]) if len(sys.argv) > 4 else 12
C = [json.loads(l) for l in open(H / "data/threads.jsonl")]
m = sorted([c for c in C if rx.search(c["text"]) and len(c["text"]) <= mx], key=lambda c: len(c["text"]))
print(len(m), "matches")
for c in m[:n]: print(f"* {c['id']} {c['created_at'][:10]} [{c['story_title'][:40]}] " + " ".join(c["text"].split()))
