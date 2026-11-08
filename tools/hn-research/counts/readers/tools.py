import json,re,collections,random,sys
TOOLS={
'Acrobat/Adobe Reader': (r"\b(acrobat|adobe reader|acroread|reader dc|adobe('s)? (pdf )?(reader|viewer)|adobe pdf)\b",re.I),
'macOS Preview': (r"(\bPreview\.app\b|\bPreview\b(?! (pane|release|build|version|feature|mode|window))|(macos|mac os|os x|osx|apple|mac)('s)? preview\b)",0),
'SumatraPDF': (r"\bsumatra\w*",re.I),
'pdf.js / Firefox viewer': (r"(pdf\.?js|pdfjs|firefox('s)? (built[- ]in |builtin |embedded |integrated |native |own |internal )?(pdf )?(viewer|reader|renderer|editor))",re.I),
'Chrome/Edge viewer + PDFium': (r"(pdfium|(chrome|chromium|edge)('s)? (built[- ]in |builtin |embedded |integrated |native |own |internal |new |old )?(pdf )?(viewer|reader|renderer)|\bedge\b[^.]{0,40}\bpdf)",re.I),
'Okular': (r"\bokular\b",re.I),
'Zathura': (r"\bzathura\b",re.I),
'Evince': (r"\b(evince|gnome (document viewer|papers))\b",re.I),
'Foxit': (r"\bfoxit\b",re.I),
'Skim': (r"\bskim(\.app)?\b(?= (is|for|on|has|does|can|app|was|handles|crash\w*|works))|\bskim\.app\b",re.I),
'PDF Expert': (r"\bpdf expert\b",re.I),
'MuPDF': (r"\bmupdf\b",re.I),
'Sioyek': (r"\bsioyek\b",re.I),
'Xodo': (r"\bxodo\b",re.I),
'PDF-XChange': (r"\b(pdf[- ]?xchange|xchange)\b",re.I),
'KOReader': (r"\bkoreader\b",re.I),
'reMarkable': (r"\bremarkable( ?2| paper pro)?\b(?! (job|work|achievement|thing|feat|how|that))",re.I),
'Zotero reader': (r"\bzotero\b",re.I),
'Google Scholar PDF Reader': (r"(scholar pdf reader|google scholar('s)? (pdf )?(reader|extension)|scholar reader)",re.I),
}
POS=re.compile(r"\b(love[sd]?|great|excellent|fantastic|amazing|awesome|best|favou?rite|wonderful|superb|brilliant|perfect(ly)?|fast(er|est)?|snappy|lightweight|solid|reliable|recommend\w*|happy with|works (well|great|fine|perfectly)|good|nice|pleasant|impressive|just works)\b",re.I)
NEG=re.compile(r"\b(hate[sd]?|terrible|awful|horrible|worst|garbage|trash|crap\w*|sucks?|slow(er|ly)?|bloat\w*|buggy|crash\w*|annoying|painful|bad|broken|abysmal|insecure|nightmare|frustrat\w*|unusable|hostile|shit\w*|clunky|sluggish|junk|mess|pain)\b",re.I)
def period(d):
    y=int(d[:4]); return '2016-19' if y<=2019 else ('2020-22' if y<=2022 else '2023-26')
import os
HN=os.environ.get("HN_DATA", ".")
src=os.path.join(HN,'areas','readers.jsonl')
rows=[json.loads(l) for l in open(src)]
out={}; ex=collections.defaultdict(list)
for t,(r,f) in TOOLS.items():
    rx=re.compile(r,f)
    st={p:collections.Counter() for p in ['2016-19','2020-22','2023-26']}
    for o in rows:
        txt=o['text']; ms=list(rx.finditer(txt))
        if not ms: continue
        p=period(o['date']); st[p]['n']+=1
        pos=neg=False
        for m in ms:
            w=txt[max(0,m.start()-150):m.end()+150]
            pos|=bool(POS.search(w)); neg|=bool(NEG.search(w))
        st[p]['pos']+=pos; st[p]['neg']+=neg
        st[p]['posonly']+=pos and not neg; st[p]['negonly']+=neg and not pos
        ex[t].append((o['id'],o['date'],pos,neg,txt[max(0,ms[0].start()-120):ms[0].end()+160].replace('\n',' ')))
    out[t]=st
print(f"{'tool':30s} "+"  ".join(f"{p:>22s}" for p in ['2016-19','2020-22','2023-26']))
print(f"{'':30s} "+"  ".join(f"{'n  +only -only net':>22s}" for p in range(3)))
for t,st in sorted(out.items(),key=lambda x:-sum(s['n'] for s in x[1].values())):
    cells=[]
    for p in ['2016-19','2020-22','2023-26']:
        s=st[p]; n=s['n']; net=(s['posonly']-s['negonly'])/n if n else 0
        cells.append(f"{n:4d} {s['posonly']:4d} {s['negonly']:4d} {net:+.2f}")
    print(f"{t:30s} "+"  ".join(f"{c:>22s}" for c in cells))

