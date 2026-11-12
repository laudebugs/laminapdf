"""Hand-coded nominations: every top-level comment in five 'best documentation' Ask HN threads
(17399340, 17905919, 32794330, 33682599, 34820382; 215 top-level comments, June 2018 to Feb 2023) was read,
and each comment that names a docs set as good is listed under that set. One comment can name several.
Prints counts. Usage: python3 -I nominations.py <workdir>"""
import json, sys
from pathlib import Path
N = {
 "Stripe": [17405141, 17406329, 17406952, 17906181, 17906574, 17906647, 17907479, 32794798, 32795204, 32805620, 33682803, 33683170, 33683680, 33700254, 34821716, 34840112],
 "Django": [17406189, 17406198, 17407821, 17910817, 32795354, 33682987, 34821869, 34821883, 34822041],
 "PostgreSQL": [17404867, 17406352, 17406422, 17907465, 32794760, 33682814, 34821707, 34822041],
 "Emacs / Elisp / Org manuals": [17404867, 17406139, 17406410, 32794723, 32794963, 34821921, 34822027, 34822730],
 "Scientific Python (scikit-learn, NumPy, SciPy, pandas)": [17406357, 17407821, 33682981, 33683337, 33683667, 34831374],
 "MATLAB / MathWorks": [17409399, 17906495, 33683109, 34821879, 34821895, 34822041],
 "MDN": [17406117, 17406581, 17407821, 33683252, 34821327],
 "Rust (book, std, compiler errors)": [17406081, 33683223, 33683549, 34822230, 34835518],
 "BSD docs and man pages (FreeBSD, OpenBSD, NetBSD)": [32794739, 32795059, 33682951, 34822680, 34840814],
 "Qt": [17406328, 17408224, 17905944, 32794917, 33682840],
 "Python standard library docs": [17905944, 33682682, 33683547, 34822117],
 "Python Requests": [17407243, 17407821, 17906200, 17907964],
 "SQLite": [17406368, 17907143, 33684041, 34823456],
 "Vue": [17406138, 32794646, 33685864],
 "Redis": [17406200, 33682996, 34821775],
 "cppreference": [17406482, 17905944, 32794610],
 "Arch wiki": [33682837, 34822884, 34823456],
 "Vim": [17404867, 33685695, 34833162],
 "Wolfram / Mathematica": [33683109, 34825053, 34832609],
 "Diátaxis / Divio (as a framework)": [33683697, 34821758, 34821801],
 "Twilio": [17406376, 17906574],
 "Tailwind": [32794733, 34822161],
 "Laravel": [17405141, 34822118],
 "Elasticsearch": [17406399, 34822100],
 "Lua manual (one page)": [17907954, 33684554],
 "ZeroMQ guide": [17406141, 34820740],
 "FastAPI": [33683078, 34821766],
 "Microsoft (Win32, learn.microsoft.com)": [32794577, 34820985],
 "Perl man pages": [33683254, 34822016],
 "Kubernetes": [33683547, 34822330],
 "AWS": [33683170, 33683223],
 "Slack API": [33692292, 34821729],
 "PHP manual": [32795098, 33682816],
 "Racket": [17411330, 33683138],
 "Common Lisp HyperSpec": [17911963, 33689051],
 "Old printed manuals (C64, VIC-20, DEC VMS, Coherent, Tandem, Win 3.1 SDK, RISC OS, 3DO, OS/2, QBASIC, mIRC, VB help, Inside Macintosh, TeXbook, K&R)":
   [17408861, 32795087, 17906555, 17906026, 33682744, 33682956, 32794894, 32794922, 17921583, 34828884, 33682967, 32806869, 32794556, 32795127, 32796156, 17406238, 32794895, 17406045],
}
H = Path(sys.argv[1])
B = {"17399340", "17905919", "32794330", "33682599", "34820382"}
top = {c["id"] for c in map(json.loads, open(H / "data/threads.jsonl")) if c["story_id"] in B and c["depth"] == 0}
bad = [i for v in N.values() for i in v if i not in top]
assert not bad, bad
print("top-level comments:", len(top))
for k, v in sorted(N.items(), key=lambda kv: -len(kv[1])):
    print(f"{len(set(v)):3}  {k}")

# Reasons: regex over the same 215 top-level comments ("example" includes "for example").
import re
R = {"examples": r"example|sample code|code samples?|snippet", "concepts/why/overview": r"concept|theory|how it all fits|explain|overview|what the program actually does|architecture",
     "search/find/navigation": r"search|easy to find|find what|navigat", "interactive/try it": r"interactive|try (the|it)|playground|in the browser|right in the browser|on the site itself",
     "offline/built-in": r"offline|local copy|built[- ]in|baked into|no internet|right in the program|in front of me", "links to source": r"source code|implementation|code symbol",
     "versions": r"version", "single page": r"single page|one flat page|page:all|all the pages at once|everything in one place"}
T = [c for c in map(json.loads, open(H / "data/threads.jsonl")) if c["story_id"] in B and c["depth"] == 0]
for k, v in R.items():
    print(f"reason {k}: {sum(1 for c in T if re.search(v, c['text'], re.I))}")
