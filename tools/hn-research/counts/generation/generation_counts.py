"""Count generation themes and tool sentiment by period over the core subset (see generation_core.py).

Usage: python3 -I generation_counts.py <hn_dir>                      # tables
       python3 -I generation_counts.py <hn_dir> --sample THEME N [CHARS]  # random matches, for spot-checks
       python3 -I generation_counts.py <hn_dir> --grep REGEX N [CHARS]    # random core comments matching REGEX
A theme is a case-insensitive regex A, optionally with a regex B that must match within WINDOW characters of a
match of A (either side). A comment counts once per theme. Periods: 2016-19, 2020-22, 2023-26 (Nov 2016 - Nov 2026).
"""
import collections, os, random, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import generation_core as gen_core

BROWSER = r"puppeteer|playwright|headless|chrom(e|ium)|electron|browserless|gotenberg|\bbrowsers?\b"
NEG = (r"\bnot\b|n't\b|cannot|\black|missing|no (browser|support)|unsupported|\bdraft|\bgaps?\b|\bpoor|incomplete|"
       r"only prince|\bhack|\bpain|\bhard\b|difficult|problem|issue|struggl|nightmare|broken|wrong|ugly|\bfight|never|annoy")
PDFCTX = r"pdf|render|compil|generat|typeset|latex|typst|chrom|puppeteer|weasyprint|wkhtml|prince|engine|documents?|pages?\b"
T = {  # name: (A, B or None, window)
    # ---- wants
    "W page layout: breaks, headers/footers, page numbers, footnotes, TOC": (
        r"page[- ]?breaks?|break-(before|after|inside)|headers? (and|&) footers?|running (head|foot)|page numb|"
        r"counter\(page|numbered pages|footnotes?|widows?|orphans?|table of contents|\btoc\b|@page|paged[- ]media|"
        r"margin box|paginat", None, 0),
    "W HTML/CSS and web skills as the template language": (
        r"html ?(\+|and|&|/) ?css|css (and|\+|&) html|web (tech|technolog|stack|developers?|devs?)\b|front[- ]?end "
        r"(dev|code|skills|developers|tech)|tailwind|\breact\b|\bvue\b|svelte|(same|existing) (html|css|templates?|front)", None, 0),
    "W templates and data-driven documents": (
        r"templat|jinja|handlebars|liquid|mustache|razor|placeholder|mail merge|from (a |the )?(database|csv|json|yaml|"
        r"spreadsheet)|(json|yaml|csv) (data|files?|input)", None, 0),
    "W programmatic generation from code": (
        r"programmatic\w*|from code\b|in code\b|code[- ]first|scripting|scriptable|programmab|turing[- ]complete|"
        r"real programming language|\bapi\b", PDFCTX, 80),
    "W typographic quality: hyphenation, justification, kerning": (
        r"hyphenat|kerning|justif|microtyp|ligature|knuth|plass|line[- ]?break|typograph|letterspac|font features", None, 0),
    "W custom fonts, embedding, OpenType": (
        r"custom fonts?|(embed|subset)\w* (the |all |your )?fonts?|fonts? (are |is )?(embedd|subset)|web ?fonts?|opentype|"
        r"variable fonts?|font features|own fonts?|\bttf\b|\botf\b|woff|google fonts", None, 0),
    "W non-Latin scripts, RTL, shaping, emoji, Unicode": (
        r"unicode|\bcjk\b|chinese|japanese|korean|arabic|hebrew|persian|farsi|urdu|devanagari|hindi|sanskrit|\bthai\b|"
        r"cyrillic|emoji|right[- ]to[- ]left|\brtl\b|bidi|harfbuzz|shaping|complex script|non[- ]latin|non[- ]english", None, 0),
    "W speed and scale": (
        r"\bfast(er|est)?\b|\bspeed|millisecond|\d+ ?ms\b|per second|at scale|\bscal(e|ing|able)\b|thousands of|"
        r"millions of|throughput|latency|compile times?|instant(ly)?\b|performance|\bquick(ly)?\b", PDFCTX, 80),
    "W live preview and fast iteration": (
        r"live preview|hot[- ]?reload|watch mode|incremental (compil|render|layout|build)|wysiwyg|instant (preview|feedback|"
        r"compil)|real[- ]?time preview|preview (pane|window|as you)|as you type|side[- ]by[- ]side", None, 0),
    "W client-side, in-browser, WASM, offline": (
        r"client[- ]side|in[- ]the[- ]browser|in[- ]browser|browser[- ]based|\bwasm\b|webassembly|\boffline\b|no server|"
        r"local[- ]first|runs? locally|on[- ]device|jspdf|pdf-lib|pdfmake|react-pdf|typst\.ts", None, 0),
    "W accessibility and tagged PDF": (
        r"accessib|tagged|pdf/ua|pdf-ua|screen[- ]?readers?|a11y|wcag|section 508|alt[- ]text|reading order|\bblind\b", None, 0),
    "W PDF/A and archiving": (
        r"pdf/a|pdf-a\b|\bpdfa\b|archival|long[- ]term (archiv|preserv|storage|access)|archiv\w* (format|purposes|quality|"
        r"standard)", None, 0),
    "W e-invoicing: ZUGFeRD, Factur-X, XRechnung, Peppol": (
        r"zugferd|factur-?x|e-?invoic|xrechnung|peppol|en ?16931|\bubl\b", None, 0),
    "W selectable, searchable, vector output": (
        r"selectable|select(ing)? (the )?text|searchable (pdf|text)|text layer|copy[- ]?(and[- ]|&)?past\w* (from|text|out)|"
        r"ctrl[- ]?f|real text|text as (an? )?(images?|pictures?|bitmaps?)|vector (text|graphics|output|pdf)",
        r"pdf|text|output|generat|render|chrom|print", 60),
    "W links, bookmarks, outlines, cross-references": (
        r"hyperlinks?|clickable (links?|toc|table|references?)|internal links?|cross[- ]?ref\w*|pdf bookmarks?|bookmarks? "
        r"(in|pane|panel|sidebar)|\boutlines?\b (in|of) (the )?pdf|links? (in|inside|within|work in) (the |a |my )?pdfs?|"
        r"toc with (page numbers|links)|linkable|links (still )?work",
        r"pdf|document|toc|table of contents|latex|typst|generat|export|pandoc|chrom|print", 80),
    "W forms and interactive PDFs": (
        r"fillable|form fields?|acroforms?|fill(ing)? (out|in) (the |a )?(pdf )?forms?|pdf forms?|interactive (pdf|forms?|"
        r"elements)|form filling", None, 0),
    "W small footprint, few dependencies, single binary": (
        r"no dependenc|zero[- ]dependenc|without (any )?(external )?dependenc|dependency[- ]free|single (static )?binary|"
        r"one binary|self[- ]contained|light[- ]?weight|footprint|bundle size|binary size|install size|"
        r"(huge|large|massive|giant) (install|dependenc|download)|gigabytes? of|\d+ ?gb (install|download|of)", None, 0),
    "W deterministic, reproducible, testable output": (
        r"determinis|reproducib|byte[- ](for[- ]byte|identical)|pixel[- ]perfect|bit[- ]perfect|same output|"
        r"identical output|snapshot test|golden|regression test|visual diff|unit test|consistent (output|results|rendering)", None, 0),
    "W Markdown or other light markup as input": (
        r"markdown|\bmd\b|commonmark|asciidoc|\brst\b|restructuredtext|org[- ]mode", None, 0),
    "W maths": (r"\bmaths?\b|equations?|formula|mathjax|katex|mathml", None, 0),
    "W charts, SVG, diagrams, images": (
        r"\bsvg|charts?\b|\bplots?\b|diagrams?\b|tikz|\bd3\b|matplotlib|vector graphics|images? (in|into)|embed\w* images?|"
        r"graphs? (in|into)", None, 0),
    "W print production: CMYK, bleed, PDF/X": (
        r"cmyk|bleed|crop marks?|pdf/x|pdf-x|spot colou?r|print[- ]ready|prepress|press[- ]ready|offset print|\bicc\b", None, 0),
    "W tables that span pages": (
        r"tables?.{0,40}(split|span|break|across|over|multiple|several|continu).{0,20}pages?|repeat\w*.{0,20}(table )?"
        r"(header|thead)|\bthead\b|long tables?|multi-?page tables?", None, 0),
    # ---- complaints
    "C LaTeX pain: errors, packages, install size, compile passes": (
        r"\blatex|\btex\b|pdflatex|xelatex|lualatex",
        r"error messages?|cryptic|obscure errors?|compil\w*.{0,30}(slow|twice|three times|multiple times|passes|runs)|"
        r"(multiple|several|two|three) (passes|runs)|packages?.{0,20}(conflict|hell|incompat|clash|break)|"
        r"tex ?live.{0,40}(gb|gigabyte|huge|large|big|install)|(\bgb\b|gigabytes?).{0,40}(tex ?live|latex|install)|"
        r"aux files?|arcane|learning curve|\bhell\b|\bpain\b|painful|archaic|baroque|clunky|\b80s\b|1980s|frustrat|"
        r"nightmare|debugging", 150),
    "C cost and pricing": (
        r"expensive|costly|overpriced|pricey|pricing|price tag|\$ ?\d[\d,.]*k? ?(/|per|a|each)? ?(month|mo\b|year|yr\b|pdf|doc|"
        r"document|page|seat|server|licen)|per (pdf|document|page)\b|licen[cs]e (fee|cost)s?",
        r"pdf|api\b|prince|docraptor|itext|aspose|pspdfkit|apryse|adobe|acrobat|overleaf|weasy|onedoc|saas|service|"
        r"librar|engine|licen", 80),
    "C licensing: AGPL/GPL, licence terms": (
        r"\bagpl|\bgpl\b|licen[cs]\w*", r"pdf|itext|ghostscript|mupdf|prince|aspose|pspdfkit|apryse|closed[- ]source|"
        r"proprietary|commercial|fonts?\b|librar", 80),
    "C Typst gaps: journals, templates, maturity, maths syntax": (
        r"typst", r"journals?|conferences?|publishers?|arxiv|\baccept|not (yet|ready|mature)|immature|missing|\black|"
        r"microtyp|math (mode|syntax)|backslash|ecosystem", 150),
    "C generation is painful (generic)": (
        r"generat\w* (a |the )?pdfs?|pdf[- ]generat|html ?(to|2) ?pdf|creat\w* pdfs?|pdf librar|making pdfs|pdfs? from",
        r"\bpain|nightmare|\bhell\b|frustrat|headache|struggl|\bfight|wrestl|\bhate\b|\bhard\b|difficult|tricky|rabbit ?hole", 100),
    "C headless browser is heavy: memory, Lambda, Docker, startup": (
        BROWSER,
        r"memory|\bram\b|heavy|bloat|resource|lambda|serverless|cold[- ]?start|\bcpu\b|docker|container|leak|zombie|"
        r"crash|\bhangs?\b|\d+ ?(mb|gb)\b|\blayer\b|overhead|spin(ning|s)? up|huge dependency|massive|gigaton|"
        r"(whole|entire|full) (web )?browser|startup time", 120),
    "C slow generation or compiles": (
        r"\bslow(er|ly|ness)?\b|takes? (forever|ages|minutes)|sluggish",
        r"pdf generat|generat\w* (the |a )?pdfs?|render\w* (a |the )?(pdf|page)|compil\w*|latex|typst|chrome|chromium|"
        r"puppeteer|playwright|weasyprint|wkhtml|prince|headless|pandoc", 60),
    "C font problems: missing, not embedded, wrong": (
        r"\bfonts?\b", r"missing|not embedded|(isn|wasn|aren|weren)'?t embedded|tofu|garbled|wrong font|fallback|substitut|"
        r"(didn|don|doesn)'?t (render|show|display|work)|not (render|show|display|work)", 40),
    "C file size": (
        r"file ?size|bloated pdf|(pdf|output|file)s? (is |are |was |were |get |gets )?(huge|large|big|bloated|massive|"
        r"enormous)|(huge|large|big|bloated|massive|enormous) pdfs?|\d+ ?(kb|mb) (pdf|file)|pdfs? (of|at|around) \d+ ?(kb|mb)|"
        r"font compression|compress\w* (the )?(pdf|fonts?|images?|streams?)|subset\w* (the )?fonts?|font subset", None, 0),
    "C privacy of hosted PDF APIs (lane A only)": (
        r"privacy|sensitive|confidential|third[- ]part|on[- ]?prem|gdpr|hipaa|soc ?2\b|data (leaving|leave)|"
        r"send(ing)? (your |my |customer |the |our )?(data|documents?|invoices?)|own (servers?|infrastructure)",
        r"\bapi\b|service|saas|cloud|hosted|vendor|third[- ]part|upload|server", 100, "A"),
    "C security of rendering HTML or templates": (
        r"\bssrf|file:///?|local files?|untrusted (html|input|content|data|user)|sandbox\w*|(html|template|latex|tex) "
        r"injection|shell[- ]escape|write18|arbitrary (code|file)|exploit|\bcve\b",
        r"html|chrom|browser|headless|puppeteer|wkhtml|phantom|render|latex|\btex\b|template|url|pdf", 120),
    "C CSS paged media missing in browsers": (
        r"paged[- ]media|@page|css[- ]page|margin box|@top|@bottom|target-counter|string-set|running\(|css print|print css|"
        r"print styles?heet|@media print", NEG, 150),
    "C page breaks and pagination are hard": (r"page[- ]?breaks?|break-inside|pagination|paginat", NEG, 100),
    "C output changes across versions and environments": (
        r"(every|each|new) (release|update|version|upgrade)|after (an? )?(update|upgrade)|upgrad\w*.{0,40}(broke|chang|differ)|"
        r"broke\w*.{0,30}(layout|output)|different (output|results?|rendering)|output (changes|changed|differs)|\bdrift|"
        r"inconsisten|across (environments|machines|platforms|systems|versions|browsers)|works on my machine", PDFCTX, 150),
    "C wkhtmltopdf old, unmaintained or buggy": (
        r"wkhtml", r"\bold\b|outdated|ancient|abandon|unmaintained|no longer|maintain|deprecat|archived|qt ?webkit|\bdead\b|"
        r"flexbox|css grid|buggy|\bbugs?\b|security|vulnerab|stale|years", 200),
    "C text not selectable or garbled on copy": (
        r"selectable|select(ing)? (the )?text|copy[- ]?(and[- ]|&)?paste|copy (the )?text|searchable|ctrl[- ]?f",
        r"\bnot\b|n't\b|cannot|unusable|garbage|garbl|jumbl|broken|impossible|wrong|half|mess", 60),
    "C accessibility missing in generated PDFs": (
        r"accessib|tagged|pdf/ua|screen[- ]?readers?", NEG, 100),
}
TOOLS = {
    "wkhtmltopdf": r"wkhtml", "Headless Chrome/Puppeteer/Playwright": r"puppeteer|playwright|headless ?(chrome|chromium)|"
    r"chrome --headless|chromium --headless|print[- ]?to[- ]?pdf",
    "WeasyPrint": r"weasyprint", "Prince/DocRaptor": r"princexml|prince ?xml|docraptor|\bprince\b(?=[^.]{0,60}(pdf|css|html|"
    r"weasy|licen|engine))|(pdf|css|html|weasy|paged)[^.]{0,60}\bprince\b",
    "Paged.js": r"paged\.?js|pagedjs", "Gotenberg": r"gotenberg", "PhantomJS": r"phantomjs",
    "iText/OpenPDF": r"\bitext|openpdf", "ReportLab": r"reportlab", "PDFBox": r"pdfbox",
    "jsPDF/pdf-lib/pdfmake": r"jspdf|pdf-lib|pdfmake", "react-pdf": r"react-pdf|reactpdf|@react-pdf",
    "QuestPDF": r"questpdf", "FPDF/TCPDF/dompdf/mPDF": r"\bfpdf|tcpdf|dompdf|\bmpdf|gofpdf",
    "Apache FOP/XSL-FO": r"apache fop|\bfop\b|xsl-?fo", "LaTeX/TeX": r"\blatex|pdflatex|xelatex|lualatex|luatex|\btex\b",
    "Typst": r"typst", "Pandoc": r"pandoc", "groff/troff": r"groff|troff", "SILE": r"\bsile\b", "Quarto": r"quarto",
    "ConTeXt": r"ConTeXt",
}
POS = (r"\blove[ds]?\b|\bloving\b|\bgreat\b|excellent|fantastic|amazing|awesome|wonderful|"
       r"works? (really |very |pretty |quite )?(well|great|fine|nicely|like a charm|perfectly|beautifully)|rock[- ]solid|"
       r"reliable|happy with|recommend|the best|gold standard|breath of fresh air|game ?changer|pleasure|delight|"
       r"impressive|never look(ed)? back|never going back|painless|easy to use|so much (easier|better|nicer)|\bjoy\b")
NEGS = (r"\bhate[ds]?\b|\bpain(ful)?\b|buggy|broken|awful|terrible|horrible|nightmare|frustrat|outdated|abandon|deprecated|"
        r"unmaintained|\bslow\b|bloat|\bheavy\b|leak|crash|\bhell\b|disaster|regret|\bugly\b|clunky|arcane|cryptic|struggl|"
        r"headache|doesn'?t work|didn'?t work|janky|flaky|garbage|\bsucks?\b|\bmess\b|annoy")
PERIODS = ["2016-19", "2020-22", "2023-26"]


def matcher(a, b, window, lane=None):
    ra = re.compile(a, re.I)
    rb = re.compile(b, re.I) if b else None
    def hit(row):
        text = row["text"]
        if lane and not row["lane" + lane]:
            return False
        if rb is None:
            return bool(ra.search(text))
        for m in ra.finditer(text):
            if rb.search(text[max(0, m.start() - window): m.end() + window]):
                return True
        return False
    return hit


def main():
    root = sys.argv[1]
    rows = [r for r in gen_core.load(root) if r["core"]]
    base = collections.Counter(r["period"] for r in rows)
    width = int(sys.argv[sys.argv.index("--sample") + 3]) if "--sample" in sys.argv and len(sys.argv) > sys.argv.index("--sample") + 3 else 260
    if "--sample" in sys.argv or "--grep" in sys.argv:
        flag = "--sample" if "--sample" in sys.argv else "--grep"
        i = sys.argv.index(flag); key, n = sys.argv[i + 1], int(sys.argv[i + 2])
        width = int(sys.argv[i + 3]) if len(sys.argv) > i + 3 else 260
        if flag == "--sample":
            name = next(k for k in T if key.lower() in k.lower()); hit = matcher(*T[name])
        else:
            hit = matcher(key, None, 0)
        hits = [r for r in rows if hit(r)]
        random.seed(3)
        print(f"{len(hits)} matches")
        for r in random.sample(hits, min(n, len(hits))):
            print(r["id"], r["date"], "|", " ".join(r["text"].split())[:width])
        return
    print(f"core comments per period: {dict(base)}; total {len(rows)}; lane A "
          f"{dict(collections.Counter(r['period'] for r in rows if r['laneA']))}; lane B "
          f"{dict(collections.Counter(r['period'] for r in rows if r['laneB']))}")
    res = {}
    for name, spec in T.items():
        hit = matcher(*spec)
        c = collections.Counter(); ca = 0; threads = set()
        for r in rows:
            if hit(r):
                c[r["period"]] += 1; threads.add(r["story_id"]); ca += r["laneA"]
        res[name] = (sum(c.values()), len(threads), c, ca)
    print("\n| theme | comments | threads | 2016-19 | 2020-22 | 2023-26 | per 1,000 core comments (16-19 / 20-22 / 23-26) | of which lane A |")
    print("|---|---|---|---|---|---|---|---|")
    for name, (tot, th, c, ca) in sorted(res.items(), key=lambda kv: (kv[0][0], -kv[1][0])):
        per = " / ".join(f"{1000 * c[p] / base[p]:.1f}" for p in PERIODS)
        print(f"| {name} | {tot} | {th} | {c['2016-19']} | {c['2020-22']} | {c['2023-26']} | {per} | {ca} |")
    pos, neg = re.compile(POS, re.I), re.compile(NEGS, re.I)
    print("\n| tool | mentions 16-19 / 20-22 / 23-26 | positive cue % | negative cue % | net (pos-neg) |")
    print("|---|---|---|---|---|")
    for tool, rx in TOOLS.items():
        rx = re.compile(rx) if tool == "ConTeXt" else re.compile(rx, re.I)
        m, p, n = collections.Counter(), collections.Counter(), collections.Counter()
        for r in rows:
            if rx.search(r["text"]):
                m[r["period"]] += 1; p[r["period"]] += bool(pos.search(r["text"])); n[r["period"]] += bool(neg.search(r["text"]))
        f = lambda x, q: str(round(100 * x[q] / m[q])) if m[q] >= 15 else "-"
        net = lambda q: str(round(100 * (p[q] - n[q]) / m[q])) if m[q] >= 15 else "-"
        print(f"| {tool} | {' / '.join(str(m[q]) for q in PERIODS)} | {' / '.join(f(p, q) for q in PERIODS)} | "
              f"{' / '.join(f(n, q) for q in PERIODS)} | {' / '.join(net(q) for q in PERIODS)} |")


if __name__ == "__main__":
    main()
