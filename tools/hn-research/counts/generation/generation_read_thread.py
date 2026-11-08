"""Print comments of given HN stories (full text from data/threads.jsonl + comment_hits), trimmed.
Usage: python3 -I generation_read_thread.py <hn_dir> <max_chars> <max_per_story> <story_id>...
"""
import json, sys
root, maxc, maxn, ids = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4:]
want = set(ids)
by = {i: [] for i in ids}
seen = set()
for src in ("data/threads.jsonl", "data/comment_hits.jsonl"):
    for line in open(f"{root}/{src}"):
        c = json.loads(line)
        sid = str(c.get("story_id"))
        if sid in want and c["id"] not in seen and len(c.get("text") or "") >= 60:
            seen.add(c["id"]); by[sid].append(c)
for sid in ids:
    rows = sorted(by[sid], key=lambda c: c.get("created_at") or "")
    print(f"===== story {sid} ({len(rows)} comments) {rows[0].get('story_title') if rows else ''}")
    for c in rows[:maxn]:
        t = " ".join(c["text"].split())
        print(f"[{c['id']} {(c.get('created_at') or '')[:10]}] {t[:maxc]}")
