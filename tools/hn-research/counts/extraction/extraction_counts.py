"""Theme and tool counts for the extraction note (docs/research/hacker-news/notes/extraction.md).

Run:  HN_DATA=<dir> python3 -I tools/hn-research/counts/extraction/extraction_counts.py [--samples THEME N]
Base: areas/parsing.jsonl minus hiring/freelancer/Typst threads, then the extraction-focused
subset REL (comment text or thread title matches REL). Each theme counts comments whose text
matches its regex (case-insensitive), by period. Rates are per 1,000 REL comments of the period.
"""
import json, re, sys, random, collections
import os
from pathlib import Path

ROOT = Path(os.environ.get("HN_DATA", "."))
rows = [json.loads(l) for l in open(ROOT / "areas/parsing.jsonl")]
NOISE = re.compile(r"who wants to be hired|who is hiring|freelancer\?|seeking freelancer|typst", re.I)
REL = re.compile(
    r"extract|\bocr\b|ocr'?d|tesseract|pdftotext|text layer|copy.?(and.?)?past|"
    r"pars(e|es|ed|ing|er|ers)\b.{0,40}pdf|pdfs?\b.{0,40}pars(e|es|ed|ing|er|ers)\b|"
    r"pdf (to|into) (text|markdown|md|html|json|csv|excel|xlsx|word|docx|data|structured)|"
    r"convert\w* (\w+ ){0,3}pdfs? (to|into)|\brag\b|ingest|textract|"
    r"document (ai|intelligence|understanding|parsing)|form recognizer|reading order|"
    r"layout (analysis|detection|model|understanding|parser)|pdfplumber|pymupdf|\bfitz\b|pdfminer|"
    r"pdfbox|camelot|tabula|\btika\b|docling|\bmarker\b|unstructured\.io|llamaparse|llama ?parse|"
    r"mistral ocr|nougat|grobid|olmocr|mathpix|paddleocr|surya|easyocr|doctr|abbyy|finereader|"
    r"ocrmypdf|vision (model|llm|language)|\bvlms?\b|multimodal|multi-modal", re.I)

def period(d):
    y = int(d[:4])
    return "2016-19" if y <= 2019 else ("2020-22" if y <= 2022 else "2023-26")

PERIODS = ["2016-19", "2020-22", "2023-26"]
base = [r for r in rows if not NOISE.search(r["story"] or "")]
rel = [r for r in base if REL.search(r["text"]) or REL.search(r["story"] or "")]

W = r"(?:\w+\W+){0,4}?"
THEMES = {
    # --- wants ---
    "tables": r"\btables?\b(?! of contents)(?<!off the table)(?<!on the table)(?<!at the table)|tabular",
    "tables_hard_cases": r"merged cells?|spann?(ing|ed|s) (multiple )?(rows|columns|cells|pages)|multi-?page tables?|"
                         r"tables? (that )?(span|spanning|across) (multiple |several )?pages|borderless|"
                         r"(no|without) (borders|lines|ruling|grid ?lines)|empty cells?|nested tables?|"
                         r"rest of (the )?table",
    "reading_order_columns": r"reading order|multi-?columns?|two.?columns?|double.?columns?|three.?columns?|"
                             r"\bcolumns?\b.{0,40}(text|layout|order|page)|(text|layout|order|page).{0,40}\bcolumns?\b",
    "layout_preserve": r"-layout\b|--layout|keep\w* (the )?layout|preserv\w* (the )?(original )?layout|"
                       r"layout (analysis|detection|model|understanding|parser|information|info)|layout-?aware",
    "scans_ocr": r"scann?ed|\bscans?\b|\bocr\b|ocr'?d|tesseract",
    "handwriting": r"handwrit|hand-written|cursive",
    "llm_formats_rag": r"markdown|\brag\b|chunk|embedding|vector (db|store|database|search)|json",
    "llm_output": r"(to|into|as|outputs?|convert\w*|extract\w*|generat\w*|produc\w*|return\w*)\W+(?:\w+\W+){0,3}?"
                  r"(markdown|json)\b|markdown (output|conversion|export)|(pdfs?|page|document)\W{0,3}(->|→|2|to) ?"
                  r"(markdown|md|json)\b|\brag\b|chunk\w*|embeddings?|vector (db|store|database|search|index)|"
                  r"(for|into|to) (an? |the )?llms?\b|llm.?(ready|friendly)",
    "chunking": r"chunk",
    "bbox_coords": r"bounding.?box|\bbbox|coordinates|positions? (on|of|in) (the )?(page|text|words?|characters?|glyphs?)",
    "citations_grounding": r"grounding|\bgrounded\b|source citations?|citations?.{0,40}(page|location|bounding|coordinates|"
                           r"exact|precise|highlight)|(page|exact|precise) locations?|cite (the )?(source|page)|"
                           r"highlight\w* (the )?(source|location|extracted)|traces? back to|attribution",
    "structure_headings": r"headings?\b|section (headers?|titles?|headings?)|hierarch|document structure|"
                          r"semantic structure|tagged pdf|structure tree|struct tree|logical structure|"
                          r"\btitles?, |outline|table of contents|\btoc\b",
    "tagged_pdf": r"tagged pdfs?|pdf/ua|structure tree|struct tree|accessibility tree|accessible pdfs?|"
                  r"marked content|alt.?text|screen readers?|pdf tags|\btags? (in|of) (the )?pdf",
    "math_formulas": r"equations?|formula(s|e)?\b|mathpix|nougat|math(ematical)? (equations|formulas|expressions|notation|symbols)|"
                     r"\bmath\b.{0,40}(latex|ocr|extract|markdown|pdf)|(latex|ocr|extract|markdown|pdf).{0,40}\bmath\b",
    "figures_charts": r"\bcharts?\b|\bgraphs?\b|figures?\b|diagrams?|infographics?|plots?\b|extract\w* (the )?images",
    "use_invoices_receipts": r"invoices?|receipts?",
    "use_financial": r"bank statements?|financial (statements?|reports?|documents?|data|filings?)|\b10-?k\b|"
                     r"sec filings?|annual reports?|brokerage|\b1099|tax (forms?|returns?|documents?)",
    "use_forms": r"\bforms?\b|acroform|form fields?|checkbox",
    "use_papers": r"scientific|academic|papers?\b|arxiv|journal",
    "use_legal_gov": r"contracts?|leases?|legal|court|lawyers?|government|foia|regulat",
    "use_books_historic": r"\bbooks?\b|old (documents|books|scans)|historic|archiv|genealog|newspapers?|manuscripts?",
    "local_privacy": r"privacy|on.?prem|self.?host(ed|ing)?|offline|run\w* (it |them |this |everything )?locally|"
                     r"local (model|llm|vlm|ocr|inference|setup|pipeline|gpu)s?|confidential|"
                     r"sensitive (data|documents?|information|files?)|hipaa|gdpr|"
                     r"(upload|send)\w* " + W + r"to (a |the )?(cloud|third|server|google|openai|anthropic|microsoft|amazon)",
    "embed_data_in_pdf": r"embed\w* (the )?(source|data|xml|original|csv|json|editable)|zugferd|factur-?x|"
                         r"xrechnung|hybrid pdf|attach\w* (the )?(data|csv|source|spreadsheet)|machine.?readable",
    "determinism": r"determinis|reproducib|same (input|document|pdf|prompt|page|file)s?.{0,60}(different|differ)|"
                   r"(slightly |completely )?different (results|output|outputs|answers) (each|every)|"
                   r"(each|every) (time|run).{0,30}different|inconsistent (results|output|outputs|answers)|"
                   r"consistent (results|output|outputs)|run (it )?(two|three|multiple) times",
    "benchmarks": r"benchmark|ground.?truth|leaderboard|olmocr-?bench|omnidocbench|edit distance|needleman|"
                  r"\beval(uation|s)?\b|test (set|suite|corpus)",
    "confidence": r"confidence",
    "speed_scale": r"\bfast(er|est)?\b|\bslow(er|est|ly|ness)?\b|throughput|pages? ?(/|per) ?(s|sec|second|minute|hour)|"
                   r"latency|millions? of (pdfs|pages|documents|files)|at scale|\bgpus?\b",
    "cost": r"\$ ?\d|per page|/ ?page|per (1,?000|1k|thousand) pages|\bcosts?\b|cheap|expensive|pricing|price|pricey",
    "scanned_docs": r"scann?ed|\bscans?\b",
    "structured_extraction": r"json schema|\bschemas?\b|structured (output|outputs|data|extraction|json)|key.?value|"
                             r"form fields|extract\w* (the |specific |key )?(fields|values|line items|entities)|line items",
    "hybrid_ocr_llm": r"(ocr|tesseract|textract|paddle)\W+(?:\w+\W+){0,12}?(llm|gpt|gemini|claude|language model)\w*\W+"
                      r"(?:\w+\W+){0,8}?(correct|fix|clean|check|verif)|"
                      r"(llm|gpt|gemini|claude)\w*\W+(?:\w+\W+){0,6}?(correct|fix|clean)\w*\W+(?:\w+\W+){0,3}?(ocr|errors)",
    "native_text_first": r"(text layer|embedded text|native text|born.?digital|digitally (born|generated|created)|"
                         r"text-based pdfs?|vector pdfs?)",
    "pdf_wrong_format": r"pdf (is|was) (designed|meant|made|intended|built) (for|to|as)|"
                        r"pdfs? (are|is) (a |an )?(terrible|awful|horrible|bad|nightmare|hell|the worst)|pdf hell|"
                        r"where data goes to die|(should|could) (be|have been) (published|provided|released|distributed|sent) "
                        r"(as|in) (html|csv|json|xml|excel|text|data|a )|(publish|provid|releas|send)\w* (the |their |raw )?"
                        r"(data|tables?|it|them) (as|in) (csv|html|json|xml|excel|a spreadsheet)|machine.?readable|"
                        r"(not|never) (meant|designed|intended|made) (to|for) (be )?(pars|extract|edit|machine|data|read by)|"
                        r"characters? (at|with|and their) (coordinates|positions)|digital paper|presentation format",
    # --- complaints ---
    "cost_complaint": r"expensive|pricey|overpriced|too costly|costs? (too much|a fortune)|can't afford|"
                      r"cost (is|was) (too )?(high|prohibitive)|prohibitive",
    "slow_complaint": r"\bslow(er|est|ly|ness)?\b|takes? (forever|ages|minutes)|minutes? per page|seconds? per page",
    "garbled_text": r"tounicode|to.?unicode|\bcmap|garbl|gibberish|mojibake|ligatures?|cid.?fonts?|type ?3 fonts?|"
                    r"copy.?(and.?)?past\w*\W+(?:\w+\W+){0,10}?(gibberish|garbage|rubbish|nonsense|broken|wrong|mess|"
                    r"junk|garbled|weird)|"
                    r"missing spaces|extra spaces|spurious spaces|surprise spaces|random spaces|"
                    r"spaces? (in|between|within|inside) (the )?(middle of )?(letters|characters|words|chars)",
    "pdf_wrong_format": r"pdf (is|was) (designed|meant|made|intended|built) (for|to|as)|"
                        r"pdfs? (are|is) (a |an )?(terrible|awful|horrible|bad|nightmare|hell|the worst)|pdf hell|"
                        r"where data goes to die|(should|could) (be|have been) (published|provided|released|distributed|sent) "
                        r"(as|in) (html|csv|json|xml|excel|text|data|a )|(publish|provid|releas|send)\w* (the |their |raw )?"
                        r"(data|tables?|it|them) (as|in) (csv|html|json|xml|excel|a spreadsheet)|machine.?readable|"
                        r"(not|never) (meant|designed|intended|made) (to|for) (be )?(pars|extract|edit|machine|data|read by)|"
                        r"characters? (at|with|and their) (coordinates|positions)|digital paper|presentation format",
    # --- complaints ---
    "hallucination": r"hallucinat|made.?up|makes? (things|stuff|it) up|making (things|stuff) up|fabricat|confabul|"
                     r"plausibl",
    "dropped_content": r"(skip|skips|skipped|skipping|omit|omits|omitted|omitting|drops|dropped|dropping|"
                       r"misses|missed|ignores|ignored|ignoring|truncates|truncated|truncating|leaves out|left out)\W+"
                       + W + r"(text|content|parts?|rows?|pages?|sections?|lines?|words?|paragraphs?|cells?|"
                       r"tables?|figures?|footnotes?|characters?|sentences?)|missing (text|content|rows?|lines?|words?|"
                       r"sentences?|paragraphs?|cells?|parts? of)|rest of (the )?table|silently (drop|skip|omit|ignor|miss)|"
                       r"\bomission",
    "numbers_misread": r"misread|mis-?recogni[sz]|(wrong|incorrect|swapped|transposed|changed|altered|substitut\w*) "
                       r"(digits?|numbers?|figures|values|amounts|decimals?|dates)|(digits?|numbers?|values|amounts|"
                       r"decimals?) (are |were |get |got |being )?(wrong|incorrect|swapped|transposed|changed|altered|"
                       r"misread|substituted)|jbig2|decimal (point|place)s?",
    "encoding_fonts": r"tounicode|to.?unicode|\bcmap|glyphs?|garbl|gibberish|mojibake|"
                      r"(font|text|character|unicode)s? encod(ing|ings|ed)|encod(ing|ings|ed) (of )?(the )?(fonts?|text|characters?|glyphs?)|"
                      r"ligatures?|cid.?fonts?|type ?3 fonts?|custom fonts?|font (subset|mapping|encoding)|"
                      r"unicode (map|mapping)|private use",
    "spacing_hyphen": r"spaces? (in|between|within|inside) (the )?(middle of )?(letters|characters|words|chars)|"
                      r"missing spaces|extra spaces|spurious spaces|surprise spaces|random spaces|"
                      r"word (boundaries|breaks|splitting)|split words|words? (are |get )?split|hyphen|"
                      r"line breaks?|newlines",
    "copy_paste": r"copy.?(and.?)?past(e|ing)|copy text|select (the )?text|selectable",
    "headers_footers": r"headers? (and|&|/) ?footers?|footers?|page numbers?|running (heads?|headers?)|"
                       r"repeated headers?|watermarks?|page (breaks?|furniture)",
    "footnotes": r"footnotes?|endnotes?",
    "pdf_format_blame": r"characters? (at|with|and their) (coordinates|positions)|glyphs? (at|with) (positions|coordinates)|"
                        r"no (semantic|structure)|lacks? (any )?(semantic|structure)|"
                        r"(not|never) (meant|designed|intended|made) (to|for) (be )?(pars|extract|edit|machine|data|read by)|"
                        r"presentation format|print(ing)? format|digital paper|(should|could) (be|have been) "
                        r"(published|provided|released|distributed|sent) (as|in) (html|csv|json|xml|text|data|a)|"
                        r"where data goes to die|drawing (commands|instructions)",
    "brittle_templates": r"(layout|format|template)s? (change|changes|changed|breaks?|broke)|parser per|"
                         r"(one|a|new) (parser|template|rule) (for|per) (each|every)|per (supplier|vendor|bank|layout|"
                         r"format|template|source)|custom parsers?|bespoke parsers?|brittle|fragile|"
                         r"break(s)? (when|whenever|every time)",
    "install_gpu_pain": r"pytorch|\bcuda\b|\bgpus?\b|dependenc(y|ies)|install(ation|ing)? (was|is|took|hell)|"
                        r"dependency hell|gigabytes? of|python (dependency|environment)",
    "licensing": r"\bagpl\b|\bgpl\b|non-?commercial|cc.?by.?nc|licen[cs](e|es|ed|ing)\b|copyleft",
    "refusals_injection": r"refus(e|es|ed|al|ing)|safety (filters?|settings?)|censor|prompt injection|"
                          r"instructions? (in|inside|within|hidden in) (the )?(document|text|pdf|scan|page)|"
                          r"rogue instruction|recitation|copyright (filter|block)",
    "multilingual": r"multilingual|multi-lingual|non-?english|other languages|foreign (text|language)|hebrew|arabic|"
                    r"chinese|japanese|korean|\bcjk\b|cyrillic|greek|right.?to.?left|\brtl\b|\bbidi\b|fraktur|"
                    r"devanagari|hindi|thai|vietnamese|urdu|persian|farsi",
    "malformed_files": r"malformed|corrupt(ed)?|broken (pdfs?|files?|xref)|invalid pdfs?|fails? to (open|parse|load)|"
                       r"crash(es|ed|ing)?|\brepair",
    "hidden_text": r"hidden text|invisible text|white (text|on white)|white-on-white|text (that is |that's )?(not )?visible|"
                   r"(different|other) (set of )?text (than|from) (what|the)|text layer (doesn't|does not) match",
    "vlm_render_to_image": r"(render|convert|turn|rasteri[sz]e|screenshot)\w* " + W + r"(to |into |as )?(an? )?"
                           r"(images?|png|jpe?g|raster|bitmaps?|screenshots?)|rasteri[sz]|page images?|"
                           r"pdf2image|pdftoppm|screenshot",
    "llm_vlm_any": r"\bllms?\b|\bvlms?\b|gpt|chatgpt|openai|claude|gemini|mistral|vision (model|llm|language)|"
                   r"multimodal|multi-modal|qwen|llama|olmocr|language models?",
    "human_review": r"human (review|in the loop|verification|check)|manual(ly)? (review|check|correct|verif|fix)|"
                    r"double.?check|spot.?check|human-in-the-loop|by hand",
}
TOOLS = {
    "Tesseract": r"tesseract",
    "Poppler/pdftotext": r"poppler|pdftotext|pdftohtml|pdftoppm|xpdf",
    "PDFBox": r"pdfbox",
    "pdfminer": r"pdfminer",
    "pdfplumber": r"pdfplumber",
    "PyMuPDF/MuPDF": r"pymupdf|mupdf|\bfitz\b|pymupdf4llm",
    "Tika": r"\btika\b",
    "Tabula": r"tabula",
    "Camelot": r"camelot",
    "ABBYY": r"abbyy|finereader",
    "AWS Textract": r"textract",
    "Azure Doc Intelligence": r"document intelligence|form recognizer|azure (ocr|document|form)",
    "Google Doc AI/Vision": r"document ?ai|cloud vision|google (vision|ocr|cloud ocr)|gcp vision|vision api",
    "GROBID": r"grobid",
    "Nougat": r"nougat",
    "Marker": r"\bmarker\b(?!s)",
    "Surya": r"\bsurya\b",
    "Docling": r"docling",
    "Unstructured": r"unstructured\.io|unstructured-io|unstructured's|\bUnstructured\b(?= (api|io|library|lib|package|does|is|has|was|gives|works))",
    "LlamaParse": r"llamaparse|llama ?parse",
    "Mistral OCR": r"mistral ?ocr|mistral's ocr|mistral-ocr",
    "olmOCR": r"olmocr",
    "Mathpix": r"mathpix",
    "PaddleOCR": r"paddle ?ocr|paddle-?ocr|\bpaddle\b",
    "EasyOCR": r"easyocr",
    "docTR": r"\bdoctr\b",
    "OCRmyPDF": r"ocrmypdf",
    "Gemini": r"gemini",
    "GPT/OpenAI": r"\bgpt|chatgpt|openai",
    "Claude": r"\bclaude\b|anthropic",
    "Qwen-VL": r"qwen",
    "Reducto": r"reducto",
    "pdf.js": r"pdf\.js|pdfjs",
    "PDFium": r"pdfium",
}
POS = re.compile(r"\b(great|excellent|works (really |very )?(well|great|fine)|worked (really |very )?(well|great|fine)|"
                 r"love|loved|impressive|impressed|best|amazing|recommend|good results|solid|happy with|nailed|"
                 r"perfect(ly)?|fantastic|brilliant|reliable|accurate|godsend|blown away|robust|"
                 r"pretty good|very good|really good|surprisingly good|well done|outperform\w*)\b", re.I)
NEG = re.compile(r"\b(terrible|awful|useless|garbage|rubbish|crap|bad|poor(ly)?|struggl\w*|fails?|failed|failing|"
                 r"broken|worse|worst|disappoint\w*|mess|messy|nightmare|hallucinat\w*|slow|expensive|"
                 r"inaccurate|unreliable|wrong|errors?|mangled|gibberish|frustrat\w*|lackluster|meh|pain)\b", re.I)

# Themes that also need an extraction-context word within 150 characters of the match.
CTX = re.compile(r"pdf|ocr|extract|pars(e|ing|er)|\bpages?\b|documents?|scan|tables?|tesseract|textract|"
                 r"llm|gpt|gemini|claude|mistral|\bmodels?\b|\bapi\b|markdown|docling|marker|vlm", re.I)
NEAR = {
    "cost_complaint_ctx": r"expensive|pricey|overpriced|too costly|costs? (too much|a fortune)|prohibitive|"
                          r"(price|pricing|cost) (is|was|are) (too |way |very )?(high|steep|much)",
    "slow_ctx": r"\bslow(er|est|ly|ness)?\b|takes? (forever|ages)|minutes? (per|a|each) page|seconds? (per|a|each) page",
    "licensing_ctx": r"\bagpl\b|\bgpl\b|non-?commercial|cc.?by.?nc|copyleft|"
                     r"(pymupdf|mupdf|itext|marker|surya|nougat|weights?|models?|ghostscript|artifex)\W+(?:\w+\W+){0,6}?licen[cs]|"
                     r"licen[cs]\w*\W+(?:\w+\W+){0,6}?(pymupdf|mupdf|itext|marker|surya|nougat|weights|artifex|commercial)",
    "figures_ctx": r"\bcharts?\b|diagrams?|infographics?|figure (extraction|captions?|references?)|"
                   r"(tables?|charts?|images?|diagrams?),? (and|or|&) figures?|figures?,? (and|or|&) (tables?|charts?|images?|diagrams?)|"
                   r"(charts?|figures?|tables?),? (and|or) graphs?|extract\w* (the )?(embedded )?(images|figures)",
    "scale_ctx": r"throughput|pages? ?(/|per) ?(s|sec|second|minute|min|hour|day)\b|pages?/s\b|"
                 r"millions? of (pdfs|pages|documents|files)|thousands of (pdfs|pages|documents)|at scale|\bfast(er|est)?\b(?! r-cnn)",
    "refusals_ctx": r"safety (filters?|settings?)|refus\w* to (process|ocr|transcribe|read|extract|answer)|"
                    r"prompt injection|instructions? (in|inside|within|hidden in) (the )?(document|text|pdf|scan|page)|"
                    r"rogue instruction|uncensored|recitation|flags? ocr|lecturing mode",
    "install_ctx": r"pytorch|\bcuda\b|dependenc(y|ies)|dependency hell|gigabytes? of|"
                   r"(install|installation|setup|set up)\w*\W+(?:\w+\W+){0,6}?(hours?|pain|hell|nightmare|failed|"
                   r"conflict|broke|gave up|took)",
}
def compile_all(d, flags=re.I):
    return {k: re.compile(v, flags) for k, v in d.items()}

TH = compile_all(THEMES)
NR = compile_all(NEAR)

def near_match(rx, text):
    for m in rx.finditer(text):
        lo, hi = max(0, m.start() - 150), m.end() + 150
        if CTX.search(text[lo:m.start()]) or CTX.search(text[m.end():hi]):
            return m
    return None
TL = {k: re.compile(v, 0 if k == "Unstructured" else re.I) for k, v in TOOLS.items()}

def by_period(sel):
    c = collections.Counter(period(r["date"]) for r in sel)
    return [c.get(p, 0) for p in PERIODS]

def layout_negative():
    t = re.compile(THEMES["reading_order_columns"] + "|" + THEMES["layout_preserve"], re.I)
    return [r for r in rel if t.search(r["text"]) and NEG.search(r["text"])]

def tables_negative():
    t = TH["tables"]
    return [r for r in rel if t.search(r["text"]) and NEG.search(r["text"])]

def main():
    if len(sys.argv) > 2 and sys.argv[1] == "--samples":
        key, n = sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 10
        if key in NR:
            rx = NR[key]
            sel = [r for r in rel if near_match(rx, r["text"])]
        else:
            rx = TH.get(key) or TL.get(key)
            if rx is None:
                sys.exit(f"unknown theme {key!r}; themes: {', '.join(sorted({*NR, *TH, *TL}))}")
            sel = [r for r in rel if rx.search(r["text"])]
        random.seed(1)
        for r in random.sample(sel, min(n, len(sel))):
            m = rx.search(r["text"])
            s = max(0, m.start() - 120)
            print(f"[{r['id']} {r['date']}] ...{r['text'][s:m.end()+160]!r}\n")
        return
    tot = by_period(rel)
    print("base (minus hiring/Typst):", len(base), by_period(base))
    print("REL:", len(rel), dict(zip(PERIODS, tot)))
    print("\nTHEMES  total | 2016-19 2020-22 2023-26 | per 1k REL")
    out = {}
    for k, rx in TH.items():
        sel = [r for r in rel if rx.search(r["text"])]
        bp = by_period(sel)
        rates = [round(1000 * b / t) for b, t in zip(bp, tot)]
        out[k] = (len(sel), bp, rates)
    for k, (n, bp, rates) in sorted(out.items(), key=lambda kv: -kv[1][0]):
        print(f"{k:24s} {n:5d} | {bp[0]:5d} {bp[1]:5d} {bp[2]:5d} | {rates}")
    for k, rx in NR.items():
        sel = [r for r in rel if near_match(rx, r["text"])]
        bp = by_period(sel)
        print(f"{k:24s} {len(sel):5d} | {bp[0]:5d} {bp[1]:5d} {bp[2]:5d} | {[round(1000*b/t) for b,t in zip(bp,tot)]}")
    UNIONS = {
        "layout_reading_order": ["reading_order_columns", "layout_preserve"],
        "bbox_or_citations": ["bbox_coords", "citations_grounding"],
        "garbled_any": ["encoding_fonts", "garbled_text", "spacing_hyphen"],
        "silent_errors": ["hallucination", "dropped_content"],
    }
    for k, parts in UNIONS.items():
        sel = [r for r in rel if any(TH[q].search(r["text"]) for q in parts)]
        bp = by_period(sel)
        print(f"{k:24s} {len(sel):5d} | {bp[0]:5d} {bp[1]:5d} {bp[2]:5d} | {[round(1000*b/t) for b,t in zip(bp,tot)]}")
    ln = layout_negative()
    print(f"{'layout_with_neg_word':24s} {len(ln):5d} | {by_period(ln)}")
    tn = tables_negative()
    print(f"{'tables_with_neg_word':24s} {len(tn):5d} | {by_period(tn)}")
    print("\nTOOLS (REL comments)  total | by period | pos/neg-word counts by period (unreliable: about half mislabeled in a 20-comment check; not reported)")
    for k, rx in TL.items():
        sel = [r for r in rel if rx.search(r["text"])]
        bp = by_period(sel)
        sent = []
        for p in PERIODS:
            ps = [r for r in sel if period(r["date"]) == p]
            pos = sum(1 for r in ps if POS.search(r["text"]) and not NEG.search(r["text"]))
            neg = sum(1 for r in ps if NEG.search(r["text"]) and not POS.search(r["text"]))
            sent.append(f"{pos}+/{neg}-")
        print(f"{k:24s} {len(sel):5d} | {bp} | {sent}")

if __name__ == "__main__":
    main()
