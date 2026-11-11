"""Stricter theme rules: a comment counts when ONE SENTENCE holds both a documentation word
(docs, documentation, manual, readme, reference, man page, tutorial, guide, wiki) and the theme's cue.
Higher precision, lower recall than themes.py. Threads excluded per theme are listed in EXCLUDE
(stories whose subject shares the cue word but not the theme). Same denominator as themes.py.
Usage: python3 -I strict.py <workdir> [--samples THEME N SEED]"""
import json, random, re, sys
from collections import Counter
from pathlib import Path
H = Path(sys.argv[1]); I = re.I
C = [json.loads(l) for l in open(H / "data/threads.jsonl")]
DOC = re.compile(r"\b(docs?|documentation|documented|manuals?|readmes?|reference|man ?pages?|manpages?|tutorials?|guides?|wiki)\b", I)
SENT = re.compile(r"(?<=[.!?])\s+|\n+")
FOREX = re.compile(r"\bfor (an )?example\b|\be\.g\.|\bexample\.(com|org)", I)
CUE = {
 "examples": r"\bexamples\b|\bexample code|sample code|code samples?|\bsnippets?\b|working example|runnable example|usage example",
 "examples_dont_work": r"(examples?|snippets?|sample code)\b[^.]{0,60}\b(don'?t|doesn'?t|do not|does not|never|won'?t|didn'?t|fail\w*|broken|outdated|out of date|wrong|stale)\b[^.]{0,30}(work|run|compile|exist)?|broken examples?|(test|tested|testing|doctests?|compile)\w*[^.]{0,30}(examples|snippets)",
 "reference_vs_guides": r"auto-?generated|generated (api )?(docs|documentation)|\bjust (an? |the )?(api )?reference|reference[- ]only|tutorials?|how-?to|concepts?|conceptual|overview|big picture|getting started|quick ?start|the why\b|explain\w* why",
 "marketing": r"marketing|landing page|fluff|buzzword|sales|hero image|salesy|hype",
 "findability": r"\bsearch\w*|can'?t find|couldn'?t find|hard to find|find (anything|what|the)|navigat\w*|discoverab\w*|sidebar|table of contents|\btoc\b|index",
 "scattered": r"discord|slack|scattered|spread (across|over|out|between)|fragmented|github issues|issue tracker|blog posts?|forums?|mailing lists?|stack ?overflow",
 "versioning": r"versions?\b|versioned",
 "slow_heavy": r"\bslow\w*|sluggish|bloat\w*|heavy|laggy|javascript|\bjs\b|single[- ]page app|\bspa\b|load(s|ing)? (slow|forever)|spinner|megabytes|lightweight|fast\b",
 "offline_plain_single": r"offline|plain[- ]?text|single (html )?page|one (big|single|long|giant|huge|flat) (html )?page|\bpdfs?\b|print\w*|\bepub\b|terminal|\bgrep\b|local copy|downloadable",
 "interactive": r"interactive|playground|runnable|try (it|them|out)|\brepl\b|live (code|examples?|editor|demo)|in the browser|sandbox|notebooks?|livebook|jupyter|editable",
 "video": r"\bvideos?\b|youtube|screencasts?",
 "ui_details": r"copy (button|to clipboard|icon|paste)|dark (mode|theme)|syntax highlight\w*|anchor|permalinks?|deep links?|line length|font|keyboard|shortcut|layout|sticky|collapsible|breadcrumb|light (mode|theme)|contrast|design",
 "ai": r"\b(chatgpt|gpt-?[345o]?|llms?|copilot|claude|chat ?bots?|rag|hallucinat\w*|cursor|gemini|agents?)\b|\bAI\b",
 "source_code": r"source code|read(ing)? the (source|code)|the code\b|\bsource\b",
 "types_ide": r"\bide\b|intellisense|autocomplete|auto-complete|type (signatures?|hints?|definitions)|docstrings?|hover",
 "changelog": r"changelog|release notes|migration guide|upgrade guide|breaking changes?|what'?s new",
 "error_messages": r"error (messages?|codes?)",
 "screenshots_diagrams": r"screenshots?|diagrams?|illustrations?|images?|gifs?\b|visuals?",
 "stack_overflow": r"stack ?overflow",
}
CUE = {k: re.compile(v, 0 if k == "ai" else I) for k, v in CUE.items()}
EXCLUDE = {"slow_heavy": {"35201848"}}  # 'New React docs pretend SPAs don't exist': about SPA architecture, not docs speed
def hits(name, c):
    if c["story_id"] in EXCLUDE.get(name, ()): return False
    t = c["text"]
    if name.startswith("examples"): t = FOREX.sub(" ", t)
    return any(DOC.search(s) and CUE[name].search(s) for s in SENT.split(t))
def main():
    core = [c for c in C if c["set"] == "core"]
    if len(sys.argv) > 2:
        name, n, seed = sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
        pool = [c for c in core if hits(name, c)]; random.Random(seed).shuffle(pool)
        for c in pool[:n]: print(f"* {c['id']} {c['created_at'][:10]} [{c['story_title'][:45]}] " + " ".join(c["text"].split())[:300])
        return
    per = Counter("late" if c["created_at"] >= "2023" else "early" for c in core)
    print(f"denominator: core {len(core)} (2015-2022 {per['early']}, 2023-2026 {per['late']})")
    print(f"{'theme':22} {'all':>5} {'thr':>4} {'15-22':>6} {'%':>5} {'23-26':>6} {'%':>5}")
    res = {}
    for name in CUE:
        m = [c for c in core if hits(name, c)]
        p = Counter("late" if c["created_at"] >= "2023" else "early" for c in m)
        thr = len({c["story_id"] for c in m})
        res[name] = dict(all=len(m), threads=thr, early=p["early"], late=p["late"])
        print(f"{name:22} {len(m):5} {thr:4} {p['early']:6} {100*p['early']/per['early']:5.1f} {p['late']:6} {100*p['late']/per['late']:5.1f}")
    json.dump(res, open(H / "data/strict_counts.json", "w"), indent=1)
if __name__ == "__main__": main()
