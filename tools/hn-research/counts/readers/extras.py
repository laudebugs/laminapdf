# Signals beyond the theme counts: Acrobat + AI / sign-in in the full comment set (deduplicated),
# dark-mode and reflow stories, and points of AI-flavoured reader Show HNs.
import json,os,re,collections
HN=os.environ.get("HN_DATA", ".")
def period(d):
    y=int(d[:4]); return '2016-19' if y<=2019 else ('2020-22' if y<=2022 else '2023-26')
seen={}
for f in ['data/comment_hits.jsonl','data/threads.jsonl']:
    for l in open(os.path.join(HN,f)):
        o=json.loads(l)
        if o.get('text') and o['id'] not in seen: seen[o['id']]=(o['created_at'][:10],o['text'])
print('unique comments',len(seen),dict(collections.Counter(period(d) for d,_ in seen.values())))
pats={
 'acrobat+AI': r"(acrobat|adobe reader|adobe('s)? (pdf )?(reader|viewer))[^\n]{0,200}\b(ai assistant|ai toolbar|ai features?|ai chat|copilot|generative ai|ai)\b|\b(ai assistant|ai toolbar|ai features?)\b[^\n]{0,200}(acrobat|adobe reader)",
 'acrobat+signin/subscription': r"(acrobat|adobe reader|reader dc)[^\n]{0,150}\b(sign[- ]?in|log[- ]?in|subscription|upsell|account)\b|\b(sign[- ]?in|log[- ]?in|subscription|upsell)\b[^\n]{0,150}(acrobat|adobe reader|reader dc)",
}
for k,p in pats.items():
    rx=re.compile(p,re.I); print(k,dict(collections.Counter(period(d) for d,t in seen.values() if rx.search(t))))
rows=[json.loads(l) for l in open(os.path.join(HN,'data','stories.jsonl'))]
pdf=lambda o: re.search(r'(?i)pdf',(o['title'] or '')+(o.get('url') or ''))
for name,p in [('dark-mode stories',r'(?i)(dark[- ]mode|night mode|invert|darken)'),('reflow stories',r'(?i)(reflow|small[- ]screen|liquid mode|e-?readers?)')]:
    rx=re.compile(p); print(name,dict(collections.Counter(period(o['created_at']) for o in rows if rx.search(o['title'] or '') and pdf(o))))
sh=re.compile(r'(?i)^show hn.*\b(pdf (reader|viewer)|reader|viewer)\b'); ai=re.compile(r'(?i)\b(ai|chat|mcp|llm|agentic|gpt|claude|ollama|q&a|ask)\b')
S=[o for o in rows if sh.search(o['title'] or '') and pdf(o)]
print('reader/viewer Show HN stories',dict(collections.Counter(period(o['created_at']) for o in S)))
A=sorted(o['points'] for o in S if o['created_at']>='2023' and ai.search(o['title'])); N=sorted(o['points'] for o in S if o['created_at']>='2023' and not ai.search(o['title']))
print('2023-26 AI-flavoured:',len(A),'median',A[len(A)//2],'max',A[-1],'| others:',len(N),'median',N[len(N)//2],'max',N[-1])
