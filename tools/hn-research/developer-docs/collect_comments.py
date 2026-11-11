"""Fetch every comment of each story in data/stories.jsonl (items API, full tree), flattened into
data/threads.jsonl. Text is HTML-unescaped with tags removed (as tools/hn-research does)."""
import html, json, re, ssl, sys, time, urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
H = Path(sys.argv[1]); CTX = ssl.create_default_context(); TAGS = re.compile(r"<[^>]+>")
def fetch(url):
    for a in range(6):
        try:
            with urllib.request.urlopen(url, context=CTX, timeout=90) as r: return json.load(r)
        except Exception as e:
            if a == 5: print("giving up", url, e, file=sys.stderr); return None
            time.sleep(2 ** a)
def clean(t):
    if not t: return ""
    t = t.replace("<p>", "\n\n")
    return html.unescape(TAGS.sub("", t)).strip()
def flat(node, s, depth, out):
    for c in node.get("children") or []:
        if c.get("type") == "comment" and c.get("text"):
            out.append({"id": c["id"], "story_id": s["objectID"], "story_title": s["title"], "set": s["set"],
                        "parent_id": c.get("parent_id"), "author": c.get("author"),
                        "created_at": c.get("created_at"), "depth": depth, "text": clean(c["text"])})
        flat(c, s, depth + 1, out)
def thread(s):
    t = fetch(f"https://hn.algolia.com/api/v1/items/{s['objectID']}")
    if not t: return s, None
    out = []; flat(t, s, 0, out); return s, out
stories = [json.loads(l) for l in open(H / "data/stories.jsonl")]
n = 0
with ThreadPoolExecutor(6) as ex, open(H / "data/threads.jsonl", "w") as f:
    for s, out in ex.map(thread, stories):
        if out is None: continue
        for c in out: f.write(json.dumps(c) + "\n"); n += 1
print("comments", n)
