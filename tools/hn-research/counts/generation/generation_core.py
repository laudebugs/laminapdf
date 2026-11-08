"""Shared definitions for the generation analysis: the core subset of areas/generation.jsonl.

Lane A (HTML and code generation): the comment names an HTML-to-PDF tool or a PDF-generation library, or uses
a phrase about producing PDFs (generate/create/render/export a PDF, invoices, certificates, paged media, print CSS),
or sits in a thread whose title is about HTML/code PDF generation.
Lane B (typesetting): the comment names a typesetting system and a document cue (pdf, paper, compile, ...) (LaTeX, TeX, Typst, Pandoc, groff/troff, SILE, ConTeXt,
Quarto, AsciiDoc, Overleaf, Tectonic, InDesign, Scribus, TeXmacs) or sits in a thread whose title is about one.
Core = A or B. The loader also adds every comment of a thread whose title matches TITLE_A or TITLE_B
(from data/threads.jsonl and comment_hits.jsonl) that the area file left out. Full comment text is taken from data/threads.jsonl / comment_hits.jsonl (not the 1,500-char cut).
"""
import json, re

HTML = (r"wkhtml|puppeteer|playwright|headless ?(chrome|chromium|browser|firefox)|chrome --headless|chromium --headless|"
        r"print[- ]?to[- ]?pdf|weasyprint|princexml|prince ?xml|\bprince\b(?=[^.]{0,60}(pdf|css|html|docraptor|weasy|licen|engine))|(pdf|css|html|docraptor|weasy|paged)[^.]{0,60}\bprince\b|docraptor|paged\.?js|pagedjs|gotenberg|browserless|vivliostyle|"
        r"flying ?saucer|openhtmltopdf|apache fop|\bfop\b|xsl-?fo|pdfreactor|antenna ?house|typeset\.sh|plutoprint|"
        r"plutobook|html ?(to|2|->|→) ?pdf|html-to-pdf|htmltopdf|phantomjs|athenapdf|api2pdf|pdfshift|html2pdf|html2canvas")
LIB = (r"reportlab|\bitext|\bpdfkit|jspdf|pdf-lib|pdfmake|\bfpdf|tcpdf|dompdf|\bmpdf|questpdf|react-pdf|reactpdf|pdfbox|"
       r"openpdf|\bprawn\b|gofpdf|hexapdf|pdfsharp|migradoc|aspose|\bpdflib\b|pdf::api2|libharu|printpdf|lopdf|krilla|"
       r"pdf-writer|\bborb\b|wicked_?pdf|jasper|crystal reports|ironpdf|docspring|carbone|docmosis|onedoc|"
       r"htmldocs|relaxed|pdf-bot|capypdf|tinypdf|pdf (generation )?librar")
GEN = (r"generat\w* (a |the |these |those |my |your |our )?pdfs?\b|pdf[- ]generat|creat\w* (a |the )?pdfs?\b|pdf creat|"
       r"produc\w* (a |the )?pdfs?\b|render\w* (it |them )?(to|into|as) (a )?pdf|export\w* (it |them )?(to|as) (a )?pdf|"
       r"save as pdf|(pdf|generat\w*|templat\w*|render\w*|print\w*|creat\w*|produc\w*|export\w*)[^.]{0,60}invoic|"
       r"invoic\w*[^.]{0,60}(pdf|generat|templat|render|print|creat|produc|export)|paged media|print css|css print|@page")
TYP = (r"\blatex|pdflatex|xelatex|lualatex|luatex|\btex\b|typst|pandoc|groff|troff|\bsile\b|quarto|asciidoc|overleaf|"
       r"tectonic|tex ?live|\bindesign|scribus|texmacs")
TITLE_A = (r"html.{0,6}(to|into|->|→|2).{0,4}pdf|wkhtml|weasyprint|prince|paged\.?js|puppeteer|headless|gotenberg|"
           r"docraptor|reportlab|\bitext|pdfkit|jspdf|pdf-lib|fpdf|tcpdf|dompdf|questpdf|react-pdf|invoice|onedoc|htmldocs|"
           r"relaxed|pdf-bot|print css|generat\w* (pdf|document|invoice)|pdf (api|generat|creat|engine|librar)|"
           r"(make|making|creat\w*|generat\w*|build\w*|produc\w*) (a |the )?pdfs?|pdfs? from (html|code|markdown|templates?|web)|webpage to pdf|url to pdf|web page to pdf|plutoprint|plutobook|tinypdf|"
           r"papermill|rubrol|capypdf|hexapdf|folio|placecard|screencraft|openpdf|text-selectable|zero-browser|"
           r"websites should be printable|resume|\bcv\b|certificate")
TITLE_B = (r"typst|latex|\btex\b|pandoc|groff|troff|\bsile\b|typeset|quarkdown|quarto|tectonic|texmacs|markdown.{0,12}pdf|"
           r"markdown is holding|thesis")

JOB_THREADS = re.compile(r"who wants to be hired|who is hiring|freelancer\? seeking|seeking freelancer", re.I)
TITLE_NOT = re.compile(r"\bocr\b|searchable|look scanned|extract|\bpars(e|ing)\b", re.I)  # extraction threads
RX = {k: re.compile(v, re.I) for k, v in dict(HTML=HTML, LIB=LIB, GEN=GEN, TYP=TYP, TITLE_A=TITLE_A, TITLE_B=TITLE_B).items()}
CONTEXT = re.compile(r"ConTeXt")  # case-sensitive: the TeX format, not the word "context"
DOC_CUE = re.compile(r"pdf|document|paper|thesis|typeset|compil|template|book|resume|\bcv\b|report|fonts?\b|pages?\b|layout|"
                     r"journal|article|slides|beamer|output|print", re.I)


def period(date):
    y = int(date[:4])
    return "2016-19" if y <= 2019 else ("2020-22" if y <= 2022 else "2023-26")


def load(root, with_threads=True):
    full = {}
    for src in ("data/threads.jsonl", "data/comment_hits.jsonl"):
        for line in open(f"{root}/{src}"):
            c = json.loads(line)
            full.setdefault(int(c["id"]), c.get("text") or "")
    rows = [json.loads(line) for line in open(f"{root}/areas/generation.jsonl")]
    if with_threads:  # add every comment of a generation-titled thread that the area file left out
        have = {r["id"] for r in rows}
        for src in ("data/threads.jsonl", "data/comment_hits.jsonl"):
            for line in open(f"{root}/{src}"):
                c = json.loads(line)
                i, title, text = int(c["id"]), c.get("story_title") or "", c.get("text") or ""
                if i in have or len(text) < 40:
                    continue
                if (RX["TITLE_A"].search(title) or RX["TITLE_B"].search(title)) and not TITLE_NOT.search(title):
                    have.add(i)
                    rows.append({"id": i, "date": (c.get("created_at") or "")[:10], "story_id": c.get("story_id"),
                                 "story": title, "thread_comments": None, "text": text, "added": True})
    out, seen_text = [], set()
    for r in rows:
        t = full.get(r["id"], r["text"])
        key = " ".join(t.split())
        if key in seen_text:  # drop exact duplicate texts (double posts)
            continue
        seen_text.add(key)
        title = r["story"] or ""
        a = bool(RX["HTML"].search(t) or RX["LIB"].search(t) or RX["GEN"].search(t) or RX["TITLE_A"].search(title))
        b = bool(((RX["TYP"].search(t) or CONTEXT.search(t)) and DOC_CUE.search(t)) or RX["TITLE_B"].search(title))
        if JOB_THREADS.search(title):  # job-seeking posts list tools without discussing them
            a = b = False
        r.update(text=t, laneA=a, laneB=b, core=a or b, period=period(r["date"]))
        out.append(r)
    return out
