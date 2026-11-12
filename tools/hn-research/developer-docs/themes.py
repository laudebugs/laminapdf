"""Theme counts over data/threads.jsonl. A comment counts for a theme when its text matches the theme's
regular expression (case-insensitive unless noted). 'for example'/'e.g.' are removed before the
examples rule. Denominator: every comment in the CORE threads (ADJ threads reported separately).
Periods: 2015-2022 and 2023-2026 by comment date.
Usage: python3 -I themes.py <workdir> [--samples THEME N [SEED]]"""
import json, random, re, sys
from collections import Counter, defaultdict
from pathlib import Path
H = Path(sys.argv[1])
C = [json.loads(l) for l in open(H / "data/threads.jsonl")]
I = re.I
DOCW = r"(docs?|documentation|manual|readme|reference|guide|site|page|website)"
THEMES = {
 "missing_outdated": re.compile(r"\b(outdated|out[- ]of[- ]date|stale (docs|documentation)|obsolete (docs|documentation)|undocumented|no (real |proper |official )?(docs|documentation)\b|lack of (docs|documentation)|missing (docs|documentation)|(docs|documentation) (is|are) (wrong|incorrect|missing|incomplete|lacking|nonexistent|non-existent|sparse|thin)|incomplete (docs|documentation)|(docs|documentation) (doesn'?t|don'?t|does not|do not) (exist|match|mention)|poorly documented|not documented|badly documented|under-?documented)", I),
 "examples": re.compile(r"(code |working |runnable |copy[- ]?paste(able)? |real[- ]world |more |good |concrete |usage |simple |minimal |complete |practical |worked |short )examples?|examples? (that|which) (don'?t|do not|doesn'?t|never) (work|run|compile)|example code|sample code|code samples?|examples (are|were) (the best|great|key|essential|king)|show me (an |the )?(example|code)|by example", I),
 "reference_only_needs_guides": re.compile(r"(just|only|merely|nothing but|little more than) (an? |the )?(api |auto-?generated )?reference|reference[- ]only|auto-?generated (api )?(docs|documentation|reference)|generated (api )?(docs|documentation)|list of (functions|methods|classes|endpoints)|(no|lack of|missing|without) (any )?(tutorials?|guides?|overview|conceptual|getting started|big picture|high[- ]level)|big picture|high[- ]level overview|how (it|they|things|the pieces|everything) fits? together|the \"?why\"?|repeat(s|ing)? the (method|function) names?|what,? not (the )?(why|how)", I),
 "diataxis_four_kinds": re.compile(r"di[aá]taxis|divio|four (kinds|types|quadrants) of (docs|documentation)|tutorials?, how-?to|explanation and reference|how-?to guides", I),
 "marketing_fluff": re.compile(r"marketing|landing page|sales ?pitch|buzzword|fluff|book a demo|contact sales|hero (image|section)|blazing(ly)? fast|enterprise[- ]grade|what (does|is) (it|this|the product) (actually )?do", I),
 "findability_search_nav": re.compile(r"\b(site|docs?|documentation)'?s? search|search (box|bar|function|feature|is (bad|terrible|useless|broken|awful|great|good))|\b(bad|poor|terrible|useless|broken|awful|good|great|fast|built-in|local|instant) search\b|navigat\w*|can'?t find|couldn'?t find|hard to find|impossible to find|findab\w*|discoverab\w*|where (is|are) the (docs|documentation)", I),
 "scattered_many_places": re.compile(r"scattered|spread (across|over|out|between)|fragmented|all over the place|(docs|documentation|information|answers?) (is|are|live|lives) (in|on) (discord|slack|github|blog|forums?|issues)|discord|slack (channel|community|workspace)|buried in|github issues|mailing list", I),
 "versioning": re.compile(r"version (mismatch|selector|switcher|drop-?down|picker|number in the url)|wrong version|(old|older|outdated|previous|latest|current|different) versions? of the (docs|documentation|api|library)|docs for (the |an )?(old|older|latest|wrong|current|previous) version|which version|version-?specific|for version \d|versioned (docs|documentation)", I),
 "slow_heavy_js": re.compile(r"\b(slow|sluggish|bloated|heavy|laggy)\b[^.\n]{0,60}\b" + DOCW + r"|\b" + DOCW + r"\b[^.\n]{0,60}\b(slow|sluggish|bloated|laggy|takes forever)\b|requires? (javascript|js)|without (javascript|js)|(javascript|js) (is )?(required|disabled|turned off|off)|noscript|single[- ]page app|\bspas?\b|loading spinner|megabytes of (javascript|js)", I),
 "man_pages": re.compile(r"\bman ?pages?\b|\bmanpages?\b|\binfo pages?\b|texinfo|`man |\bman \w+\(?\d?\)?\b(?<!man who)", I),
 "plaintext_offline_single_page": re.compile(r"plain[- ]?text|single (html )?page|one (big|single|long|giant|huge) (html )?page|all on one page|offline|pdf (version|manual|of the docs)|printable|print(ed)? (it|them|manual|out)|\bzeal\b|dash\.app|\bdash\b|devdocs|--help", I),
 "interactive_playground": re.compile(r"playground|interactive|runnable|try it|\brepl\b|live (code|examples?|editor|demo)|in[- ]browser|sandbox|jupyter|notebook|livebook|api explorer|swagger ui|executable examples?|run (it|the code|examples?) (in|from) the (browser|docs|page)", I),
 "ai_instead_of_docs": re.compile(r"(ask|asked|asking) (chatgpt|claude|gemini|an? llm|the llm|copilot|gpt|the ai|an ai|ai)\b|(chatgpt|claude|llms?|copilot|ai|gpt)\b[^.\n]{0,40}(instead of|rather than|replac\w*|better than) (reading |the |any )?(docs|documentation|manual|man pages?)|(don'?t|never|no longer|rarely|barely) (read|look at|open|check) (the )?(docs|documentation|manual)", I),
 "ai_distrust_hallucination": re.compile(r"hallucinat\w*|\b(ai|llms?|chat ?bots?|doc ?bots?|chatgpt|copilot|gpt|claude)\b[^.\n]{0,60}\b(wrong|lie|lies|lied|lying|made up|makes? (things|stuff|up)|inaccura\w*|incorrect|nonsense|bullshit)", I),
 "ai_generated_docs": re.compile(r"(ai|llm|gpt|chatgpt)[- ](generated|written) (docs|documentation|readmes?)|generat\w+ (the )?(docs|documentation) (with|using|by) (ai|llms?|gpt|chatgpt)|\bslop\b", I),
 "ai_llm": re.compile(r"\b(chatgpt|gpt-?[345o]?|llms?|copilot|claude|chatbot|chat ?bot|rag|hallucinat\w*|cursor|gemini|openai|anthropic|perplexity|vibe[- ]cod\w*|coding agents?|ai agents?|language models?)\b|\bAI\b", 0),
 "docs_for_agents_llmstxt": re.compile(r"llms?(-full)?\.txt|agents?\.md|claude\.md|cursor ?rules|\.cursorrules|(docs|documentation|written|write|writing) for (llms|agents|ai|machines|models)|machine[- ]readable|mcp server|\bmcp\b|markdown (version|twin|copy|export) of|context window", I),
 "video_vs_text": re.compile(r"\bvideos?\b|youtube|screencasts?|video tutorials?", I),
 "changelog_release_notes": re.compile(r"changelogs?|change logs?|release notes|migration guides?|upgrade guides?|breaking changes?", I),
 "error_messages_link_docs": re.compile(r"error (messages?|codes?)[^.\n]{0,100}(link|url|docs|documentation|explain)|(link|url)s? (to|in) [^.\n]{0,30}(docs|documentation)[^.\n]{0,40}error|--explain|error index|\bE0\d{3}\b", I),
 "ui_details": re.compile(r"copy (button|to clipboard|icon)|dark (mode|theme)|syntax highlight\w*|anchor links?|permalinks?|deep links?|line length|font size|(language|code) (switcher|tabs|toggle)|keyboard shortcuts?|three[- ]column|sticky header|collapsible|breadcrumb|light mode|contrast", I),
 "assumed_knowledge_tone": re.compile(r"\b(just|simply|easily|obviously|trivially)\b[^.\n]{0,40}\b(word|words|docs|documentation|say|saying|writing)|curse of knowledge|assumes? (you|the reader|prior|that you)|jargon|beginners?|newcomers?|newbies?|novices?", I),
 "read_the_source": re.compile(r"read(ing)? the (source|code)|source (code )?is the (only )?(documentation|docs)|use the source|code is the (documentation|docs)|dig(ging)? (through|into) the (source|code)", I),
 "stack_overflow": re.compile(r"stack ?overflow|\bSO\b", 0),
 "types_ide_as_docs": re.compile(r"type (signatures?|hints?|definitions|annotations)|autocomplete|auto-complete|intellisense|\.d\.ts|hover (docs|documentation|tooltips?)|docstrings?|in the ide|inline docs", I),
 "rtfm_reading_behaviour": re.compile(r"(nobody|no one|no-one|people don'?t|users don'?t|devs don'?t|developers don'?t|never) (actually )?reads?\b|\brtfm\b|\bskim(s|med|ming)?\b|cover to cover|read (the )?(docs|documentation|manual) (first|before)|tl;?dr", I),
 "screenshots_diagrams": re.compile(r"screenshots?|diagrams?|illustrations?|visuals?|gifs?\b", I),
 "docs_tooling": re.compile(r"sphinx|docusaurus|mkdocs|material for mkdocs|hugo|gitbook|mintlify|starlight|astro|readthedocs|read the docs|vitepress|jekyll|asciidoc|restructuredtext|\brst\b|docs[- ]as[- ]code|docs like code|readme\.io|redoc|swagger|openapi|typedoc|rustdoc|godoc|pkg\.go\.dev|hexdocs|writerside|confluence|notion", I),
}
SITES = {
 "Stripe": r"\bstripe\b", "Django": r"\bdjango\b", "Rust book / Rust docs": r"rust (book|docs|documentation)|the book\b|rust by example|\brustdoc\b", "docs.rs": r"docs\.rs",
 "MDN": r"\bmdn\b|developer\.mozilla", "PostgreSQL": r"postgres(ql)?", "Tailwind": r"tailwind", "Laravel": r"laravel",
 "FreeBSD handbook": r"freebsd", "OpenBSD": r"openbsd", "Arch wiki": r"arch ?wiki|archwiki|arch linux wiki|wiki\.archlinux", "SQLite": r"sqlite",
 "Python docs": r"python (docs|documentation)|docs\.python", "Go docs": r"\bgo (docs|documentation)|pkg\.go\.dev|godoc|go by example",
 "PHP docs": r"php\.net|php (docs|documentation|manual)", "Vue": r"\bvue(\.?js)?\b", "React docs": r"react (docs|documentation)|react\.dev",
 "Elixir/hexdocs": r"elixir|hexdocs", "Qt": r"\bqt\b", "Redis": r"\bredis\b", "Twilio": r"twilio", "Kubernetes": r"kubernetes|\bk8s\b",
 "Microsoft docs": r"msdn|docs\.microsoft|microsoft (docs|documentation)|learn\.microsoft", "Apple docs": r"apple('s)? (developer )?(docs|documentation)|developer\.apple",
 "AWS docs": r"aws (docs|documentation)|amazon('s)? (docs|documentation)", "Google Cloud docs": r"(gcp|google cloud) (docs|documentation)",
 "Java (javadoc)": r"javadoc", "Clojure": r"clojure", "Haskell/Hackage": r"hackage|haskell", "Ruby docs": r"ruby (docs|documentation)|ruby-doc",
 "jQuery": r"jquery", "Flask": r"\bflask\b", "FastAPI": r"fastapi", "Sidekiq/others": r"sidekiq", "Godot": r"godot", "Unity": r"\bunity\b",
 "Linux kernel docs": r"kernel (docs|documentation)", "GNU/info": r"\bgnu\b|texinfo|info pages?", "cppreference": r"cppreference", "Vulkan": r"vulkan",
 "Write the Docs": r"write the docs", "Divio/Diátaxis": r"di[aá]taxis|divio", "Supabase": r"supabase", "Zig": r"\bzig\b", "Elm": r"\belm\b",
 "Nix/NixOS": r"\bnix(os)?\b", "Gentoo wiki": r"gentoo", "Debian": r"debian", "Oxide": r"\boxide\b", "Kagi": r"\bkagi\b", "Twitter API": r"twitter api",
 "Scala": r"\bscala\b", "Angular": r"angular", "Svelte": r"svelte", "Rails guides": r"rails guides?|guides\.rubyonrails|ruby on rails", "Ansible": r"ansible", "Terraform": r"terraform",
 "Hugo": r"\bhugo\b", "Swift/SwiftUI": r"swiftui|\bswift\b", "Unreal": r"unreal", "Nginx": r"nginx", "Git": r"\bgit (docs|documentation|book)|pro git|git-scm",
}
SITES = {k: re.compile(v, I) for k, v in SITES.items()}
POS = re.compile(r"\b(great|excellent|best|love[sd]?|good|fantastic|amazing|gold standard|superb|wonderful|well[- ]written|clear|brilliant|outstanding|exemplary|top[- ]notch|stellar|model|praise|favou?rite|incredible|beautiful|helpful)\b", I)
NEG = re.compile(r"\b(bad|terrible|awful|worst|horrible|poor|useless|lacking|missing|outdated|confusing|hate|sucks?|garbage|abysmal|atrocious|crap|crappy|frustrat\w*|painful|piss-poor|nightmare|inadequate)\b", I)
SENT = re.compile(r"(?<=[.!?\n])\s+")
FOREX = re.compile(r"\bfor (an )?example\b|\be\.g\.", I)
def year(c): return int(c["created_at"][:4])
def period(c): return "2023-2026" if year(c) >= 2023 else "2015-2022"
DOCCTX = re.compile(r"\b(docs?|documentation|manual|readme|reference|man pages?|tutorials?|guides?)\b", I)
def match(name, c):
    t = c["text"]
    if name == "examples":
        t = FOREX.sub(" ", t)
        return bool(THEMES[name].search(t) and DOCCTX.search(t))
    return bool(THEMES[name].search(t))
def main():
    core = [c for c in C if c["set"] == "core"]; adj = [c for c in C if c["set"] == "adj"]
    if len(sys.argv) > 2 and sys.argv[2] == "--samples":
        name, n = sys.argv[3], int(sys.argv[4]); seed = int(sys.argv[5]) if len(sys.argv) > 5 else 1
        pool = [c for c in core if match(name, c)] if name in THEMES else [c for c in core if SITES[name].search(c["text"])]
        random.Random(seed).shuffle(pool)
        for c in pool[:n]:
            print(f"--- {c['id']} {c['created_at'][:10]} [{c['story_title'][:50]}]\n{c['text'][:700]}\n")
        return
    per = Counter(period(c) for c in core); stories = defaultdict(set)
    for c in core: stories[period(c)].add(c["story_id"])
    print(f"CORE comments {len(core)} in {len({c['story_id'] for c in core})} threads; dates {min(c['created_at'] for c in core)[:10]}..{max(c['created_at'] for c in core)[:10]}")
    print(f"  by period: {dict(per)}; threads by period (comment dates): { {k: len(v) for k, v in stories.items()} }")
    print(f"ADJ comments {len(adj)} in {len({c['story_id'] for c in adj})} threads\n")
    print(f"{'theme':32} {'all':>6} {'%':>5} {'thr':>4} | {'15-22':>6} {'%':>5} | {'23-26':>6} {'%':>5} | adj")
    out = {}
    for name in THEMES:
        m = [c for c in core if match(name, c)]
        a = sum(1 for c in adj if match(name, c))
        p = Counter(period(c) for c in m)
        thr = len({c["story_id"] for c in m})
        out[name] = {"all": len(m), "threads": thr, "early": p["2015-2022"], "late": p["2023-2026"], "adj": a}
        print(f"{name:32} {len(m):6} {100*len(m)/len(core):5.1f} {thr:4} | {p['2015-2022']:6} {100*p['2015-2022']/per['2015-2022']:5.1f} | {p['2023-2026']:6} {100*p['2023-2026']/per['2023-2026']:5.1f} | {a}")
    print("\nSITES: comments mentioning (all core+adj), threads, sentences with name+positive word, name+negative word, comments in 2023-2026")
    rows = []
    for k, rx in SITES.items():
        m = [c for c in C if rx.search(c["text"])]
        pos = neg = 0
        for c in m:
            ss = [s for s in SENT.split(c["text"]) if rx.search(s)]
            if any(POS.search(s) for s in ss): pos += 1
            if any(NEG.search(s) for s in ss): neg += 1
        rows.append((len(m), k, len({c['story_id'] for c in m}), pos, neg, sum(1 for c in m if year(c) >= 2023)))
    for n, k, t, pos, neg, late in sorted(rows, reverse=True):
        print(f"{k:24} {n:5} thr {t:3}  pos {pos:4}  neg {neg:4}  23-26 {late:4}")
    json.dump(out, open(H / "data/theme_counts.json", "w"), indent=1)
if __name__ == "__main__": main()
