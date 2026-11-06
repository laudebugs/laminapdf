# Checks every quote in quotes.py is an exact substring of its comment and at most 40 words.
import json,os,sys
HERE=os.path.dirname(os.path.abspath(__file__)); HN=os.environ.get("HN_DATA", ".")
sys.path.insert(0,HERE)
from quotes import Q
T={}; D={}
for f,dk in [('areas/readers.jsonl','date'),('data/comment_hits.jsonl','created_at'),('data/threads.jsonl','created_at')]:
    for l in open(os.path.join(HN,f)):
        o=json.loads(l)
        if o.get('text'): T.setdefault(int(o['id']),[]).append(o['text']); D[int(o['id'])]=o[dk][:10]
bad=0
for tag,i,q in Q:
    ok=any(q in t for t in T.get(i,[])); w=len(q.split())
    if not ok or w>40: bad+=1; print('FAIL' if not ok else 'LONG',tag,i,w,q[:70])
print(f'checked {len(Q)} quotes, {bad} failed')
if len(sys.argv)>1:
    for tag,i,q in Q: print(tag,i,D.get(i))
