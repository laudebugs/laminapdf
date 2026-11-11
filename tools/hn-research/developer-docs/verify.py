"""Check every quoted line ('- "…" — [id](…item?id=N), YYYY-MM-DD') in a Markdown file against the comment
fetched again from the HN items API: verbatim after whitespace and curly-quote normalisation, <= 40 words,
date matches. Usage: python3 -I verify.py FILE.md"""
import html, json, re, ssl, sys, urllib.request
CTX = ssl.create_default_context(); TAGS = re.compile(r"<[^>]+>")
TYPO = str.maketrans({"’": "'", "‘": "'", "“": '"', "”": '"'})
def norm(s): return re.sub(r"\s+", " ", s.translate(TYPO)).strip()
LINE = re.compile(r'"(?P<q>.+?)" — \[(?P<id>\d+)\]\(https://news\.ycombinator\.com/item\?id=(?P=id)\)(?:, (?P<d>\d{4}-\d\d-\d\d))?')
bad = 0; n = 0
for line in open(sys.argv[1]):
    for m in LINE.finditer(line):
        n += 1
        i = m["id"]
        with urllib.request.urlopen(f"https://hn.algolia.com/api/v1/items/{i}", context=CTX, timeout=60) as r: it = json.load(r)
        text = html.unescape(TAGS.sub("", (it.get("text") or "").replace("<p>", "\n\n")))
        probs = []
        if norm(m["q"]) not in norm(text): probs.append("not verbatim")
        if len(m["q"].split()) > 40: probs.append("over 40 words")
        if m["d"] and (it.get("created_at") or "")[:10] != m["d"]: probs.append(f"date {it.get('created_at','')[:10]}")
        if it.get("type") != "comment": probs.append("not a comment")
        if probs: bad += 1; print("FAIL", i, probs, m["q"][:60])
print(f"{n} quotes checked, {bad} problems"); sys.exit(1 if bad else 0)
