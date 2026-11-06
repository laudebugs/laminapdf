"""Count editor and workflow themes in ten years of HN PDF comments (editors researcher).

Run from the hn/ folder:
    HN_DATA=<dir> python3 -I -B tools/hn-research/counts/editors/editors_counts.py              # theme table + tool sentiment + stories
    HN_DATA=<dir> python3 -I -B tools/hn-research/counts/editors/editors_counts.py --samples THEME[,THEME…] [N] [SEED]   # random matches, for spot checks
    HN_DATA=<dir> python3 -I -B tools/hn-research/counts/editors/editors_counts.py --hand-sample        # the seeded samples that HAND codes

Base: every unique comment of at least 40 characters in data/threads.jsonl and
data/comment_hits.jsonl that sits in a PDF story (data/stories.jsonl) or mentions a PDF
term (the same filter as scripts/prep.py): 56,722 comments. A comment counts once per
theme. "pdf" themes also need a PDF word (PDFISH) in the comment itself; "need" is a second
regex the comment must also match. "ed" is how many matches are also in areas/editors.jsonl.
Periods: 2016-19, 2020-22, 2023-26; "per 1k" divides by the period's comment total.
"""

import json
import os
import random
import re
import sys
from collections import Counter
from multiprocessing import Pool

ROOT = os.environ.get("HN_DATA", ".")

PDF = re.compile(r"\bpdfs?\b|pdf\.js|pdfjs|pdfium|mupdf|poppler|acrobat|ghostscript|wkhtmltopdf|weasyprint|typst|"
                 r"sumatra|okular|zathura|foxit|pdfbox|itext|qpdf|pdftk|docling|pdfplumber|pymupdf", re.I)
PDFISH = re.compile(PDF.pattern + r"|\bpreview\b|\bxfa\b|acroform|docusign|stirling|xournal|smallpdf|ilovepdf|pdf24", re.I)


def rx(p):
    return re.compile(p, re.I)


ACT = (r"(edit(s|ed|ing|or|ors|able)?|annotat\w*|fill\w*|forms?|sign(s|ed|ing|ature|atures|er|ers)?|redact\w*|merg\w*|split\w*|rotat\w*|reorder\w*|compress\w*|"
       r"ocr\w*|watermark\w*|highlight\w*|stamp\w*|pdf editors?)")
OPS = r"(merg\w*|split\w*|rotat\w*|fill\w*|sign(s|ed|ing|ature|atures|er|ers)?|ocr\w*|compress\w*|watermark\w*|stamp\w*|redact\w*|flatten\w*)"
NOT_CODE_SIG = r"(?<!type )(?<!function )(?<!method )(?<!call )(?<!api )"


def near(a, b, n=60):
    """a then b, or b then a, within n characters of one sentence."""
    return rf"\b{a}\b[^.\n]{{0,{n}}}\b{b}\b|\b{b}\b[^.\n]{{0,{n}}}\b{a}\b"


THEMES = {
    # ---- tasks people ask for or describe doing ----
    "page_ops": dict(pdf=True, need=rx(
        r"preview|acrobat|pdftk|qpdf|ghostscript|stirling|pdfsam|foxit|xchange|libreoffice|\btools?\b|\bapps?\b|"
        r"\beditors?\b|online|browser|command|script|\bcli\b|edit(s|ed|ing)?\b|sidebar|thumbnails?|drag"), rx=rx(
        r"\b(merg\w*|split\w*|rotat\w*|reorder\w*|re-order\w*|rearrang\w*|concatenat\w*|combin\w*)\b"
        r"[^.\n]{0,25}\b(pdfs?|pages?|(pdf )?documents?|(pdf )?files)\b|"
        r"\b(extract\w*|delet\w*|insert\w*|remov\w*|crop\w*|reorder\w*|rotat\w*)\b[^.\n]{0,15}\bpages?\b|"
        r"\b(pdfs?|pages?)\b[^.\n]{0,20}\b(merg\w*|split\w*|rotat\w*|reorder\w*|rearrang\w*)\b|"
        r"pdfunite|pdfseparate|pdfjam|pdfsam|pdf split and merge")),
    "form_fill": dict(pdf=False, rx=rx(
        r"\bfill(s|ed|ing)?( in| out)?\b[^.\n]{0,40}\bforms?\b|fillable|form[- ]fill\w*|\bacroforms?\b|\bxfa\b|"
        r"\bpdf forms\b|\b(a|the|this|that|fillable|government|tax|interactive) pdf form\b|livecycle")),
    "xfa": dict(pdf=False, rx=rx(r"\bxfa\b|livecycle|dynamic (xfa |pdf )?forms?\b")),
    "sign_any": dict(pdf=False, rx=rx(
        r"\bsign(s|ed|ing)? (a |the |my |your |this |these )?(pdfs?|documents?|contracts?|forms?|paperwork)\b|"
        + NOT_CODE_SIG + r"\bsignatures?\b|\be-?signatures?|\be-?sign(ing|ed)?\b|docusign|hellosign|docuseal|"
        r"documenso|adobe sign")),
    "sign_image": dict(pdf=False, rx=rx(
        r"(image|png|picture|pic|scan(ned)?|photo|jpe?g|drawing|drawn|doodle|squiggle)\b[^.\n]{0,15}\bof (my|your|a|"
        r"the|his|her) (physical |handwritten |hand-written |own |real )?signature|signature (image|png|stamp|picture|"
        r"file)|(paste|pasting|pasted|insert\w*|stick\w*|overlay\w*|slap\w*)[^.\n]{0,25}\bsignature|"
        r"draw(ing|n)? (a |my |your |the )?signature|signature\.png|(trackpad|touchpad|webcam|camera|mouse|finger|"
        r"apple pencil)[^.\n]{0,40}signature|signature[^.\n]{0,40}(trackpad|touchpad|webcam|camera|mouse)")),
    "sign_crypto": dict(pdf=False, need=rx(r"sign"), rx=rx(
        r"digital(ly)? sign(s|ed|ing|ature|atures|er|ers)?|cryptographic(ally)?( digital)? sign(s|ed|ing|ature|atures|er|ers)?|crypto(graphic)? signatures?|"
        r"(signing|signature|x\.?509|qualified|personal) certificates?|certificate[- ]based (sign|signature)\w*|\beidas\b|"
        r"qualified (electronic )?signature|smart ?cards?|\bpkcs ?#?1[12]\b|\bpades\b|"
        r"open-pdf-sign|jsignpdf|pyhanko|\bid[- ]cards?\b|\be-?id\b|national id")),
    "print_sign_scan": dict(pdf=False, rx=rx(
        r"print[^.\n]{0,20}\bsign[^.\n]{0,20}\bscan|wet[- ]ink|wet signatures?|\"wet\" signature|"
        r"look(s|ed|ing)? (like (it|they) (was|were|had been) )?(printed and )?scanned|fake[- ]?scan\w*|"
        r"falsi(scan|sign)")),
    "annotate": dict(pdf=True, rx=rx(near(
        r"annotat(e|es|ed|ing|ion|ions|or|ors)",
        r"(pdfs?|papers?|documents?|highlight\w*|comments?|notes?|readers?|viewers?|okular|preview|acrobat|xournal\+*|"
        r"ipad|pencil|stylus|zotero|margins?|pen)", 40) +
        r"|(?<!syntax )(?<!code )\bhighlight(s|ed|ing|er|ers)? (\w+ )?(text|passages?|sections?|lines?|words?|pdfs?|"
        r"papers?|documents?)\b|\bmark(ing)? up (a |the )?(pdfs?|documents?|papers?)|sticky notes?|"
        r"comments? (on|in) (a |the )?pdf")),
    "edit_text": dict(pdf=True, need=rx(near(r"edit(s|ed|ing|or|ors|able)?", r"pdfs?", 50)), rx=rx(
        r"\bedit(s|ed|ing)?\b[^.\n]{0,25}\b(existing |actual |real |original |the |its |their |pdf'?s? )*text\b|"
        r"\b(modify|modifying|change|changing|replace|replacing|fix|fixing|correct\w*)\b[^.\n]{0,15}\b(the |some |a |"
        r"one )?(text|typos?|words?|sentences?|paragraphs?|numbers?|dates?|names?)\b|text editing|"
        r"like a word doc|edit(s|ed|ing|or|ors|able)? (the )?(actual |page )?content")),
    "edit_hard": dict(pdf=True, need=rx(r"\bedit"), rx=rx(
        r"(can't|cannot|couldn't|can not|unable to|impossible to|hard to|difficult to|no way to|not possible to|"
        r"doesn't let you|won't let you)\b[^.\n]{0,25}\bedit(s|ed|ing|or|ors|able)?|\bedit(s|ed|ing|or|ors|able)?\b[^.\n]{0,40}\b(is|are|was) (really |very |"
        r"incredibly |notoriously |so |basically |almost |nearly )?(hard|impossible|difficult|a nightmare|a pain|painful|"
        r"not possible)|(hard|impossible|difficult|painful|nightmare)\b[^.\n]{0,15}\bto edit|so hard to edit")),
    "redact": dict(pdf=False, rx=rx(r"\bredact\w*")),
    "privacy_local": dict(pdf=True, need=rx(r"upload|never leaves?|\bleav(e|es|ing)\b|privacy|private|sensitive|"
                                            r"confidential|trust|tracking"), rx=rx(near(
        r"(client[- ]side|no uploads?|without upload\w*|never leaves?|leav(e|es|ing) (your|the|my) (computer|machine|"
        r"device|browser)|runs? (100% |entirely |fully |completely |all )?locally|local[- ](only|first)|"
        r"privacy[- ](first|friendly|focused|respecting))",
        r"(pdfs?|pdf (tools?|editors?)|tools?|editors?|merg\w*|split\w*|sign(ing|ed)?|redact\w*|compress\w*|ocr|"
        r"convert\w*|edit(ing|s)?|forms?)", 60))),
    "online_tools": dict(pdf=True, rx=rx(
        r"smallpdf|ilovepdf|i love pdf|pdf24|sejda|pdfescape|dochub|pdffiller|online (pdf )?(tools?|services?|editors?|"
        r"converters?)\b")),
    "self_hosted": dict(pdf=True, rx=rx(r"stirling[- ]?pdf|stirling[- ]tools|\bstirling\b[^.\n]{0,30}\b(pdf|tools?|"
                                        r"docker|self|host\w*)\b|" + near(r"self[- ]?host\w*", r"(pdfs?|stirling)", 80) +
                                        r"|paperless-ngx|paperless-ng|paperless ngx")),
    "ocr_searchable": dict(pdf=True, rx=rx(
        r"ocrmypdf|searchable (pdfs?|text|layer)|text layer|ocr layer|(make|making|made) (it|them|the pdfs?|pdfs?|"
        r"scans?|documents?) searchable|(add|adding|added|run|running|runs) (an |the )?ocr|\bocr\w*\b[^.\n]{0,40}"
        r"\bscan\w*|\bscan\w*\b[^.\n]{0,40}\bocr\w*|pdfsandwich|abbyy|finereader")),
    "compress": dict(pdf=True, need=rx(r"\b(file ?size|sizes?|mb|kb|gb|megabytes?|smaller|shrink\w*|too (big|large)|"
                                       r"e-?mail\w*|upload\w*|attach\w*)\b"), rx=rx(near(
        r"(compress(ing|ion|or|ors)?|compress (the|a|my|it|them|your)|shrink\w*|reduc\w* (the |its |their )?(file )?size|"
        r"(file )?size limits?|too (big|large)|downsampl\w*)", r"(pdfs?|scans?|scanned)", 40) +
        r"|pdfsettings|-dpdfsettings")),
    "cli_tools": dict(pdf=False, rx=rx(
        r"pdftk|\bqpdf\b|pdfunite|pdfseparate|pdfjam|\bgs -|-sdevice=pdfwrite|mutool|pdfcpu|\bcpdf\b|img2pdf|"
        r"\bconvert -density|pikepdf|ocrmypdf|pystitcher|pdfly")),
    "batch_script": dict(pdf=True, rx=rx(near(
        r"(batch\w*|scripts?|scripting|scripted|scriptable|automate|automating|programmatic\w*|command[- ]line|cli|bulk|"
        r"hundreds of|thousands of)", OPS, 60))),
    "mobile": dict(pdf=True, rx=rx(near(
        r"(ipad|iphone|ios|android|phones?|tablets?|apple pencil|stylus|smartphones?)",
        r"(annotat\w*|sign(ing|ed)? (a |the |my )?(pdfs?|documents?|forms?|contracts?)|signatures?|fill(ing|ed)? (out |in )?"
        r"(the |a |pdf )?forms?|edit(ing)? (a |the )?pdfs?|pdf editors?|highlight(ing|s)?|mark ?up)", 60))),
    "linux": dict(pdf=True, rx=rx(near(r"linux", r"(pdf editors?|edit(s|ed|ing|or|ors|able)?|annotat\w*|fill\w*|forms?|sign(s|ed|ing|ature|atures|er|ers)?)", 80))),
    "want_oss_acrobat": dict(pdf=False, rx=rx(
        r"(open[- ]source|foss|free software|libre)\b[^.\n]{0,30}\b(pdf editors?|alternatives? to (adobe )?acrobat|"
        r"acrobat alternatives?|equivalent (of|to) (adobe )?acrobat)|(no|isn't|is not|there's no|there is no|lack of|"
        r"never found|haven't found|can't find|nothing)\b[^.\n]{0,50}\b(open[- ]source|free|foss|linux)\b"
        r"[^.\n]{0,20}\b(pdf )?(editors?|alternatives?|equivalents?)\b|acrobat (alternatives?|replacements?)|"
        r"alternatives? to (adobe )?acrobat|replace (adobe )?acrobat|(gimp|libreoffice) (of|for) pdfs?")),
    "browser_edit": dict(pdf=True, rx=rx(near(
        r"(firefox|pdf\.js|pdfjs|edge|chrome|chromium)",
        r"(edit(s|ed|ing|or|ors|able)? (pdfs?|text|it|them)|annotat\w*|fill\w* (out |in )?(the |a |pdf )?forms?|form[- ]fill\w*|"
        r"sign(s|ed|ing|ature|atures|er|ers)? (pdfs?|documents?|it|them)|add\w* (text|signatures?|drawings?|images?)|highlight\w*|draw\w* on)", 60))),
    "ai_edit": dict(pdf=True, rx=rx(near(
        r"(llms?|gpt-?\d\w*|chatgpt|claude|gemini|ai|copilot|nano banana|agents?|agentic)",
        r"(fill\w* (out |in )?(the |a |pdf )?forms?|edit(s|ed|ing|or|ors|able)? (the |a )?pdfs?|redact\w*|annotat\w* (the |a )?pdfs?|"
        r"pdf editors?|form[- ]fill\w*)", 50))),
    "watermark": dict(pdf=True, rx=rx(r"watermark\w*")),
    "workflow_chain": dict(pdf=True, rx=rx(
        r"\b(merg\w*|split\w*|compress\w*|ocr|rotat\w*)\b[^.\n]{0,25}\b(then|and then|->|→)\b[^.\n]{0,25}\b(merg\w*|"
        r"split\w*|compress\w*|ocr|rotat\w*|add\w* page|remov\w* page)|(chain|pipeline|workflow|in one (go|step|"
        r"click)|one click|multiple steps)\b[^.\n]{0,40}\b(merg\w*|split\w*|compress\w*|rotat\w*)\b")),
    "flatten": dict(pdf=True, rx=rx(r"\bflatten\w*")),
    "pdf_editor_phrase": dict(pdf=False, rx=rx(r"\bpdf editors?\b|edit(ing)? (a |the )?pdfs?\b")),
    # ---- complaints ----
    "redact_fail": dict(pdf=False, need=rx(r"redact"), rx=rx(
        r"(black|white|opaque)[- ](box|boxes|bar|bars|rectangle|rectangles|highlight\w*|marker|squares?|ink)|"
        r"unredact\w*|un-redact\w*|redaction (fail\w*|mistakes?|blunders?|fails)|bad redactions?|"
        r"(text|data|content|information) (is |was )?still (there|underneath|in the|present|selectable)|"
        r"(copy|copied|select\w*)[^.\n]{0,30}(underneath|under the|beneath|redacted)|draw\w* (on top|over)|"
        r"\b(layers?|underneath)\b")),
    "not_for_editing": dict(pdf=True, need=rx(r"\b(edit|modif|chang)"), rx=rx(
        r"(not|never|wasn't|isn't|weren't) (really )?(meant|designed|intended|made|supposed) (to be |for )?"
        r"(edit(s|ed|ing|or|ors|able)?|modif\w*|changed)|(write|read)-only (format|by design)|edit the (source|original)|"
        r"(go back to|use|have|need|get) the (source|original) (file|document)|digital paper|electronic paper|"
        r"append-only (format|by design)|(final|presentation|print|distribution) format")),
    "acrobat_price": dict(pdf=False, rx=rx(
        r"\b(acrobat|adobe)\b[^\n]{0,100}\b(subscri\w*|\$\s?\d+|price\w*|pricing|expensive|paywall\w*|per month|"
        r"a month|/mo\b|/month|monthly|cancel\w*|cancellation)|(subscri\w*|\$\s?\d+|pric\w*|expensive|paywall\w*|"
        r"per month|monthly|cancel\w*)[^\n]{0,80}\b(acrobat|adobe)\b")),
    "acrobat_bloat": dict(pdf=False, rx=rx(
        r"\b(acrobat|adobe reader|adobe's reader|adobe pdf)\b[^\n]{0,100}\b(bloat\w*|slow\w*|sluggish|nag\w*|spam\w*|"
        r"\bads\b|upsell\w*|crap\w*|garbage|terrible|awful|horrible|hate|malware|resource[- ]hungry|intrusive|"
        r"sign[- ]in|log ?in|account|telemetry|crash\w*|worst|annoying|clunky)|(bloat\w*|slow|sluggish|nag\w*|"
        r"terrible|awful|horrible|hate|malware|intrusive|crash\w*|worst|annoying|clunky)[^\n]{0,60}\b(acrobat|"
        r"adobe reader)\b")),
    "save_breaks": dict(pdf=True, rx=rx(
        r"\b(preview|libreoffice|draw|pdftk|acrobat|foxit|edge|chrome|xournal|okular|evince|sav(e|es|ed|ing))\b"
        r"[^.\n]{0,60}\b(corrupt\w*|mangl\w*|destroy\w*|garbl\w*|scrambl\w*|ruin\w*|messe[sd] up|broke (the|my|it|"
        r"them)|breaks (the|my|it|them))|(corrupt\w*|mangl\w*|destroy\w*|garbl\w*|scrambl\w*)\b[^.\n]{0,50}\b(when|"
        r"after|on|by|while) (sav\w*|edit(s|ed|ing|or|ors|able)?|annotat\w*|sign(s|ed|ing|ature|atures|er|ers)?|rotat\w*|merg\w*|fill\w*)|(lost|lose|losing) "
        r"(all )?(my |the )?(annotations?|highlights?|edits?|changes|form (data|entries))")),
    "annot_interop": dict(pdf=True, rx=rx(near(
        r"(annotat\w*|highlights?)",
        r"(other (apps?|viewers?|readers?|programs?|software|tools)|show up|lost|lose|disappear\w*|don't show|"
        r"doesn't show|not show\w*|compatib\w*|incompatib\w*|sidecar|external (file|database)|separate file|"
        r"proprietary|stored (in|outside)|export\w*|sync\w*|locked in|lock-in)", 80))),
    "forms_dont_work": dict(pdf=True, rx=rx(
        r"\b(forms?|fields?)\b[^.\n]{0,60}\b(only (open|work|render)s? in|require\w* (adobe|acrobat)|won't (open|save|"
        r"work)|doesn't (open|save|work)|don't (open|save|work)|can't (fill|save|open)|not (saved|saving|fillable)|"
        r"disappear\w*|please wait)|(can't|cannot|couldn't|unable to) (fill|save)[^.\n]{0,30}\bforms?\b|"
        r"non-?fillable|not fillable|print (it |them )?out[^.\n]{0,30}\b(fill\w*|by hand|pen)\b|"
        r"\bprint\w*\b[^.\n]{0,20}\bfill\w*[^.\n]{0,20}\bscan")),
    "sign_insecure": dict(pdf=False, need=rx(r"sign"), rx=rx(
        r"pdf-insecurity|shadow attacks?|spoof\w*|signature\w*[^.\n]{0,40}(bypass\w*|vulnerab\w*|attack\w*|fragile|"
        r"insecure)|incremental saving attack|signature wrapping")),
    "incremental_history": dict(pdf=True, rx=rx(
        r"incremental (save|saving|update|updates)\b|/prev\b|previous (revisions?|versions?)\b[^.\n]{0,40}"
        r"(inside|in the (file|pdf)|embedded|preserved|still there|recover\w*)|(edit|revision) history\b[^.\n]{0,40}"
        r"pdf|pdf[^.\n]{0,40}(edit|revision) history")),
}

THEMES["upload_worry"] = dict(pdf=True, rx=rx(
    r"(?=[^.\n?!]*\b(privacy|private|sensitive|confidential|personal|trust\w*|leak\w*)\b)[^.\n?!]*\b(upload\w*|"
    r"send(ing|s)?|sent)\b[^.\n?!]{0,60}\b(servers?|cloud|third[- ]part\w*|websites?|online)\b"))

# Themes reported together: a comment counts once if it matches any member.
UNIONS = {
    "edit_existing_text": ("edit_text", "edit_hard"),
    "linux_or_no_oss_editor": ("linux", "want_oss_acrobat"),
    "forms_locked_to_acrobat": ("xfa", "forms_dont_work"),
    "acrobat_cost_or_bloat": ("acrobat_price", "acrobat_bloat"),
    "local_or_upload_worry": ("privacy_local", "upload_worry"),
}

# Hand-coded sentiment of seeded random samples (--hand-sample prints them): P praise or a capability
# the writer values, N complaint, 0 neutral or off-topic. Coded by reading each snippet.
HAND = {
    ("Acrobat", "2016-19"): dict(P=[19756165, 16448214], N=[19665448, 18671129, 18091890, 21237656],
                                 Z=[19820613, 15429071, 19820740, 19483955, 16625718, 18389607, 14249780, 16448952,
                                    19320063, 13450542, 16703804, 17974777, 20900169, 15429068]),
    ("Acrobat", "2020-22"): dict(P=[32003197, 26753802, 22643035, 30315740, 24556167],
                                 N=[26431211, 30187108, 23275232, 33709780, 27972648, 27969491, 30418051, 24569010],
                                 Z=[31874214, 29619940, 33742698, 28141677, 23260355, 29859534, 33147215]),
    ("Acrobat", "2023-26"): dict(P=[37996588, 46131906, 46372867, 48977763],
                                 N=[49334299, 39181831, 46508667, 35092563, 39218759, 38605271, 40607088],
                                 Z=[37028120, 38517743, 43211786, 46575336, 50020273, 37044287, 41735692, 41069564,
                                    34537830]),
    ("Preview (macOS)", "2016-19"): dict(P=[19729419, 14269436, 15746208, 19699882, 21334002, 18377663, 17537456,
                                            19822242], N=[21592648, 13310377, 17209558],
                                         Z=[13360059, 19890618, 15412442, 19733071]),
    ("Preview (macOS)", "2020-22"): dict(P=[32542971, 30027103, 24558073, 30242265, 33709647, 27344654, 26776014,
                                            31315644, 31692631, 30379292], N=[27345770],
                                         Z=[26275548, 23670631, 23340355, 23183704]),
    ("Preview (macOS)", "2023-26"): dict(P=[45599819, 40353320, 38766019, 45599206, 34413745, 46717548, 35225035,
                                            40585284, 39880033], N=[37114946, 43249902],
                                         Z=[47564020, 43638631, 38978254, 47297653]),
    ("Stirling-PDF", "2023-26"): dict(P=[40341439, 47824192, 44780902, 46766979, 38771215, 43665447, 40996941],
                                      N=[40246085, 43237882, 42488932, 40245185],
                                      Z=[38766688, 43908626, 48061778, 46702774]),
    ("DocuSign", "2020-22"): dict(P=[29257578, 22815623, 29365518], N=[30025167, 29256853, 27806207],
                                  Z=[22812754, 23169091, 24123278, 31151639, 30036985, 30029868]),
    ("DocuSign", "2023-26"): dict(P=[], N=[38720733, 36806928, 50056992, 42682218, 42483810, 42482804],
                                  Z=[37080733, 36799512, 39037401, 39037341, 35580127, 34820728]),
    ("browser_edit", "2020-22"): dict(P=[33715341, 24555382, 24558073, 34133059, 28227784, 30026031, 33200360],
                                      N=[26517321], Z=[31020458, 24555584, 26841701, 24563449]),
    ("browser_edit", "2023-26"): dict(P=[41849376, 38049374, 37996241, 37112473, 45612356], N=[37139155],
                                      Z=[40277356, 48979737, 46676358, 44935802, 36460521, 40421846]),
}

TOOLS = {
    "Acrobat": r"\bacrobat\b|adobe reader",
    "Preview (macOS)": r"\bpreview\.app\b|macos preview|apple'?s preview|\bin preview\b|\bpreview (app|is|on|can|does|has|lets)\b",
    "Foxit": r"foxit",
    "PDF-XChange": r"pdf[- ]?xchange",
    "Okular": r"okular",
    "Xournal(++)": r"xournal",
    "Master PDF Editor": r"master ?pdf",
    "LibreOffice Draw": r"libre ?office( draw)?\b[^.\n]{0,40}\bpdf|\bpdf\b[^.\n]{0,40}libre ?office",
    "Stirling-PDF": r"stirling",
    "pdftk": r"pdftk",
    "qpdf": r"\bqpdf\b",
    "Ghostscript": r"ghostscript",
    "OCRmyPDF": r"ocrmypdf",
    "PDF Expert": r"pdf expert",
    "Firefox/pdf.js": r"firefox|pdf\.js|pdfjs",
    "Edge": r"\bedge\b[^.\n]{0,40}\b(pdf|viewer|reader)|\bpdf\b[^.\n]{0,40}\bedge\b",
    "Smallpdf/ilovepdf/pdf24": r"smallpdf|ilovepdf|pdf24|sejda",
    "DocuSign": r"docusign",
    "Bluebeam": r"bluebeam",
}
TOOLS = {k: rx(v) for k, v in TOOLS.items()}
POS = rx(r"\b(love[sd]?|great|excellent|amazing|awesome|best|fantastic|recommend\w*|happy|solid|reliable|perfect(ly)?|"
         r"life ?saver|works (well|great|fine|perfectly)|just works|good|nice|useful|handy|favou?rite|impressive|"
         r"underrated|gem|snappy)\b")
NEG = rx(r"\b(hate[sd]?|terrible|awful|horrible|bloat\w*|slow|crash\w*|corrupt\w*|broke|broken|bugg?y|bugs?|garbage|"
         r"crap\w*|worst|annoying|nag\w*|expensive|sucks?|nightmare|useless|spam\w*|malware|clunky|frustrat\w*|"
         r"mangl\w*|destroy\w*|intrusive|subscriptions?)\b")

PERIODS = ("2016-19", "2020-22", "2023-26")


def period(date: str) -> str:
    y = int((date or "0000")[:4])
    return PERIODS[0] if y <= 2019 else PERIODS[1] if y <= 2022 else PERIODS[2]


def load():
    stories = {s["objectID"]: s for s in map(json.loads, open(ROOT + "/data/stories.jsonl"))}
    seen = {}
    for src in ("threads.jsonl", "comment_hits.jsonl"):
        for line in open(ROOT + "/data/" + src):
            c = json.loads(line)
            c["id"] = int(c["id"])
            if len(c.get("text") or "") < 40:
                continue
            seen.setdefault(c["id"], c)
    base = [c for c in seen.values() if str(c.get("story_id")) in stories or PDF.search(c["text"])]
    editors = {json.loads(l)["id"] for l in open(ROOT + "/areas/editors.jsonl")}
    return stories, base, editors


def classify(c):
    """Return (id, period, themes matched, {tool: (pos, neg)})."""
    text = c["text"]
    pdfish = bool(PDFISH.search(text))
    themes = []
    for t, spec in THEMES.items():
        if spec["pdf"] and not pdfish:
            continue
        if "need" in spec and not spec["need"].search(text):
            continue
        if spec["rx"].search(text):
            themes.append(t)
    tools = {}
    if pdfish:
        for name, r in TOOLS.items():
            m = r.search(text)
            if m:
                window = text[max(0, m.start() - 200): m.end() + 200]
                tools[name] = (bool(POS.search(window)), bool(NEG.search(window)))
    return c["id"], period(c.get("created_at")), themes, tools


def hand_sample(base) -> None:
    """Print the seeded samples that HAND codes (the same draws, in the same order)."""
    def draw(label, r, p, n):
        rows = [c for c in base if period(c.get("created_at")) == p and PDFISH.search(c["text"]) and r.search(c["text"])]
        print(f"##### {label} {p} ({len(rows)})")
        for c in random.sample(rows, min(n, len(rows))):
            m = r.search(c["text"])
            print(f"[{c['id']}] {c['text'][max(0, m.start() - 220):m.end() + 220]}".replace("\n", " "))
    random.seed(42)
    for tool, n in (("Acrobat", 20), ("Preview (macOS)", 15)):
        for p in PERIODS:
            draw(tool, TOOLS[tool], p, n)
    random.seed(43)
    for tool, n, periods in (("Stirling-PDF", 15, ["2023-26"]), ("DocuSign", 12, ["2020-22", "2023-26"])):
        for p in periods:
            draw(tool, TOOLS[tool], p, n)
    for p in ("2020-22", "2023-26"):
        draw("browser_edit", THEMES["browser_edit"]["rx"], p, 12)


def main() -> None:
    stories, base, editors = load()
    if sys.argv[1:2] == ["--hand-sample"]:
        hand_sample(base)
        return
    with Pool(4) as pool:
        results = pool.map(classify, base, chunksize=500)
    by_id = {c["id"]: c for c in base}
    if len(sys.argv) >= 3 and sys.argv[1] == "--samples":
        n = int(sys.argv[3]) if len(sys.argv) > 3 else 8
        for theme in sys.argv[2].split(","):
            rows = [by_id[i] for i, _, th, _ in results if theme in th]
            random.seed(int(sys.argv[4]) if len(sys.argv) > 4 else 5)
            print(f"=== {theme} ({len(rows)})")
            for c in random.sample(rows, min(n, len(rows))):
                m = THEMES[theme]["rx"].search(c["text"])
                s = max(0, m.start() - 90)
                snippet = c["text"][s:m.end() + 90].replace("\n", " ")
                print(f"[{c['id']} {c['created_at'][:10]}] ...{snippet}")
        return
    totals = Counter(p for _, p, _, _ in results)
    print("base", len(base), dict(totals))
    print(f"{'theme':20} {'total':>6} {'ed':>5} | " + " | ".join(f"{p}: n (per 1k)" for p in PERIODS))
    counts = {t: Counter() for t in THEMES}
    ed = Counter()
    for i, p, th, _ in results:
        for t in th:
            counts[t][p] += 1
            ed[t] += i in editors
    for t, per in sorted(counts.items(), key=lambda kv: -sum(kv[1].values())):
        cells = " | ".join(f"{p}: {per[p]:>4} ({1000 * per[p] / totals[p]:5.1f})" for p in PERIODS)
        print(f"{t:20} {sum(per.values()):>6} {ed[t]:>5} | {cells}")
    sets = {t: set() for t in THEMES}
    for i, p, th, _ in results:
        for t in th:
            sets[t].add((i, p))
    print("\nUnions (a comment counted once):")
    for u, members in UNIONS.items():
        rows = set().union(*(sets[m] for m in members))
        per = Counter(p for _, p in rows)
        cells = " | ".join(f"{p}: {per[p]:>4} ({1000 * per[p] / totals[p]:5.1f})" for p in PERIODS)
        print(f"{u:24} {len(rows):>6} | {cells}")
    print("\nHand-coded samples (P praise / N complaint / 0 neutral):")
    for (tool, p), lab in HAND.items():
        n = sum(len(v) for v in lab.values())
        missing = [i for v in lab.values() for i in v if i not in by_id]
        print(f"  {tool:16} {p}: n={n:>2}  P={len(lab['P']):>2}  N={len(lab['N']):>2}  0={len(lab['Z']):>2}"
              + (f"  MISSING {missing}" if missing else ""))
    print("\nTool mentions in PDF comments; pos/neg = mentions with a positive/negative word within 200 chars;"
          " net = (pos-neg)/(pos+neg)")
    agg = {}
    for _, p, _, tools in results:
        for name, (pos, neg) in tools.items():
            a = agg.setdefault(name, {q: [0, 0, 0] for q in PERIODS})
            a[p][0] += 1
            a[p][1] += pos
            a[p][2] += neg
    for name, a in sorted(agg.items(), key=lambda kv: -sum(v[0] for v in kv[1].values())):
        cells = []
        for q in PERIODS:
            n, pos, neg = a[q]
            net = (pos - neg) / (pos + neg) if pos + neg else 0
            cells.append(f"{q}: n={n:>4} ({1000 * n / totals[q]:4.1f}/1k) +{pos:>3} -{neg:>3} net={net:+.2f}")
        print(f"{name:24} " + " | ".join(cells))
    show = rx(r"show hn")
    tool_story = rx(r"pdf")
    local = rx(r"browser|client[- ]side|local|wasm|webassembly|no upload|without upload|privacy|private|offline|"
               r"self-hosted|no back ?end")
    edit = rx(r"edit|tools?|merg|split|sign|redact|annotat|form|compress|rotate|scan|watermark")
    print("\nShow HN PDF editing/tool stories (title: show hn + pdf + edit/tool word), and those also saying local/browser:")
    for q in PERIODS:
        rows = [s for s in stories.values() if period(s["created_at"]) == q and show.search(s["title"] or "")
                and tool_story.search(s["title"] or "") and edit.search(s["title"] or "")]
        loc = [s for s in rows if local.search(s["title"] or "")]
        pts = sorted((s.get("points") or 0) for s in loc)
        print(f"  {q}: {len(rows)} stories; {len(loc)} local/browser; their median points "
              f"{pts[len(pts) // 2] if pts else 0}")
    all_by_year = Counter(s["created_at"][:4] for s in stories.values())
    for y in sorted(all_by_year):
        rows = [s for s in stories.values() if s["created_at"][:4] == y and show.search(s["title"] or "")
                and tool_story.search(s["title"] or "") and edit.search(s["title"] or "")]
        loc = [s for s in rows if local.search(s["title"] or "")]
        print(f"  {y}: {len(rows):>3} of {all_by_year[y]:>3} PDF stories ({100 * len(rows) / all_by_year[y]:4.1f}%);"
              f" local/browser {len(loc)}")


if __name__ == "__main__":
    main()
