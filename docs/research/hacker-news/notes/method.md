# Method: how the Hacker News data was collected, sorted and counted

*Part of [What PDF users ask for: ten years of Hacker News](../pdf-users-on-hacker-news.md). The scripts are in [`tools/hn-research/`](../../../../tools/hn-research/README.md).*

The research rests on 3,755 stories and 56,907 comments from Hacker News, posted between 1 November 2016 and 6 November 2026. Scripts collected them through the public HN Search API, sorted the comments into five areas, and counted themes with regular expressions. Five agents each read one area in depth and checked the counts by hand. This note describes each step, what it keeps and drops, and how far the numbers can be trusted.

## 1. Stories

[`collect_stories.py`](../../../../tools/hn-research/collect_stories.py) asks the API's `search` endpoint for stories (`tags=story`) whose title (`restrictSearchableAttributes=title`) matches a query, one window at a time (`numericFilters=created_at_i`), 1,000 hits per request.

- **"pdf" by month.** One query per month from November 2016 to November 2026. Monthly windows keep each request under the API's cap of 1,000 hits.
- **31 tool names by year.** Names that the search engine doesn't split into "pdf", or that lack it: MuPDF, PyMuPDF, Poppler, Ghostscript, Acrobat, PDFBox, iText, wkhtmltopdf, WeasyPrint, Docling, pdfplumber, pdftk, qpdf, Okular, SumatraPDF, Zathura, Typst, LlamaParse, olmOCR, PDFium, pdf.js, Mistral OCR, Paged.js, Prince XML, Foxit, Preview.app, PDFKit, ReportLab, Unstructured, Tesseract and OCRmyPDF.

Together they returned 53,144 distinct stories (`data/stories_raw.jsonl`). Most were links to papers: Hacker News marks a link to a PDF file with "[pdf]" in the title, and about nine in ten hits matched only that tag. The script keeps a story only when "pdf" appears outside a bracketed tag such as "[pdf]" or "(PDF, 2 pages)", or when the title names one of the tools. That rule kept 3,755 stories (`data/stories.jsonl`): 3,352 from the "pdf" query and 403 from tool names. The tool rule also drops the search engine's fuzzy matches, such as "Acrobatics" for "acrobat".

## 2. Comments

[`collect_comments.py`](../../../../tools/hn-research/collect_comments.py) gathers comments two ways:

1. **Whole threads.** For each of the 307 kept stories with 20 or more comments, the API's `items` endpoint returns the full comment tree, which the script flattens: 27,185 comments (`data/threads.jsonl`).
2. **Keyword searches across all of Hacker News.** 42 terms, searched in comment text one year at a time with typo tolerance off, find PDF talk in threads whose titles don't mention PDF: 37,771 comments (`data/comment_hits.jsonl`). The terms cover the five areas: "pdf reader", "pdf viewer", "pdf editor", "acrobat", "pdf library", "pdf parsing", "parse pdf", "pdf extraction", "extract text pdf", "pdf tables", "pdf generation", "generate pdf", "html to pdf", "pdf forms", "fill pdf", "sign pdf", "redact pdf", "annotate pdf", "pdf ocr", "pdf markdown", and 22 tool names. `data/comment_counts.json` records the API's hit count for each term and year; none exceeded 1,000, so no search was cut short.

Comment HTML becomes plain text. The latest comment collected was posted at 01:15 UTC on 6 November 2026.

## 3. Areas

[`prep.py`](../../../../tools/hn-research/prep.py) merges the two sources and keeps one copy of each comment that has at least 40 characters: 56,907 comments. It then keeps a comment for the areas only if it sits in a kept story or names PDF or a PDF tool. One regular expression per area sorts it, so a comment can fall in several areas:

| Area | Comments | What the pattern looks for |
| --- | ---: | --- |
| Readers | 7,277 | reader and viewer names, Preview, browser viewers, e-readers and tablets, dark mode, reflow, zoom |
| Editors | 9,069 | editing, annotating, form filling, XFA, signing, redaction, merging, splitting, compressing, page tools |
| Extraction | 13,682 | parsing, extracting, tables, OCR, layout, reading order, Markdown, RAG and LLMs, extraction tools |
| Generation | 10,987 | generating, HTML to PDF and its tools, LaTeX, Typst, templates, invoices, typesetting |
| Format and security | 10,546 | the spec, PDF/A and PDF/UA, accessibility, security, exploits, JavaScript, licences, metadata, forensics, fonts, archiving |

Each area file (`areas/<area>.jsonl`) lists its comments from the busiest threads down, with text cut to 1,500 characters. The script also counts mentions of 47 tools per year (`stats/tool_mentions_by_year.json`). [`story_stats.py`](../../../../tools/hn-research/story_stats.py) counts stories per year, per area (by title pattern) and as Show HN launches (`stats/story_volume.json`).

The area patterns are deliberately broad, so they let in noise: "generate" catches talk about LLMs, "table" catches "table of contents". The agents narrowed each area again before counting (§5).

## 4. Periods

Counts are reported by year or by three periods: 2016–19, 2020–22 and 2023–26. The first period starts in November 2016 and the last ends on 6 November 2026, so 2016 and 2026 are partial years. Comment volume more than doubled between the first and last periods, so the notes give rates per 1,000 comments of a period wherever a trend matters.

## 5. Reading and counting

Five agents each took one area. Each read between 900 and 1,400 comments in full or in long excerpts: the largest threads of the decade first, then seeded random samples from every year. From that reading, each agent wrote one regular expression per theme ("tables come out broken", "sign without printing"), usually with a second pattern that must match nearby and a rule that the comment names PDF or a PDF tool.

Each theme then went through spot checks. The agent read 6 to 20 random matches and recorded how many were on topic. Themes that scored low were tightened or dropped. The generation and format and security notes multiply matches by the on-topic share to estimate on-topic comments; the other notes give raw matches and the share beside them. Each note's §7 lists its spot-check scores, which run from about a third to all of the matches on topic.

The agents built slightly different bases, because each tuned its own filter: the editors note counts over 56,722 comments, the format and security note over 54,992, the readers note over 7,277 area comments with 58,841 for its extra counts, the extraction note over 6,028 extraction comments and the generation note over 9,523 generation comments. Each note states its base.

Sentiment came from reading. A word-list score rated Acrobat neutral, against what the readers agent read, and mislabelled about half of a 20-comment check in extraction. So the readers and editors notes rest their sentiment on hand labels for seeded samples of 12 to 20 comments per tool and period, and the editors and generation notes mark any word-list score as a weak signal. The readers note's labels are in [`counts/readers/sentiment_labels.json`](../../../../tools/hn-research/counts/readers/sentiment_labels.json).

## 6. Quotes

Every quote cites its comment by ID and links to it. [`verify_quotes.py`](../../../../tools/hn-research/verify_quotes.py) checks that each quote line appears word for word in its comment (whitespace aside, with "…" allowed between pieces in order), runs 40 words or fewer and carries the comment's date. It passed all 388 quotes in the five notes before they were committed.

## 7. Limits

- **Who comments.** Hacker News leans towards developers, founders and people on Mac and Linux. Office workers, lawyers, government staff, publishers and blind readers appear mostly second-hand. Command-line tools, vim keys and LaTeX probably count for more here than among all PDF users; forms, signing and phone reading for less.
- **Comments, not people.** A long thread lifts a period: the two 2020 "What's so hard about PDF text extraction?" threads hold 15% of 2020–22's extraction comments.
- **Patterns.** Regular expressions miss paraphrases and catch other meanings. Read counts as relative sizes and trends, not totals.
- **Events.** The pandemic lifted signing in 2020–22, and the Epstein releases lifted redaction in December 2025.
- **Titles.** The story set comes from title searches, so stories that don't name PDF or a listed tool are missing, along with their points. Their comments enter only through the keyword searches.
- **Cut text.** Area files cut each comment at 1,500 characters, so later mentions in long comments are missed. The editors note counted over the full text.
- **Acrobat.** The first `prep.py` matched "acrobat" without word boundaries, so 166 of its 523 Acrobat comments in 2016–19 were about acrobatics. The committed version uses `\bacrobat\b`; the notes' Acrobat figures come from the agents' own patterns, which always used word boundaries.
- **A moving source.** The API's points and comment counts change, and deleted comments drop out, so a rerun gives slightly different numbers.

## 8. Data handling

The collected data stays out of git: it is about 120 MB of other people's writing, and the API serves it again on request. The notes quote short passages, at most 40 words each, with a link to the original comment. The notes cite comments by ID and link, never by username.
