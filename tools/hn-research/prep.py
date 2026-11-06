"""Merge the collected comments, sort them into research areas, and count mentions.

Reads data/threads.jsonl, data/comment_hits.jsonl, data/stories.jsonl; writes
areas/<area>.jsonl (comments for that area, most-discussed threads first, text cut to 1,500
characters) and stats/*.json (tool mentions and area volume by year).
"""

import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(sys.argv[1])
DATA, AREAS, STATS = ROOT / "data", ROOT / "areas", ROOT / "stats"
AREAS.mkdir(exist_ok=True)
STATS.mkdir(exist_ok=True)

PDF = re.compile(r"\bpdfs?\b|pdf\.js|pdfjs|pdfium|mupdf|poppler|acrobat|ghostscript|wkhtmltopdf|weasyprint|typst|"
                 r"sumatra|okular|zathura|foxit|pdfbox|itext|qpdf|pdftk|docling|pdfplumber|pymupdf", re.I)

AREA_RE = {
    "readers": r"pdf (reader|viewer)s?|acrobat reader|adobe reader|\breader app|macos preview|preview\.app|apple'?s preview|"
               r"\bin preview\b|sumatra|okular|zathura|evince|foxit|pdf\.js|pdfjs|\bskim\b|xodo|pdf expert|dark mode|"
               r"\breflow|e-?readers?\b|kindle|remarkable|kobo|\bipad\b|browser'?s? (built-in )?(pdf )?viewer|"
               r"chrome'?s? (pdf )?viewer|firefox'?s? (pdf )?viewer|edge'?s? (pdf )?viewer|scrolling|zoom",
    "editors": r"pdf editors?|edit(ing)? (a |the )?pdfs?|annotat|\bhighlight|fill(ing)? (out |in )?(the |a )?(pdf )?forms?|"
               r"form fill|acroform|\bxfa\b|sign(ing)? (a |the )?pdf|signatures?|e-?sign|docusign|redact|"
               r"\bmerg(e|ing)\b|\bsplit(ting)?\b|compress|stirling|pdftk|qpdf|acrobat (pro|dc|standard)|ocrmypdf|"
               r"rotate|reorder|bookmark|watermark|flatten",
    "parsing": r"\bpars(e|es|ed|ing|er|ers)\b|extract|\btables?\b|\bocr\b|tesseract|layout|reading order|markdown|\brag\b|"
               r"\bllms?\b|embedding|chunk|pdfplumber|pymupdf|\bfitz\b|pdfminer|camelot|tabula|\btika\b|docling|"
               r"\bmarker\b|unstructured|llamaparse|mistral ocr|nougat|grobid|gemini|gpt-?4|\bclaude\b|pdftotext",
    "generation": r"\bgenerat(e|es|ed|ing|ion|or)\b|html to pdf|html-to-pdf|wkhtmltopdf|weasyprint|prince ?(xml)?|"
                  r"paged\.?js|puppeteer|headless chrome|playwright|latex|typst|reportlab|itext|pdfkit|jspdf|pdf-lib|"
                  r"\bfpdf|tcpdf|dompdf|\binvoices?\b|templat|print css|@page|typeset",
    "format_security": r"\bspec(ification)?\b|iso 32000|pdf 2\.0|pdf/a|pdf/ua|tagged pdf|accessib|screen readers?|"
                       r"\bsecurity\b|vulnerab|exploit|malware|javascript|\bcve\b|sandbox|licen[cs]e|\bagpl\b|\bgpl\b|"
                       r"metadata|incremental|forensic|postscript|\bfonts?\b|archiv|unfit|hate pdf|pdfs are",
}
AREA_RE = {k: re.compile(v, re.I) for k, v in AREA_RE.items()}

TOOLS = {
    "Adobe Acrobat/Reader": r"\bacrobat\b|adobe reader", "macOS Preview": r"macos preview|preview\.app|apple'?s preview|\bin preview\b",
    "pdf.js / Firefox viewer": r"pdf\.js|pdfjs|firefox'?s? (pdf )?(viewer|reader)", "Chrome/Edge viewer": r"chrome'?s? (pdf )?(viewer|reader)|edge'?s? (pdf )?(viewer|reader)",
    "SumatraPDF": r"sumatra", "Okular": r"okular", "Zathura": r"zathura", "Evince": r"evince", "Foxit": r"foxit",
    "Skim": r"\bskim\b", "PDF Expert": r"pdf expert", "Xodo": r"xodo",
    "PDFium": r"pdfium", "MuPDF/PyMuPDF": r"mupdf|pymupdf|\bfitz\b", "Poppler/pdftotext": r"poppler|pdftotext|pdftoppm",
    "Ghostscript": r"ghostscript", "qpdf": r"\bqpdf\b", "pdftk": r"pdftk", "PDFBox": r"pdfbox", "iText": r"\bitext\b",
    "pdfminer": r"pdfminer", "pdfplumber": r"pdfplumber", "Tika": r"\btika\b", "Camelot/Tabula": r"camelot|tabula",
    "Tesseract": r"tesseract", "OCRmyPDF": r"ocrmypdf", "Docling": r"docling", "Marker": r"\bmarker\b",
    "Mistral OCR": r"mistral ocr|mistral'?s ocr", "Gemini": r"gemini", "GPT/OpenAI": r"\bgpt-?\d|openai|chatgpt",
    "Claude": r"\bclaude\b", "Unstructured": r"unstructured", "LlamaParse": r"llamaparse|llama ?parse",
    "wkhtmltopdf": r"wkhtmltopdf", "Puppeteer/headless Chrome": r"puppeteer|headless chrom|playwright",
    "WeasyPrint": r"weasyprint", "Prince": r"princexml|prince xml|\bprince\b", "Paged.js": r"paged\.?js",
    "LaTeX": r"\blatex\b|pdflatex|xelatex|lualatex", "Typst": r"typst", "ReportLab": r"reportlab",
    "jsPDF/pdf-lib": r"jspdf|pdf-lib|pdflib", "Stirling-PDF": r"stirling", "DocuSign": r"docusign",
    "pdf2htmlEX": r"pdf2htmlex", "Pandoc": r"pandoc",
}
TOOLS = {k: re.compile(v, re.I) for k, v in TOOLS.items()}


def main() -> None:
    stories = {s["objectID"]: s for s in map(json.loads, open(DATA / "stories.jsonl"))}
    comments: dict[int, dict] = {}
    for src in ("threads.jsonl", "comment_hits.jsonl"):
        for line in open(DATA / src):
            c = json.loads(line)
            c["id"] = int(c["id"])
            if len(c.get("text") or "") < 40:
                continue
            c.setdefault("source", src.split(".")[0])
            comments.setdefault(c["id"], c)
    print("unique comments:", len(comments), file=sys.stderr)

    per_area: dict[str, list] = defaultdict(list)
    tool_year: dict[str, Counter] = defaultdict(Counter)
    area_year: dict[str, Counter] = defaultdict(Counter)
    for c in comments.values():
        text, year = c["text"], (c.get("created_at") or "????")[:4]
        in_pdf_thread = str(c.get("story_id")) in stories
        if not (in_pdf_thread or PDF.search(text)):
            continue
        for tool, rx in TOOLS.items():
            if rx.search(text):
                tool_year[tool][year] += 1
        story = stories.get(str(c.get("story_id"))) or {}
        weight = story.get("num_comments") or 0
        for area, rx in AREA_RE.items():
            if rx.search(text):
                area_year[area][year] += 1
                per_area[area].append({
                    "id": c["id"], "date": (c.get("created_at") or "")[:10], "story_id": c.get("story_id"),
                    "story": c.get("story_title"), "thread_comments": weight, "text": text[:1500],
                })
    for area, rows in per_area.items():
        rows.sort(key=lambda r: (-(r["thread_comments"] or 0), r["date"]))
        with open(AREAS / f"{area}.jsonl", "w") as f:
            for r in rows:
                f.write(json.dumps(r) + "\n")
        print(area, len(rows), file=sys.stderr)
    (STATS / "tool_mentions_by_year.json").write_text(json.dumps({k: dict(sorted(v.items())) for k, v in tool_year.items()}, indent=1))
    (STATS / "area_volume_by_year.json").write_text(json.dumps({k: dict(sorted(v.items())) for k, v in area_year.items()}, indent=1))
    totals = sorted(((sum(v.values()), k) for k, v in tool_year.items()), reverse=True)
    for n, k in totals:
        print(f"{k:28} {n}", file=sys.stderr)


if __name__ == "__main__":
    main()
