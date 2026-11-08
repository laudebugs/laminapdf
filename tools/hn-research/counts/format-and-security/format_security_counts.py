"""Theme counts for the format, security and trust note (docs/research/hacker-news/notes/format-and-security.md).

Run:  HN_DATA=<dir> python3 -I tools/hn-research/counts/format-and-security/format_security_counts.py            # the table
                    HN_DATA=<dir> python3 -I tools/hn-research/counts/format-and-security/format_security_counts.py --sample-all N [SEED]   # N random hits per theme
                    HN_DATA=<dir> python3 -I tools/hn-research/counts/format-and-security/format_security_counts.py --sample THEME N [SEED]

Universe: every unique comment in data/threads.jsonl + data/comment_hits.jsonl with at least 40
characters that sits in a PDF story (data/stories.jsonl) or names PDF or a PDF tool (prep.py's rule),
minus job threads ("who wants to be hired", "who is hiring", "freelancer?").

Matching: URLs are removed from the text first (a link ending in .pdf says nothing about PDF).
A theme matches a comment when its first pattern (the anchor) matches, every further pattern
matches within 150 characters of that anchor, no exclusion matches, and, by the theme's anchor
level, a PDF name (PDF, a PDF tool, PDF/A, PostScript ...) occurs within 150 characters of the
anchor (level 2), or the thread title names one (level 1 also accepts that), or neither is needed (0).
All patterns are case-insensitive. Periods: P1 2016-2019 (Nov 2016 on), P2 2020-2022, P3 2023-2026
(to early Nov 2026). Rates are per 1,000 comments of the universe in that period.
"""

import json
import os
import random
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(os.environ.get("HN_DATA", "."))
PDF = re.compile(r"\bpdfs?\b|pdf\.js|pdfjs|pdfium|mupdf|poppler|acrobat|ghostscript|wkhtmltopdf|weasyprint|typst|"
                 r"sumatra|okular|zathura|foxit|pdfbox|itext|qpdf|pdftk|docling|pdfplumber|pymupdf", re.I)
JOBS = re.compile(r"who wants to be hired|who is hiring|freelancer\?", re.I)
URL = re.compile(r"https?://\S+|www\.\S+|\S+\.pdf\b", re.I)
PDFTXT = re.compile(r"\bpdfs?\b|pdf\.js|pdfjs|pdfium|mupdf|poppler|acrobat|adobe reader|ghostscript|foxit|itext|"
                    r"qpdf|pdftk|pymupdf|pdf/a|pdf/ua|postscript|preview\.app|sumatra|okular|evince|xpdf", re.I)
WINDOW = 150

# JavaScript or other active content said to live inside PDF files (not JS libraries that make PDFs)
JSPDF = (r"(javascript|\bjs\b)( code| scripts?)? (in|inside|within|embedded in|into) (a |the |your )?pdfs?|"
         r"pdfs? (files? )?(can |could |may |might |that |which |also |even |still )*(contain|run|execute|support|embed|have|include|carry|with|allow)\w* (embedded |arbitrary |some )?(javascript|\bjs\b|scripts?|code)|"
         r"(acrobat|adobe reader|pdf (readers?|viewers?|spec|specification|standard|format|forms?))\b.{0,50}\b(javascript|\bjs\b)\b|"
         r"(javascript|\bjs\b) (engine|interpreter|support|api)\b.{0,50}\b(pdfs?|acrobat|reader)\b|\bxfa\b|/openaction|/javascript\b|launch actions?|"
         r"pdfs?\b.{0,50}\b(embedded )?(flash|3d models?|3d objects?|rich media|movies)\b")
LONG = (r"archiv(al|ing|ed|e format|e quality|ists?)|long[- ]term|preserv(e|ed|ing|ation)|\bdecades\b|years from now|"
        r"(in|for|after) (\d+|ten|twenty|thirty|fifty|a hundred) years|future[- ]proof|bit ?rot|test of time")
COMPLEX = r"huge|enormous|massive|complex|complicated|monstro\w*|insane|bloat\w*|crazy|crazypants|baroque|ginormous|kitchen[- ]sink|byzantine|sprawling|convoluted|nightmare|\d{3,4}[- ]pages?"

# name: (patterns, exclusions, anchor). The first pattern is the anchor; every further pattern must
# match within WINDOW characters of it. anchor 0: no PDF needed; 1: the thread title names PDF or a
# PDF name is within WINDOW of the anchor; 2: a PDF name must be within WINDOW of the anchor.
XFA = (r"\bxfa\b|only (opens?|works?|renders?|displays?) (properly |correctly |right )?(in|with) (adobe|acrobat)|(requires?|need(s|ed)?) (adobe |acrobat )?(acrobat|reader)( reader)? to (open|view|fill|display)|"
       r"please wait\W{0,5}.{0,60}(adobe|acrobat|reader)")
THEMES = {
    # --- complaints and worries ---
    "screens_reflow": ([r"\breflow|pinch[- ]?(to[- ])?zoom|(phones?|mobile|smartphones?|small screens?|e-?readers?|kindle|tablets?)\b.{0,100}\b(zoom|scroll|unreadable|tiny|squint|pan(ning)? |two[- ]columns?|columns)|"
                        r"(zoom|scroll|unreadable|tiny|squint|two[- ]columns?)\b.{0,100}\b(phones?|mobile|smartphones?|small screens?|e-?readers?|kindle)\b|"
                        r"(not|un)(fit|suited|designed|meant|made) for (reading on )?screens?|designed for (print|paper)|optimi[sz]ed for print|unfit for human"], [], 1),
    "spec_complexity": ([rf"(spec|specification|standard|format)s?\b.{{0,40}}\b({COMPLEX})|\b({COMPLEX})\b.{{0,25}}\b(file )?(spec|specification|format|standard)s?\b|(\d{{3,4}}|thousand)[- ]pages? (long |of )?(spec|specification|standard)"], [], 2),
    "malformed_files": ([r"malformed|\bcorrupt(ed|ion|s)?\b|non-?conform\w*|non-?complian\w*|out[- ]of[- ]spec|(not|doesn'?t|don'?t|never) (follow|conform to|respect|comply with) the (spec|standard)|"
                         r"(lenient|forgiving|tolerant|liberal) (in what|parsers?|readers?|renderers?|viewers?)|(broken|invalid|buggy|cursed) pdfs?\b|pdfs? (in|from) the wild|\brepair(ed|ing|s)? (the |a )?(pdf|file|xref)"], [], 2),
    "readers_disagree_acrobat_only": ([XFA + r"|(render|look|display|show|print)\w* (completely |totally |slightly )?different(ly)? (in|on|across|between|depending)|different (pdf )?(readers|viewers|renderers)\b.{0,60}(different|differently|disagree|break|wrong)|"
                                       r"(works|opens|renders|looks|displays) (fine|ok|correctly|perfectly|properly) in [\w.]+.{0,80}\b(but|not in|doesn'?t|breaks)|"
                                       r"\b(preview(\.app)?|pdf\.js|chrome'?s?|firefox'?s?|evince|poppler)\b.{0,60}\b(mangles?|garbles?|renders? (it )?wrong|blank pages?|missing (text|glyphs|fonts)|invisible text|can'?t (handle|open|fill))"], [], 2),
    "js_active_content": ([JSPDF], [], 2),
    "js_as_risk": ([JSPDF, r"exploit|malware|malicious|attack|vulnerab|dangerous|security|unsafe|risk|disable|turn(ed|ing)? off|switch(ed)? off|why (on earth |would |do |does )|nightmare|horrif|terrif|scary|insane|mistake|bad idea"], [], 2),
    "exploits_cves": ([r"\bcves?\b|exploit(s|ed|able|ing)?\b|vulnerab\w*|malware|malicious|zero[- ]?days?|\b0[- ]?days?|attack (surface|vector)|\brce\b|remote code|code execution|sandbox escape|weaponi[sz]\w*|jbig2|forcedentry"],
                      [r"exploit(ing|s)? (the |a )?(loophole|market|labou?r|workers)"], 2),
    "sandbox_isolation": ([r"sandbox\w*|qubes|dangerzone|gvisor|virtual machine|\bin a vm\b|trustedpdf|memory[- ]safe"], [], 2),
    "tracking_phone_home": ([r"phon(e|es|ed|ing) home|tracking pixel|web beacon|canary ?tokens?|(tracks?|tracking|spy|spies) (where|when|who|what you|users|readers|opens?|clicks?|views?|downloads?)|unique (hash|identifier|watermark|fingerprint)|"
                             r"per[- ](user|download|customer|subscriber) (hash|id|identifier|watermarks?|fingerprints?)|(embed|insert)\w* (a |an )?(unique |hidden |invisible )?(hash|identifier|watermark|fingerprint)|telemetry|fingerprint\w*"], [r"track changes"], 2),
    "metadata_hidden_history": ([r"metadata|incremental (update|save)s?|revision history|previous (versions?|revisions?)|edit(ing)? history|hidden (text|content|layers?|data|objects?)|pdfresurrect|orphan(ed)? objects?|\bexif\b|\bxmp\b|author (name|field)|producer (field|string|line)",
                                 r"leak|reveal|expos|strip|remov|scrub|sanitiz|privacy|forensic|recover|incriminat|identif|deanonym|traces?\b|caught|oops|careful"], [], 2),
    "redaction": ([r"redact\w*|unredact\w*"], [], 1),
    "redaction_failures": ([r"redact\w*|unredact\w*",
                            r"black (boxes|box|bars?|rectangles?|highlight\w*)|draw\w* (a |black )?(box|rectangle)|(underlying|original|hidden) text|copy(ing)?[ -](and[ -])?paste|select(ing|able)? (the )?text|still (there|in the|present|readable|selectable|extractable)|recover|unredact|"
                            r"(bad|improper|failed|botched|sloppy|incompetent|fake|naive)\w* redact|redact\w* (wrong|incorrectly|improperly|badly|fail)|not (really |actually |properly )?redacted|overlay|kerning|glyph (positions|widths)|x-?ray"], [], 1),
    "authenticity_forgery": ([r"\bauthenticity\b|\bauthentic\b|forger(y|ies)|\bforged\b|\bforg(e|ing) (a|the|pdfs?|documents?|signatures?)\b|tamper\w*|doctored|(fake[ds]?|altered|edited|modified|falsified) (a |the )?(pdfs?|documents?|bank statements?|invoices?|receipts?|pay ?stubs?|contracts?)\b|unaltered|"
                              r"(can'?t|cannot|can not|couldn'?t) be (edited|altered|changed|modified|tampered)|unalterable|uneditable"], [r"sourceforge"], 2),
    "digital_signatures": ([r"digital(ly)? sign\w*|cryptographic(al(ly)?)? sign\w*|signatures? (validation|verification|validity|is valid|are valid|panel)|\bpades\b|\bltv\b|long[- ]term validation|timestamp(ing)? authority|\bpki\b|\bx\.?509|eidas|qualified (electronic )?signatures?|"
                            r"(sign|signed|signing) (with|using) (a |my |your |the |an )?(certificate|cert|private key|smart ?card|id ?card|eid|hsm)|certif(y|ied|ication) signatures?|docmdp|signed pdfs?|pdf signatures?|byterange|shadow attack"], [], 2),
    "image_signatures": ([r"(print|printing|printed)\w*,? (it )?(out )?,?(sign|signing|signed)\w*,? (it )?(and )?(then )?(scan|scanning|scanned)|sign(ed|ing)?,? (and )?scan|scan(ned)? signature|(image|picture|png|jpe?g|photo) of (my |your |a |the )?signature|signature (image|png|jpe?g)|falsiscan|(fake|look) (hand[- ])?signed"], [], 1),
    "accessibility_disability": ([r"screen ?-?readers?|tagged pdfs?|pdf/ua|\bpdf ?ua\b|wcag|\ba11y\b|section 508|\balt[- ]text|visually impaired|low vision|\bblind (users|people|readers|person|colleagues?|mathematicians?|students?)|assistive tech\w*|"
                                  r"(in)?accessib\w+\b.{0,80}\b(disab|blind|screen ?readers?|impair|tagg|wcag|508|\bada\b|deaf|dyslex)|(disab|blind|impair)\w*\b.{0,80}\baccessib"], [], 1),
    "pdfa_archival": ([r"pdf/a\b|pdf/a-\d|\bpdfa\b|pdf-a\b|verapdf|iso 19005"], [r"pdfa\.org"], 0),
    "longevity_archiving": ([r"archiv(al|ing)|long[- ]term (preservation|storage|archiv\w*|access|readab\w*)|digital preservation|(in|for|after) (\d+|ten|twenty|thirty|fifty|a hundred) years|years from now|decades (from now|later|old)|future[- ]proof|bit ?rot|test of time|still (open|read|render|view)\w*"], [], 2),
    "licensing_pdf_libs": ([r"\bagpl\w*|\bgpl\w*|\blgpl|copyleft|licen[cs](e|es|ed|ing)\b|\bmit\b|apache[- ]?2|commercial licen|permissive\w*",
                            r"mupdf|pymupdf|\bfitz\b|itext|ghostscript|artifex|pdfium|pdf\.js|pdfbox|poppler|\bqpdf|xpdf|pdftron|apryse|aspose|pspdfkit|nutrient|syncfusion|ironpdf|questpdf|pdf (librar|sdk|engine|toolkit|renderer|parser)|pdf-lib|reportlab|stirling|borb|openpdf|pdfsharp"],
                           [], 0),
    "agpl_pdf_libs": ([r"\bagpl", r"mupdf|pymupdf|\bfitz\b|itext|ghostscript|artifex|pdf (librar|sdk|engine|toolkit)|stirling|borb"], [], 0),
    "fonts_embedding": ([r"fonts? (are |is |was |were |aren'?t |isn'?t )?(not |never )?(embedded|subsetted|missing)|embed(ded|ding|s)? (the |all |its |their |your )?fonts?|missing fonts?|font substitut\w*|substitut\w* (a |the )?fonts?|type ?3 fonts?|tounicode|cid ?fonts?|font (licen\w+|embedding)|wrong fonts?"], [], 2),
    "spec_access_paywall": ([r"(iso|spec|specification|standard|32000)\b.{0,80}(paywall\w*|\bpay\b|cost\w*|\$ ?\d+|\d+ ?chf|chf ?\d+|€ ?\d+|expensive|for free|free (copy|download|access|of charge)|no[- ]cost|behind a)|paywalled|took down the (pdf )?(1\.7 )?spec"], [], 2),
    "adobe_distrust": ([r"\badobe\b|acrobat", r"subscription|bloat\w*|spyware|malware|hostile|\bads\b|upsell|\bnag|crapware|creative cloud|\bai assistant|monthly|predatory|dark pattern|extension silently|updater|background (services|processes)|cancel(lation)? fee|telemetry"], [], 2),
    "data_locked_in_pdf": ([r"machine[- ]readable|where data goes to die|data (is |gets )?(trapped|locked|buried) in|trapped in pdfs?|locked (up |away )?in pdfs?|instead of (an? )?(api|csv|json|xml|spreadsheet)|(publish|provide|release|give)\w* .{0,30}(csv|json|xml|api|spreadsheet)s? instead"], [], 2),
    "privacy_local_processing": ([r"(upload|uploading|send|sending)\w* .{0,40}(sensitive|confidential|private|tax|medical|legal|personal)|(random|sketchy|shady|untrusted|third[- ]party|online) (pdf )?(websites?|sites|services|tools|converters)|client[- ]side|in[- ]browser|never leaves?|no uploads?|without upload\w*|privacy[- ](first|focused|friendly)|local[- ]first|runs? (entirely |completely |fully )?(locally|offline|in (the|your) browser)"], [], 2),
    "llm_ai_context": ([r"\bllms?\b|\bgpt-?\d?\b|chatgpt|\bclaude\b|gemini|\bai\b|large language model|\brag\b"], [], 2),
    # --- what people want, use or praise ---
    "praise_fixed_layout": ([r"look\w* (exactly )?the same (everywhere|on every|on any|on all|across|regardless|no matter)|render\w* (exactly )?the same (everywhere|on every|on any|across)|same (everywhere|on every (device|machine|platform|screen|computer|printer|os))|pixel[- ]perfect|"
                             r"(faithful|exact|reliable|consistent) (reproduction|layout|rendering|printing|print)|print\w* (exactly|reliably|perfectly) |as the (author|creator|designer|publisher) intended|digital (paper|print)\b|electronic paper|paper[- ](replacement|equivalent|substitute)"], [], 2),
    "praise_self_contained": ([r"self[- ]contained|single (self-contained )?file|no (ads|trackers|tracking|popups|pop-?ups|cookie banners)\b|link ?rot|dead links|in (my|your) (custody|possession)|can'?t be taken away|(won'?t|doesn'?t|don'?t) (disappear|go away)"], [], 2),
    "prefer_html_over_pdf": ([r"(html|epub|web ?page|plain text|markdown)( version)? (instead of|rather than|over|not|is better than|are better than|beats?) (a |the )?pdfs?|"
                              r"instead of (a |the |sending |publishing |just )?(a )?pdfs?\b.{0,60}\b(html|epub|web ?pages?|markdown|plain text|text)\b|rather than (a |the )?pdfs?\b.{0,60}\b(html|epub|web ?pages?|markdown|text)\b|"
                              r"(should|would) (be|have been) (published |made available |provided )?(in|as) (an? )?(html|epub|web ?page|plain text)|prefer (the |an? )?(html|epub)|(html|epub) (version|edition)s? (of|is|are|would|please)|"
                              r"(stop|quit) (using|publishing|sending) pdfs?|pdfs? (should|need to) (die|go away|be replaced)|hate pdfs?|pdfs? (are|is) (terrible|awful|horrible|the worst|unfit|a nightmare|a pain)"], [], 2),
    "prefer_pdf_over_web": ([r"(rather|prefer to) (read|have|get|receive)\w* (a |the )?pdfs?|prefer(s|red)? (the )?pdfs?|give me (a |the )?pdfs?|pdfs? (is|are) (much |way |far )?(better|nicer|superior|easier to read)|(i |we )(love|like) pdfs?|pdfs? (is|are) (great|fantastic|wonderful|amazing|perfect|underrated|the best)"], [], 2),
    "want_safe_static": ([r"(disable[sd]?|disabling|turn(ed|ing|s)? off|switch(ed|es)? off|strip(ped|ping|s)?|remov(e|es|ed|ing)|block(ed|ing|s)?|ignor(e|es|ed|ing)|doesn'?t (run|support|execute|implement)|don'?t (run|support|execute|implement)|never (run|implemented|supported)|without)\b.{0,25}\b(javascript|\bjs\b|active content|scripting|launch actions?)\b|"
                          r"sanitiz\w*|dangerzone|trustedpdf|locked[- ]down (pdf )?(viewer|reader)|safe (subset|pdf viewer|pdf reader|viewer)|(convert|flatten|rasteri[sz])\w* (it |them |the pdf |pdfs? |pages? |each page )?(to|into) (images|pixels|bitmaps|pngs?|raster)"], [], 2),
    "want_inspect_tools": ([r"qpdf --qdf|\bqdf\b|mutool (clean|show)|pdf-?parser|peepdf|pdfid|\brups\b|pdf debugger|pdfbox debugger|(inspect|look inside|peek inside|visuali[sz]e|explore)\w* (the )?(internals?|structure|objects?|inside) of (a |the )?pdfs?|pdf (internals|structure|object tree|objects) (viewer|tool|inspector)|(open|read|look at) (it |the pdf |a pdf )?in a text editor|uncompress(ed)? (the )?(pdf|streams)"], [], 2),
    "fun_hacks_polyglots": ([r"polyglot|tetris|\bdoom\b|breakout|(game|emulator|linux|risc-?v|snake|chess)\b.{0,30}\b(in|inside|running in) (a )?pdfs?|pdfs? (that|which) (is|are) (also )?(a )?(zip|html|jpe?g|png|mp3|bootloader|game|executable|valid)|larger than germany|smallest (possible )?(valid )?pdf|minimal pdf|pocorgtfo"], [], 1),
}


def period(year: int) -> str:
    return "P1" if year <= 2019 else ("P2" if year <= 2022 else "P3")


def load():
    stories = {}
    for line in open(ROOT / "data/stories.jsonl"):
        s = json.loads(line)
        stories[s["objectID"]] = s
    seen, rows = set(), []
    for src in ("threads.jsonl", "comment_hits.jsonl"):
        for line in open(ROOT / "data" / src):
            c = json.loads(line)
            cid = int(c["id"])
            if cid in seen:
                continue
            seen.add(cid)
            text = c.get("text") or ""
            title = c.get("story_title") or ""
            if len(text) < 40 or JOBS.search(title):
                continue
            if str(c.get("story_id")) in stories or PDF.search(text):
                c["clean"] = URL.sub(" ", text)
                c["pdf_title"] = bool(PDFTXT.search(title))
                rows.append(c)
    return rows


def compile_themes():
    return {name: ([re.compile(p, re.I | re.S) for p in pats], [re.compile(e, re.I) for e in excl], anchored)
            for name, (pats, excl, anchored) in THEMES.items()}


def matches(c, theme):
    pats, excl, anchor = theme
    text = c["clean"]
    if any(e.search(text) for e in excl):
        return False
    for m in pats[0].finditer(text):
        lo, hi = max(0, m.start() - WINDOW), m.end() + WINDOW
        if anchor == 2 or (anchor == 1 and not c["pdf_title"]):
            if not PDFTXT.search(text, lo, hi):
                continue
        if all(p.search(text, lo, hi) for p in pats[1:]):
            return True
    return False


def main():
    rows = load()
    themes = compile_themes()
    base = Counter(period(int(c["created_at"][:4])) for c in rows)
    counts, hits = defaultdict(Counter), defaultdict(list)
    for c in rows:
        p = period(int(c["created_at"][:4]))
        for name, theme in themes.items():
            if matches(c, theme):
                counts[name][p] += 1
                hits[name].append(c)
    args = sys.argv[1:]
    if args[:1] in (["--sample-all"], ["--sample"]):
        names = list(THEMES) if args[0] == "--sample-all" else [args[1]]
        rest = args[1:] if args[0] == "--sample-all" else args[2:]
        n, seed = int(rest[0]), int(rest[1]) if len(rest) > 1 else 1
        random.seed(seed)
        for name in names:
            pool = hits[name]
            print(f"===== {name} ({len(pool)})")
            for c in random.sample(pool, min(n, len(pool))):
                print(f'  [{c["id"]} {c["created_at"][:10]} {(c.get("story_title") or "")[:24]}] ' + c["text"][:200].replace("\n", " "))
        return
    print(f"universe: {len(rows)} comments; per period: " + ", ".join(f"{p} {base[p]}" for p in ("P1", "P2", "P3")))
    print(f'{"theme":28} {"total":>6} {"P1":>5} {"P2":>5} {"P3":>5}   per 1,000: {"P1":>5} {"P2":>5} {"P3":>5}')
    for name in THEMES:
        k = counts[name]
        rate = [1000 * k[p] / base[p] for p in ("P1", "P2", "P3")]
        print(f"{name:28} {sum(k.values()):6} {k['P1']:5} {k['P2']:5} {k['P3']:5}   {rate[0]:16.1f} {rate[1]:5.1f} {rate[2]:5.1f}")


if __name__ == "__main__" and sys.argv[1:2] != ["--extra"]:
    main()


# Narrow follow-up counts (run with --extra): wants that the broad themes above mix with complaints.
EXTRA = {
    "tagged_pdf_ua_output": ([r"tagged pdfs?|pdf/ua|\bpdf ?ua\b|tagpdf|structure tree|struct ?tree|\btagging\b"], [], 1),
    "want_real_redaction_tool": ([r"redact\w*", r"\btools?\b|software|feature|properly|real|true|actual\w*|permanent\w*|apply redactions|burn"], [], 1),
    "browser_viewer_safer": ([r"(pdf\.js|pdfjs|pdfium|chrome'?s? (pdf )?(viewer|reader)|firefox'?s? (pdf )?(viewer|reader)|browser'?s? (built-in )?(pdf )?(viewer|reader)|(in|inside) (the|a|your) browser)",
                              r"safe|secur|sandbox|isolat|attack surface|harden"], [], 2),
}


def extra():
    rows = load()
    themes = {name: ([re.compile(p, re.I | re.S) for p in pats], [re.compile(e, re.I) for e in excl], anchor)
              for name, (pats, excl, anchor) in EXTRA.items()}
    base = Counter(period(int(c["created_at"][:4])) for c in rows)
    random.seed(3)
    for name, theme in themes.items():
        hits = [c for c in rows if matches(c, theme)]
        k = Counter(period(int(c["created_at"][:4])) for c in hits)
        print(f"{name:28} {len(hits):6} {k['P1']:5} {k['P2']:5} {k['P3']:5}   per 1,000: "
              + " ".join(f"{1000 * k[p] / base[p]:5.1f}" for p in ("P1", "P2", "P3")))
        for c in random.sample(hits, min(8, len(hits))):
            print(f'  [{c["id"]} {c["created_at"][:10]}] ' + c["text"][:160].replace("\n", " "))


if __name__ == "__main__" and sys.argv[1:2] == ["--extra"]:
    extra()
