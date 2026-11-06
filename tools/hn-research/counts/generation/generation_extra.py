"""Extra counts for the generation findings: tool mentions per year, and a few cross-cutting signals by period.
Usage: python3 -I generation_extra.py <hn_dir>
"""
import collections, re, sys
sys.path.insert(0, __file__.rsplit("/", 1)[0])
import generation_core as gen_core
from generation_counts import TOOLS, matcher, PERIODS

rows = [r for r in gen_core.load(sys.argv[1]) if r["core"]]
if "--sample" in sys.argv:
    import random
    key, n = sys.argv[sys.argv.index("--sample") + 1], int(sys.argv[sys.argv.index("--sample") + 2])
    SAMPLE = (key, n)
else:
    SAMPLE = None
years = [str(y) for y in range(2016, 2027)]
print("| tool | " + " | ".join(years) + " |")
print("|---|" + "---|" * len(years))
for tool in ["wkhtmltopdf", "Headless Chrome/Puppeteer/Playwright", "WeasyPrint", "Prince/DocRaptor", "Paged.js",
             "iText/OpenPDF", "jsPDF/pdf-lib/pdfmake", "LaTeX/TeX", "Typst", "Pandoc"]:
    rx = re.compile(TOOLS[tool], re.I)
    c = collections.Counter(r["date"][:4] for r in rows if rx.search(r["text"]))
    print(f"| {tool} | " + " | ".join(str(c[y]) for y in years) + " |")

base = collections.Counter(r["period"] for r in rows)
SIGNALS = {
    "Typst used to generate documents from data or in production (invoices, reports, JSON, API, server)": (
        r"typst", r"invoic|reports? (generation|from)|in production|production|\bjson\b|yaml|\bapi\b|backend|back[- ]end|"
        r"server|on the fly|generat\w* (pdfs|documents|reports|invoices)|saas|programmatic", 100),
    "AI/LLM/agent near a generation term": (
        r"\bllms?\b|\bgpt|chatgpt|\bclaude\b|\bagents?\b|agentic|\bmcp\b|copilot|vibe[- ]?cod|gemini|\bai\b",
        r"generat|pdfs?\b|typst|latex|document|template|layout|typeset", 80),
    "'just use the browser's print to PDF' advice": (
        r"print[- ]?to[- ]?pdf|save as pdf|print dialog|ctrl[- ]?p\b|browser'?s? print|print (it )?from (the |your )?browser|"
        r"window\.print", None, 0),
    "PDF/UA or tagged PDF named explicitly": (r"pdf/ua|pdf-ua|tagged pdf|tagged output|tag tree|structure tree", None, 0),
    "browser engine replacement: own layout engine, no browser": (
        r"(no|without( a)?|zero)[- ](headless )?browser|browser[- ]?(less|free)|own (layout|rendering) engine|"
        r"from[- ]scratch (layout|rendering|html) engine|custom (layout|rendering) engine|not (use|using) (a )?(headless )?"
        r"(browser|chrom)", None, 0),
}
print("\n| signal | 2016-19 | 2020-22 | 2023-26 | per 1,000 core (16-19 / 20-22 / 23-26) |")
print("|---|---|---|---|---|")
for name, spec in SIGNALS.items():
    hit = matcher(*spec)
    if SAMPLE and SAMPLE[0].lower() in name.lower():
        hits = [r for r in rows if hit(r)]
        random.seed(5)
        for r in random.sample(hits, min(SAMPLE[1], len(hits))):
            print(r["id"], r["date"], "|", " ".join(r["text"].split())[:150])
    c = collections.Counter(r["period"] for r in rows if hit(r))
    print(f"| {name} | {c['2016-19']} | {c['2020-22']} | {c['2023-26']} | "
          + " / ".join(f"{1000 * c[p] / base[p]:.1f}" for p in PERIODS) + " |")

# unions and one extra complaint
from generation_counts import T, NEG
union = [matcher(*T["C CSS paged media missing in browsers"]), matcher(*T["C page breaks and pagination are hard"])]
c = collections.Counter(r["period"] for r in rows if any(h(r) for h in union))
print(f"\nunion paged media missing OR page breaks hard: {sum(c.values())} {dict(c)}")
nonlatin_bad = matcher(T["W non-Latin scripts, RTL, shaping, emoji, Unicode"][0], NEG + r"|tofu|garbl|squish|isolated|dotted circle|"
                       r"wrong|malformed|atrocious|hell", 80)
hits = [r for r in rows if nonlatin_bad(r)]
c = collections.Counter(r["period"] for r in hits)
print(f"non-Latin/Unicode near a problem word: {len(hits)} {dict(c)}")
if "--sample-nl" in sys.argv:
    import random
    random.seed(9)
    for r in random.sample(hits, 10):
        print(r["id"], r["date"], "|", " ".join(r["text"].split())[:150])

# unions used in the findings
def union(names, extra=()):
    hs = [matcher(*T[n]) for n in names] + list(extra)
    hits = [r for r in rows if any(h(r) for h in hs)]
    c = collections.Counter(r["period"] for r in hits)
    return len(hits), len({r["story_id"] for r in hits}), dict(c)
print("union PDF/A + print production + e-invoicing:", union(["W PDF/A and archiving", "W print production: CMYK, bleed, PDF/X",
      "W e-invoicing: ZUGFeRD, Factur-X, XRechnung, Peppol"]))
print("union non-Latin problem + font problems:", union(["C font problems: missing, not embedded, wrong"], [nonlatin_bad]))
print("union security + privacy:", union(["C security of rendering HTML or templates", "C privacy of hosted PDF APIs (lane A only)"]))
print("union paged media + page breaks (with threads):", union(["C CSS paged media missing in browsers", "C page breaks and pagination are hard"]))
print("union page layout + tables span pages:", union(["W page layout: breaks, headers/footers, page numbers, footnotes, TOC", "W tables that span pages"]))
