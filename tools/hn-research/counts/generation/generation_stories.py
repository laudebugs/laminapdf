"""Stories per generation family and period from data/stories.jsonl (title regex), with total points.
Usage: python3 -I generation_stories.py <hn_dir>
"""
import collections, json, os, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from generation_core import period
FAM = {
    "Typst": r"typst",
    "LaTeX/TeX": r"latex|\btex\b|tex live|luatex|pdftex",
    "HTML/CSS to PDF (wkhtmltopdf, headless Chrome, WeasyPrint, Prince, Paged.js, APIs)":
        r"html.{0,12}pdf|pdfs? from html|wkhtml|weasyprint|prince|paged\.?js|headless|puppeteer|gotenberg|web ?page to pdf|"
        r"url to pdf|plutoprint|plutobook|htmldocs|relaxed|onedoc|pdf-bot|print css|printable pdf",
    "code libraries (jsPDF, pdf-lib, ReportLab, iText, QuestPDF, gofpdf ...)":
        r"jspdf|pdf-lib|reportlab|itext|openpdf|questpdf|gofpdf|fpdf|tcpdf|hexapdf|capypdf|tinypdf|pdf librar|pdfkit|react-pdf|"
        r"folio|pdf (engine|generation)",
    "Markdown to PDF": r"markdown.{0,20}pdf|\bmd\b.{0,10}pdf|mdpdf",
    "invoices, resumes, certificates": r"invoice|resume|\bcv\b|certificate",
}
rows = [json.loads(l) for l in open(f"{sys.argv[1]}/data/stories.jsonl")]
print("| family | stories 16-19 / 20-22 / 23-26 | points 16-19 / 20-22 / 23-26 | stories ≥ 100 points |")
print("|---|---|---|---|")
for name, rx in FAM.items():
    rx = re.compile(rx, re.I)
    n, p, big = collections.Counter(), collections.Counter(), 0
    for s in rows:
        if rx.search(s["title"] or ""):
            q = period(s["created_at"]); n[q] += 1; p[q] += s["points"] or 0; big += (s["points"] or 0) >= 100
    ps = ["2016-19", "2020-22", "2023-26"]
    print(f"| {name} | {' / '.join(str(n[q]) for q in ps)} | {' / '.join(str(p[q]) for q in ps)} | {big} |")
