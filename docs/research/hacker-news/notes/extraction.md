# PDF parsing and extraction on Hacker News, Nov 2016 – Nov 2026

*Part of [What PDF users ask for: ten years of Hacker News](../pdf-users-on-hacker-news.md). How the data was collected: [method](method.md). The work it leads to: the [trust and proof plan](../../../plans/trust-and-proof.md).*

This report covers ten years of Hacker News comments about getting text, tables, layout and structure out of PDFs, including OCR and conversion for LLMs. The base is the 13,682 comments classified as parsing-related. I removed 849 from hiring and Typst threads, then kept the 6,028 that talk about extraction: 795 from 2016–19, 1,424 from 2020–22 and 3,809 from 2023–26. Every count below is a number of comments in that subset whose text matches a stated regex, produced by `tools/hn-research/counts/extraction/extraction_counts.py` (output in `tools/hn-research/counts/extraction/output.txt`). Counts read "total (2016–19 / 2020–22 / 2023–26)". Because volume grew fivefold, I give rates per 1,000 subset comments where the trend matters.

The main finding: what people want from extraction barely moved in ten years (tables, scanned pages, reading order), but the tools they reach for changed after 2023, when vision-language models took over the conversation and brought new complaints about made-up text, price and output that changes between runs.

## 1. What users want

Ranked by number of comments. "Precision" is how many of a random sample of matches were on topic when I read them.

**1. Tables as data.** People want tables out as rows and cells (CSV, a DataFrame, Markdown), merged cells included. 810 comments (96 / 164 / 550), regex `tables` (`\btables?\b` minus "table of contents", "off the table", plus "tabular"); precision 8 of 12. The rate held at 115–144 per 1,000 in every period, which makes tables the most stable want of the decade.
- "My go-to solution has been 'pdftotext -layout' with a bit of hackery before giving it to pandas.read_fwf. That usually gets me 80% of the way there 80% of the time." ([18200905](https://news.ycombinator.com/item?id=18200905), 2018-10-12)
- "Tables are still the big unsolved problem for me." ([39879481](https://news.ycombinator.com/item?id=39879481), 2024-03-30)
- "For my use case, overall Marker seems to work pretty well - but it has issues with tables. Merged cells, misplaced headers, and so forth." ([41107213](https://news.ycombinator.com/item?id=41107213), 2024-07-30)

**2. Output an LLM can use: Markdown, JSON and chunks.** Since 2023 the goal of extraction is often a prompt or a RAG index, not a text file. 791 comments (22 / 54 / 715), regex `llm_output` (Markdown or JSON as an output, RAG, chunks, embeddings, vector stores, "for an LLM"); precision 6 of 8. The rate went from 28 to 38 to 188 per 1,000. Chunking alone: 172 (6 / 10 / 156).
- "I chose markdown because I wanted to preserve equations (fenced by $/$$), tables, bold/italic information, and headers." ([38487125](https://news.ycombinator.com/item?id=38487125), 2023-12-01, the author of Marker)
- "Fixed size chunks is holding back a bunch of RAG projects on my backlog." ([42955403](https://news.ycombinator.com/item?id=42955403), 2025-02-05)
- "So I can ask Gemini to return chunks of variable size, where each chunk is a one complete idea or concept, without arbitrarily chopping a logical semantic segment into multiple chunks." ([42954142](https://news.ycombinator.com/item?id=42954142), 2025-02-05)

**3. Text from scanned pages.** Many PDFs hold only page images, and users want accurate OCR for them, handwriting included. 617 comments name scans (101 / 172 / 344), regex `scanned_docs` (`scann?ed|\bscans?\b`); precision 11 of 12. Any OCR or scan term appears in 2,648 (44% of the subset; 48%, 48% and 42% by period). Handwriting: 139 (11 / 25 / 103).
- "Tesseract was amazingly powerful and accurate, but it seemed to struggle if the page was warped or tilted even a little." ([21844608](https://news.ycombinator.com/item?id=21844608), 2019-12-20)
- "I bet 90% of the problem space is legacy PDFs. My company has thousands of these. Some are crappy scans. Some have Adobe's OCR embedded, but most have none at all." ([43975217](https://news.ycombinator.com/item?id=43975217), 2025-05-13)
- "My experience is that “text is actually images or paths” is closer to the 40% case than the 1% case." ([44784425](https://news.ycombinator.com/item?id=44784425), 2025-08-04)

**4. Low cost per page.** Users compare price per 1,000 pages and pages per dollar. 570 comments (50 / 111 / 409), regex `cost` (dollar amounts, "per page", cost, cheap, expensive, pricing); precision 7 of 12. The rate rose from 63 to 107 per 1,000 as paid APIs and LLMs entered.
- "We parse millions of PDFs using Apache Tika and process about 30,000 per dollar of compute cost." ([42955385](https://news.ycombinator.com/item?id=42955385), 2025-02-05)
- "I've found Google's Flash to cut my OCR costs by about 95+% compared to traditional commercial offerings that support structured data extraction" ([43974639](https://news.ycombinator.com/item?id=43974639), 2025-05-13)
- "Most use cases dont need that kind of accuracy, just doesnt justify the 3-4usd range." ([49290517](https://news.ycombinator.com/item?id=49290517), 2026-08-13)

**5. Local, private processing.** Users do not want to upload contracts, client files or medical notes to a cloud service. 279 comments (30 / 41 / 208), regex `local_privacy` (privacy, on-prem, self-hosted, offline, run locally, local model, confidential, sensitive data, uploading to a cloud); precision 5 of 8. The concern was already there in 2017, before LLMs.
- "Yeah apparently it uses https://ocr.space/, deal-breaker for me." ([14594467](https://news.ycombinator.com/item?id=14594467), 2017-06-20)
- "I'm working with client data that I don't want to bang through Google." ([22475648](https://news.ycombinator.com/item?id=22475648), 2020-03-03)
- "$1/1000 pages is significantly more expensive than my current local Docling based setup that already does a great job of processing PDF's for my needs." ([43283593](https://news.ycombinator.com/item?id=43283593), 2025-03-06)

**6. Throughput at scale.** Search engines, archives and document pipelines process millions of pages and need fast extraction. 265 comments (24 / 42 / 199), regex `scale_ctx` (throughput, pages per second, millions of PDFs, at scale, fast, next to an extraction word); precision 5 of 8.
- "The article is in the context of an internet search engine, the corpus to be converted is of order 1 TB." ([43974337](https://news.ycombinator.com/item?id=43974337), 2025-05-13)
- "And is an order of magnitude more efficient than generating an image in 300-600 DPI and doing OCR or Visual LLM." ([44783418](https://news.ycombinator.com/item?id=44783418), 2025-08-04)

**7. Correct reading order and layout.** Text from multi-column pages must come out in the order a person reads it, with the layout kept where it matters. 226 comments (32 / 55 / 139), regex `layout_reading_order` (reading order, multi-column and two-column, `-layout`, layout analysis or detection); precision 6 of 7. The rate stayed near 40 per 1,000 all decade.
- "That has a --layout option that works really well sometimes and really terrible other times." ([13729654](https://news.ycombinator.com/item?id=13729654), 2017-02-25, on pdftotext)
- "outputs a single stream of text with the correct reading order (even for multi column PDF)" ([43212718](https://news.ycombinator.com/item?id=43212718), 2025-02-28, listed as a strength of olmOCR)
- "I have investigated many tools, but two-column layouts and footers etc often still mess up the content." ([43979397](https://news.ycombinator.com/item?id=43979397), 2025-05-14)

**8. Named fields as JSON.** Invoices, statements and forms need specific values pulled into a schema, not prose. 214 comments (12 / 25 / 177), regex `structured_extraction` (JSON schema, structured output or data, key-value, form fields, line items); precision 6 of 8. The common use cases are invoices and receipts (169), bank and financial statements (130) and forms (309, a looser regex).
- "2. structured data extraction (pull out targeted values)" ([43975807](https://news.ycombinator.com/item?id=43975807), 2025-05-13)
- "You provide a PDF and a JSON schema defining what to extract, and it returns the extracted values, the citations and their precise locations in the document." ([43823338](https://news.ycombinator.com/item?id=43823338), 2025-04-28)

**9. Document structure.** Users want headings, sections, lists and footnotes, not one flat stream of text. 189 comments (21 / 61 / 107), regex `structure_headings` (headings, section headers, hierarchy, document structure, outline, table of contents); precision 5 of 8. Tagged PDF and structure trees come up rarely: 54 comments (7 / 28 / 19), regex `tagged_pdf`.
- "Extracting plain text isn’t that much of a problem, relatively speaking. It’s interpreting more complex elements like nested lists, tables, side bars, footnotes/endnotes, cross-references, images and diagrams where things get challenging." ([42955074](https://news.ycombinator.com/item?id=42955074), 2025-02-05)
- "OCR is fantastic these days but maintaining a global structure to the document is much trickier." ([43976203](https://news.ycombinator.com/item?id=43976203), 2025-05-13)

**10. A fair way to judge quality.** Each new model claims a top score, and users ask for public benchmarks they can rerun. 163 comments (2 / 5 / 156), regex `benchmarks` (benchmark, ground truth, leaderboard, eval, edit distance, test set); precision 7 of 8. The rate went from 3 to 41 per 1,000.
- "\"World's best OCR model\" - that is quite a statement. Are there any well-known benchmarks for OCR software?" ([43283348](https://news.ycombinator.com/item?id=43283348), 2025-03-06)
- "measuring accuracy in table parsing is surprisingly challenging." ([42953775](https://news.ycombinator.com/item?id=42953775), 2025-02-05, the author of the Gemini post)

**11. Math as LaTeX.** Scientific papers and textbooks need formulas as LaTeX, not mangled symbols. 159 comments (20 / 26 / 113), regex `math_formulas` (equations, formulas, Mathpix, Nougat, math next to LaTeX/OCR/PDF); precision 6 of 8.
- "Tesseract is awful for scientific papers. The structure is mangled, formulas are completely rubbish, tables are nearly useless, etc." ([43219730](https://news.ycombinator.com/item?id=43219730), 2025-03-01)
- "perfect markdown with latex for the math every time." ([46332217](https://news.ycombinator.com/item?id=46332217), 2025-12-19, on Gemini 3 Pro)

**12. Bounding boxes and citations.** Teams want every extracted value tied to a page region, so a reviewer can check it and an answer can cite it. 155 comments (22 / 17 / 116), regex `bbox_or_citations` (bounding box, bbox, coordinates, grounding, citations to a page or location); precision 7 of 7 for the box terms.
- "we’ve found bounding box based attribution to be a non-negotiable for many of our current customers." ([42954289](https://news.ycombinator.com/item?id=42954289), 2025-02-05, CTO of Reducto)
- "Gemini 2.0 still materially struggles with bounding boxes on digital content." ([42953466](https://news.ycombinator.com/item?id=42953466), 2025-02-05)
- "bounding boxes are must have for proper grounding for building answers by LLMs." ([49290517](https://news.ycombinator.com/item?id=49290517), 2026-08-13)

**13. Figures and charts.** Users want images extracted and charts or diagrams turned into text an LLM can use. 132 comments (12 / 21 / 99), regex `figures_ctx` (charts, diagrams, infographics, "tables and figures", extracting images); precision 5 of 8.
- "Text from diagrams can be useful in LLMs." ([43216447](https://news.ycombinator.com/item?id=43216447), 2025-03-01)
- "It outperforms the competition significantly AND can extract embedded images from the text." ([43283146](https://news.ycombinator.com/item?id=43283146), 2025-03-06, on Mistral OCR)

## 2. What they complain about most

**1. Tables come out broken.** Rows merge, headers shift, empty cells vanish, and LLMs cut long tables short. 246 comments mention tables together with a negative word (16 / 45 / 185); this is a rough co-occurrence count (`tables` plus a word such as fail, wrong, mess, mangled, struggle).
- "any kind of data table gets absolutely mangled/unusable for cut/paste purposes after creation." ([24111127](https://news.ycombinator.com/item?id=24111127), 2020-08-10)
- "extracting tables in pdfs using any of the available openAI models has been a waste of prompting time here too." ([42178239](https://news.ycombinator.com/item?id=42178239), 2024-11-18)
- "they'd often insert \"...rest of table follows\" and similar terminations, regardless of different kinds of prompting." ([42953492](https://news.ycombinator.com/item?id=42953492), 2025-02-05)

**2. Models make up or drop content.** LLM and VLM extractors write plausible text that is not on the page, or leave out parts without saying so. 205 comments (5 / 11 / 189), union of `hallucination` (hallucinate, made up, fabricate, plausible; 172 comments, 1 / 5 / 166, precision 8 of 8) and `dropped_content` (skipped, omitted, missing or truncated text, rows or pages; 39, a lower bound). The rate went from 6 to 50 per 1,000. Users contrast this with classic OCR errors, which stay small and look wrong.
- "When a LLM hallucinates, there is no limit to how much text can be made up." ([42956082](https://news.ycombinator.com/item?id=42956082), 2025-02-05)
- "The biggest thing I noticed is how it skips anything it classifies as an image/figure." ([43285983](https://news.ycombinator.com/item?id=43285983), 2025-03-06, on Mistral OCR)
- "Fields that aren't in the document are being made up 11 to 40% of the time depending on the API." ([49536697](https://news.ycombinator.com/item?id=49536697), 2026-09-02)

**3. Garbled text from fonts and encodings.** Missing ToUnicode maps, ligatures, odd spaces inside words and copy-paste that yields junk. 160 comments (29 / 46 / 85), union of `encoding_fonts`, `garbled_text` and `spacing_hyphen` (ToUnicode, CMap, glyphs, gibberish, ligatures, CID or Type 3 fonts, missing or extra spaces, copy-paste next to "garbage"); precision 7 of 8. The rate fell from 36 to 22 per 1,000, partly because rendering pages to images sidesteps the problem.
- "Copy-paste from PDF very often results in complete rubbish, even when it is not deliberately prevented" ([13731292](https://news.ycombinator.com/item?id=13731292), 2017-02-25)
- "Do you have any tricks for dealing with missing unicode character mapping tables for embedded fonts?" ([22476806](https://news.ycombinator.com/item?id=22476806), 2020-03-03)
- "Most of the titles have incorrectly split words" ([43974913](https://news.ycombinator.com/item?id=43974913), 2025-05-13, on a Markdown converter's output)

**4. Slow tools.** Layout models and VLMs take seconds or minutes per page. 112 comments (14 / 22 / 76), regex `slow_ctx` (slow, takes forever, minutes or seconds per page, next to an extraction word); precision 4 of 7.
- "OCR developers from decades past must be slapping their foreheads now that it seems users will wait a whole minute per page and be happy." ([46331397](https://news.ycombinator.com/item?id=46331397), 2025-12-19)
- "Docling and Marker work pretty well but are very slow." ([47569939](https://news.ycombinator.com/item?id=47569939), 2026-03-30)
- "they run at 1-5 pages/s because of the VLMs under the hood." ([47753206](https://news.ycombinator.com/item?id=47753206), 2026-04-13)

**5. Price.** Commercial OCR, cloud document APIs and OCR models are called too expensive for bulk work. 98 comments (10 / 19 / 69), regex `cost_complaint_ctx` (expensive, pricey, prohibitive, next to an extraction word); precision 5 of 7.
- "While Abbyy is likely the best, it's also incredibly expensive." ([22481705](https://news.ycombinator.com/item?id=22481705), 2020-03-04)
- "the resulting costs were way higher than expected" ([41107230](https://news.ycombinator.com/item?id=41107230), 2024-07-30, on Azure Document Intelligence)
- "1000 Pages / 3.5€ this is expensive as hell." ([49289573](https://news.ycombinator.com/item?id=49289573), 2026-08-13, on Mistral OCR 4.1)

**6. Headers, footers and page numbers in the text.** Running heads and page numbers break sentences and pollute chunks. 95 comments (4 / 22 / 69), regex `headers_footers`; precision 4 of 7. Footnotes come up in another 27.
- "if a PDF has a footer applied Acrobat sees that as renderable text and blows off the rest of the rest of the page." ([22475648](https://news.ycombinator.com/item?id=22475648), 2020-03-03)
- "Do you consider the header/footer text to be important, or just noise" ([24471568](https://news.ycombinator.com/item?id=24471568), 2020-09-14)
- "Layout (2-columns) along with header/footers can cause some headaches" ([46332351](https://news.ycombinator.com/item?id=46332351), 2025-12-19)

**7. Files that tools cannot open.** Many parsers fail on real-world files with broken offsets or odd producers. 85 comments (10 / 33 / 42), regex `malformed_files` (malformed, corrupt, broken PDF or xref, fails to open or parse, crash, repair).
- "Only three of them can process all 2500 files i tried" ([44785629](https://news.ycombinator.com/item?id=44785629), 2025-08-04, after testing about 25 engines on machine manuals)
- "mupdf is by far the slowest and least able to open valid pdfs when it came to text extraction." ([46440768](https://news.ycombinator.com/item?id=46440768), 2025-12-31)

**8. Columns read in the wrong order.** Lines from two columns interleave. 80 comments pair reading-order or layout terms with a negative word (10 / 13 / 57), a rough co-occurrence count.
- "it seems to struggle with multi-column layouts (lines get intermingled)." ([38485411](https://news.ycombinator.com/item?id=38485411), 2023-12-01, on Marker)
- "The lines from the two columns are mixed and a lot of spaces are randomly placed" ([46206703](https://news.ycombinator.com/item?id=46206703), 2025-12-09)

**9. PDF is the wrong container for data.** Users resent having to extract data that was generated by a computer in the first place. 60 comments (9 / 22 / 29), regex `pdf_wrong_format` (machine-readable, "should be published as CSV/HTML", "PDF hell", "characters at coordinates"); precision 5 of 7. A related wish, embedding the source data in the PDF (ZUGFeRD, attached CSV), appears in 40 comments.
- "we decided to standardize document distribution on the one that makes it as hard to extract data as if it were on physical paper." ([13730647](https://news.ycombinator.com/item?id=13730647), 2017-02-25)
- "Consuming banking data as PDFs is a nightmare." ([24112759](https://news.ycombinator.com/item?id=24112759), 2020-08-10)
- "how nuts it is that we have spun up whole industries around extracting text that was typically printed from a computer, back into a computer." ([42956650](https://news.ycombinator.com/item?id=42956650), 2025-02-05)

**10. Heavy installs.** Model-based tools pull in PyTorch, CUDA and gigabytes of weights. 58 comments (4 / 7 / 47), regex `install_ctx`; precision 3 of 7, so treat it as small.
- "It took an hour and a half to install 12 gigabytes of pytorch dependencies that can't even run on my device" ([46331521](https://news.ycombinator.com/item?id=46331521), 2025-12-19)

**11. Results change from run to run.** The same page gives different output on a second call. 55 comments (4 / 6 / 45), regex `determinism`; precision 3 of 7, so treat it as small but new.
- "even with the same model, running it two or three times in a row gives different results." ([42956937](https://news.ycombinator.com/item?id=42956937), 2025-02-05, on Gemini)
- "their biggest flaw is hallucinations and overall the nondeterministic nature." ([43213626](https://news.ycombinator.com/item?id=43213626), 2025-03-01)

**12. One parser per layout.** Rule-based pipelines need a template per bank or supplier and break when the layout changes. 37 comments (3 / 7 / 27), regex `brittle_templates`; the regex misses many paraphrases, and the complaint reads as larger in the 2020 thread on text extraction than the count shows.
- "Always end up with a parser per supplier." ([22473986](https://news.ycombinator.com/item?id=22473986), 2020-03-03)
- "You end up creating a new rule every time you miss the data extraction." ([22475088](https://news.ycombinator.com/item?id=22475088), 2020-03-03)
- "your workflows will break if you are not building it for a specific pdf template." ([41108191](https://news.ycombinator.com/item?id=41108191), 2024-07-30)

**13. Licences that block use.** AGPL libraries and non-commercial model weights stop people from shipping. 28 comments (4 / 5 / 19), regex `licensing_ctx` (AGPL, GPL, non-commercial, CC-BY-NC, or a licence word next to PyMuPDF, MuPDF, iText, Marker, Nougat or model weights). The count is small, but the warnings appear in launch threads, where they shape adoption.
- "Everything was alright until I discovered their AGPL license." ([22201037](https://news.ycombinator.com/item?id=22201037), 2020-01-31, on iText)
- "Due to the licensing of the underlying models like layoutlmv3 and nougat, this is only suitable for noncommercial usage." ([38487108](https://news.ycombinator.com/item?id=38487108), 2023-12-01, quoting Marker's README)
- "careful if you plan on using this. it leverages pymupdf which is AGPL." ([44937078](https://news.ycombinator.com/item?id=44937078), 2025-08-18)

**14. Refusals, censorship and prompt injection.** An LLM may refuse a document, skip text it finds objectionable, or follow instructions written in the page. 20 comments (0 / 0 / 20), regex `refusals_ctx`. Small, but it only exists for model-based extraction.
- "I want to be able to run OCR against things like police incident reports without worrying that a safety filter in the LLM will refuse to process the document" ([41203888](https://news.ycombinator.com/item?id=41203888), 2024-08-09)
- "they can't be trusted not to engage in unexpected censorship, typically skipping parts they find objectionable without reliably telling you that that's what they did." ([43074159](https://news.ycombinator.com/item?id=43074159), 2025-02-17)
- "I'm still paranoid that some rogue instruction in the scanned text (accidental or deliberate) might result in an inaccurate result." ([44781078](https://news.ycombinator.com/item?id=44781078), 2025-08-04)

## 3. What excites them

Attention moved from table tools and JavaScript OCR (2017–19) to essays on why extraction is hard (2020), then to model-based converters (2023–24) and finally to OCR models and LLM ingestion (2025–26). Points and comment counts are from `data/stories.jsonl`, except Textract and DeepSeek-OCR, which are only in `data/stories_raw.jsonl`.

| Date | Story | Points | Comments | What drew attention |
|---|---|---:|---:|---|
| 2017-02-25 | [Show HN: PDFLayoutTextStripper](https://news.ycombinator.com/item?id=13729301) | 283 | 92 | Text with the layout kept, on PDFBox; long comparison with `pdftotext -layout` and Tabula |
| 2017-06-20 | [Show HN: Copyfish](https://news.ycombinator.com/item?id=14593906) | 292 | 89 | OCR from a browser extension; the thread turned on documents being sent to a server |
| 2018-10-12 | [A Python Library to extract tabular data from PDFs (Camelot)](https://news.ycombinator.com/item?id=18199708) | 501 | 99 | Lattice and stream table detection; "This library works perfectly and could've saved me a lot of time!" ([18200833](https://news.ycombinator.com/item?id=18200833)) |
| 2018-11-28 | [Amazon Textract](https://news.ycombinator.com/item?id=18554122) | 229 | 72 | Cloud forms and tables extraction |
| 2019-12-20 | [Tesseract.js](https://news.ycombinator.com/item?id=21843713) | 364 | 77 | OCR in pure JavaScript and WASM |
| 2020-03-03 | [What's so hard about PDF text extraction?](https://news.ycombinator.com/item?id=22473263) | 733 | 342 | The decade's reference thread on rule-based extraction; reposted 2020-09-13 ([406 points](https://news.ycombinator.com/item?id=24460142)) |
| 2022-07-08 | [OCRmyPDF](https://news.ycombinator.com/item?id=32028752) | 240 | 57 | Adding a text layer to scans |
| 2023-03-23 | [The drama in trying to convert election PDFs to Spreadsheets](https://news.ycombinator.com/item?id=35272227) | 716 | 141 | Public data locked in PDFs |
| 2023-12-01 | [Marker: Convert PDF to Markdown](https://news.ycombinator.com/item?id=38482007) | 683 | 95 | The first popular PDF-to-Markdown tool on layout models; "Let's build a pipeline: all the pdfs -> markdown them all -> archive.org them all" ([38485143](https://news.ycombinator.com/item?id=38485143)) |
| 2024-01-24 | [Show HN: Open-source Rule-based PDF parser for RAG](https://news.ycombinator.com/item?id=39113972) | 293 | 32 | Structure-aware chunks without a model |
| 2024-02-20 | [LlamaCloud and LlamaParse](https://news.ycombinator.com/item?id=39443972) | 195 | 82 | Hosted parsing for RAG; open-source users wary of a proprietary turn |
| 2024-03-30 | [Running OCR against PDFs and images directly in the browser](https://news.ycombinator.com/item?id=39877391) | 301 | 58 | Tesseract.js in the browser, no upload |
| 2024-08-09 | [Show HN: LLM-aided OCR](https://news.ycombinator.com/item?id=41203306) | 479 | 172 | Tesseract output corrected by an LLM; hallucination and safety-filter worries |
| 2025-02-05 | [Ingesting PDFs and why Gemini 2.0 changes everything](https://news.ycombinator.com/item?id=42952605) | 1,303 | 447 | Cheap VLM ingestion with chunking; "If Gemini can do semantic chunking at the same time as extraction, all for so cheap and with nearly perfect accuracy, and without brittle prompting incantation magic, this is huge." ([42953745](https://news.ycombinator.com/item?id=42953745)) |
| 2025-02-25 | [OlmOCR](https://news.ycombinator.com/item?id=43174298) | 313 | 40 | Open-weight OCR model with reading order; "it's the only AI Vision toolkit so far that actually recognizes Latin and medieval scripts." ([43215123](https://news.ycombinator.com/item?id=43215123)) |
| 2025-03-06 | [Mistral OCR](https://news.ycombinator.com/item?id=43282905) | 1,756 | 417 | The most upvoted extraction story of the decade; $1 per 1,000 pages; an independent benchmark disputed the claimed score |
| 2025-08-03 | [So you want to parse a PDF?](https://news.ycombinator.com/item?id=44780353) | 408 | 230 | Parser internals; a long fight over parsing versus rasterising |
| 2025-10-21 | [Karpathy on DeepSeek-OCR paper](https://news.ycombinator.com/item?id=45658928) | 410 | 173 | Pixels as LLM input (this thread's comments are not in the data) |
| 2025-12-18 | [Mistral OCR 3](https://news.ycombinator.com/item?id=46313390) | 694 | 130 | Commenters asked for comparisons with open models: PaddleOCR-VL, olmOCR-2, Chandra, dots.ocr, MinerU |
| 2025-12-30 | [Zpdf: PDF text extraction in Zig](https://news.ycombinator.com/item?id=46437288) | 217 | 87 | A speed claim (41,000 pages a second) met with doubt about correctness and vibe-coded code |
| 2026-06-23 | [Mistral OCR 4](https://news.ycombinator.com/item?id=48645152) | 501 | 136 | Bounding boxes; price ($4 per 1,000 pages) questioned |
| 2026-08-13 | [Mistral OCR 4.1](https://news.ycombinator.com/item?id=49288889) | 411 | 166 | Accuracy versus price versus speed |

Open OCR models (olmOCR, Qwen-VL, Surya, PaddleOCR-VL, dots.ocr, Chandra, MinerU, DeepSeek-OCR, SmolDocling and others) appear in 15 subset comments in 2024, 54 in 2025 and 45 in 2026 up to November. Hybrid pipelines draw interest too: OCR or a native parser first, then a model only where needed.
- "Part of the fix is to allow a 'manager' VLM to dispatch to OCR on dense, simple documents, while running charts, graphs etc. through the more expensive VLM pipeline." ([41206121](https://news.ycombinator.com/item?id=41206121), 2024-08-09)
- "Imagine you have a cheap and 99% accurate ocr. The other 1% you can detect and apply more powerful (more accurate but slower and more expensive) ocr method." ([49290831](https://news.ycombinator.com/item?id=49290831), 2026-08-13)

## 4. Tools and sentiment over time

The table gives the number of subset comments naming each tool and the share of the period's subset. Tools are matched by name; Gemini, GPT and Claude counts include some comments where the model is used for other work. The 2016–22 Marker counts are mostly the common word "marker".

| Tool | 2016–19 | 2020–22 | 2023–26 |
|---|---|---|---|
| Tesseract | 81 (10.2%) | 136 (9.6%) | 211 (5.5%) |
| Poppler / pdftotext | 26 (3.3%) | 63 (4.4%) | 68 (1.8%) |
| Tabula | 43 (5.4%) | 70 (4.9%) | 75 (2.0%) |
| Camelot | 12 (1.5%) | 19 (1.3%) | 8 (0.2%) |
| PDFBox | 15 (1.9%) | 9 (0.6%) | 14 (0.4%) |
| ABBYY FineReader | 17 (2.1%) | 30 (2.1%) | 15 (0.4%) |
| OCRmyPDF | 2 (0.3%) | 32 (2.2%) | 28 (0.7%) |
| PyMuPDF / MuPDF | 3 (0.4%) | 7 (0.5%) | 70 (1.8%) |
| AWS Textract | 3 (0.4%) | 17 (1.2%) | 68 (1.8%) |
| Azure Document Intelligence | 1 (0.1%) | 1 (0.1%) | 29 (0.8%) |
| Docling | 0 | 0 | 110 (2.9%) |
| Marker | (6) | (4) | 61 (1.6%) |
| Mistral OCR | 0 | 0 | 59 (1.5%) |
| LlamaParse | 0 | 0 | 41 (1.1%) |
| GPT / OpenAI | 0 | 4 (0.3%) | 373 (9.8%) |
| Gemini | 0 | 2 (0.1%) | 287 (7.5%) |
| Claude | 0 | 0 | 202 (5.3%) |

Taken together, comments naming a classic open-source library (pdftotext, PDFBox, pdfminer, pdfplumber, PyMuPDF, Tika, Tabula, Camelot, iText, pdf.js, PDFium) held at 13.6% of the subset in 2016–19 and 2020–22, then fell to 7.6% in 2023–26. Comments naming any LLM or VLM went from 0% to 0.5% to 40.4%. By year, the LLM/VLM share was 1.2% in 2022, 21% in 2023, 38% in 2024, 47% in 2025 and 44% in 2026. I tried a positive/negative word count per tool; on a 20-comment check it mislabeled about half, so the sentiment notes below rest on reading, with quotes.

**Tesseract: from default to baseline.** In 2016–22 people argued over whether it was good enough; by 2024 some called it obsolete for hard pages, though it remains the cheap baseline that VLM pricing is measured against.
- "I know Tesseract is OS but when I tried it, it was nearly useless." ([14742871](https://news.ycombinator.com/item?id=14742871), 2017-07-11)
- "I’m not using tesseract for anything anymore, for my tasks it’s obsolete." ([41203750](https://news.ycombinator.com/item?id=41203750), 2024-08-09)
- "Tbf even with tesseract you already get shit ton of accuracy" ([49290503](https://news.ycombinator.com/item?id=49290503), 2026-08-13)

**pdftotext, pdfminer and PDFBox: trusted, fast and plain.** Opinion stayed steady and positive for speed and plain text. Their share of attention fell, because users now ask for Markdown and structure, which these tools do not give.
- "I've had remarkably good results in general (for reading) using the Poppler library's \"pdftotext\" utility." ([22477347](https://news.ycombinator.com/item?id=22477347), 2020-03-03)
- "Poppler's pdftotext utility and pdfminer.six were generally the fastest." ([43977036](https://news.ycombinator.com/item?id=43977036), 2025-05-13)

**Tabula, Camelot and pdfplumber: respected, with manual work.** In 2017–21 they were the answer to every table question. Users praised them and still described hand-tuning; after 2023 pdfplumber is the one still recommended for large jobs.
- "In my experience it's the most reliable open source software for extracting tables from PDFs." ([13731306](https://news.ycombinator.com/item?id=13731306), 2017-02-25, on Tabula)
- "Still a decent amount of manual work afterwards though but it's probably unreasonable to expect otherwise." ([22474159](https://news.ycombinator.com/item?id=22474159), 2020-03-03, on Camelot)
- "I did this at a very large scale recently and ended up just using pdfplumber." ([43220837](https://news.ycombinator.com/item?id=43220837), 2025-03-01)

**PyMuPDF and MuPDF: popular after 2023, with an AGPL asterisk.** PyMuPDF4LLM put it in RAG stacks. Praise for capability sits next to licence warnings and one strong complaint about speed and file coverage.
- "pymupdf is incredible." ([44780975](https://news.ycombinator.com/item?id=44780975), 2025-08-04)
- "careful if you plan on using this. it leverages pymupdf which is AGPL." ([44937078](https://news.ycombinator.com/item?id=44937078), 2025-08-18)

**ABBYY and the cloud document APIs: from best-in-class to replaced.** ABBYY was the accuracy benchmark in 2017. In 2024 Textract, Azure and Google Document AI were still called the most accurate for forms and tables. In 2025–26 several users report replacing them with Gemini or newer models.
- "ABBYY has dominated the field for many years (decades really) and still outperforms every solution out there." ([15667504](https://news.ycombinator.com/item?id=15667504), 2017-11-10)
- "Nothing else listed in the comments is as accurate, especially when trying to extract forms, tables and text." ([41108191](https://news.ycombinator.com/item?id=41108191), 2024-07-30)
- "Modern VLMs put classic FineReader to shame for processing low-resolution/degraded/non-standard text." ([48646326](https://news.ycombinator.com/item?id=48646326), 2026-06-23)

**Marker, Docling and Nougat: the open model-based converters.** Marker (2023) was welcomed as faster and broader than Nougat, then criticised for tables and multi-column pages. Docling became the most named open model-based converter of 2023–26 (110 comments), praised for structure and for running locally. Users call both slow.
- "I did a quick side-by-side testing against Nougat, and it clearly works better." ([38486560](https://news.ycombinator.com/item?id=38486560), 2023-12-01, on Marker)
- "So far I've had great success using docling." ([43976001](https://news.ycombinator.com/item?id=43976001), 2025-05-13)
- "IBM's Docling project, which uses ML models, produced by far the best formatting" ([43977036](https://news.ycombinator.com/item?id=43977036), 2025-05-13)

**Gemini, GPT and Claude as extractors: easy, cheap to start, not trusted alone.** In 2023–26 GPT and Gemini were the two most named tools in the subset. Users praise how easy they are and how they handle layout. Those with high accuracy needs add OCR, a second model or a human check.
- "Gemini won because it was so darn easy to use and it worked with minimal effort." ([42953665](https://news.ycombinator.com/item?id=42953665), 2025-02-05, after replacing an OCR vendor)
- "A multimodal LLM can and will hallucinate data at arbitrary scale (phrases, sentences, etc.)." ([42955931](https://news.ycombinator.com/item?id=42955931), 2025-02-05)
- "It got about 20% of the dates wrong yet rated all as “high confidence”." ([48646343](https://news.ycombinator.com/item?id=48646343), 2026-06-23, on Claude Opus and receipts)

**Mistral OCR: excitement, then a measured view.** Each release drew hundreds of points. The first thread swung from praise to an independent benchmark that disputed the score; by 2026 users who process old or messy documents report good results, while bulk users call the price high.
- "Super fast and significantly more accurate than google, Claude and others." ([43283132](https://news.ycombinator.com/item?id=43283132), 2025-03-06)
- "Which is pretty far cry from the 95% accuracy they were advertising from their private benchmark." ([43285983](https://news.ycombinator.com/item?id=43285983), 2025-03-06)
- "I have been quite happy with Mistral OCR for the documents I needed to process" ([49291086](https://news.ycombinator.com/item?id=49291086), 2026-08-13)

**LlamaParse and other hosted parsers: useful, distrusted as closed services.** Hosted parsers (LlamaParse, Unstructured's API, Reducto) were recommended for tables. Many of the most positive comments come from their own founders and staff.
- "Llamaparse by LlamaIndex is probably SOTA at the moment and seems to have no problems with tables." ([41107099](https://news.ycombinator.com/item?id=41107099), 2024-07-30)
- "This is my problem with projects that start off as open source and become famous because of their community contributions" ([39446772](https://news.ycombinator.com/item?id=39446772), 2024-02-20)

## 5. How the decade changed

- **The tools changed and the wants stayed.** Per 1,000 subset comments, tables went from 121 to 115 to 144, reading order and layout from 40 to 39 to 36, and OCR or scan terms from 477 to 477 to 417. Over the same years LLM and VLM terms went from 0 to 5 to 404.
- **The goal moved from text and CSV to input for LLMs.** LLM-ready output (Markdown, JSON, chunks, RAG) went from 28 to 38 to 188 per 1,000; chunking from 8 to 7 to 41.
- **New failure words arrived with the models.** Hallucination terms went from 1 to 4 to 44 per 1,000, and benchmark talk from 3 to 4 to 41, as users tried to verify vendor claims. Font and encoding complaints fell from 36 to 22 per 1,000.
- **Classic libraries lost attention, not respect.** Their share fell from 13.6% to 7.6% of the subset, and Tabula/Camelot from 7–8% of comments a year in 2018–21 to 1.3% in 2026. Comments about them stay positive on speed and cost.
- **Rendering pages to images became common advice but never settled the debate.** Users suggested rasterising first throughout the decade (36, 65 and 49 per 1,000). In 2025 the "So you want to parse a PDF?" thread still split between people who parse millions of PDFs natively and people who rasterise everything. "Rendering is a different problem from understanding what's rendered." ([44782662](https://news.ycombinator.com/item?id=44782662), 2025-08-04)
- **Trust became the open question.** In 2016–22 the question was whether a tool could get the text at all. In 2023–26 it is whether you can check what a model returned: bounding boxes, citations, confidence scores (11 per 1,000, up from 6 and 1) and human review (19 per 1,000, up from 13 and 8).

## 6. What this means for a PDF extraction library built today

Lamina PDF already reads native text with reading order, tables, Markdown, JSON and chunks in Rust and WASM. The evidence points to these priorities.

1. **Ship a native-text path that is fast and gives the same output every time, and say so in the docs.** Users complain about model hallucination (205 comments), slowness (112), price (98) and output that changes between runs (55). Bulk users still count pages per dollar (Tika's 30,000 per dollar). Publish pages per second, memory and byte-identical output as features. Lamina's extraction JSON is already checked byte for byte against goldens; the docs should make that a headline.
2. **Tell users which pages need OCR or a VLM.** Scanned pages come up in 617 comments, and one user puts pages whose text is images or paths at 40%. The hybrid designs users praise send only the hard pages to a model (41206121, 49290831). Give each page a signal: no text layer, the share of unmapped glyphs, invisible or hidden text, text that does not match the rendering. Lamina already counts dropped and unmapped text; make it a per-page "needs OCR" flag that the CLI and SDK can route on.
3. **Treat tables as a first-class product.** Tables are both the most stable want (810 comments) and the top complaint (246). Merged cells, empty cells, borderless tables, tables split over pages and headers are the named failures. Lamina's lattice and stream detectors and cell spans fit; add tables that continue over pages, test against a public table benchmark, and publish the results.
4. **Put a bounding box and a page on everything.** Citations and grounding are "non-negotiable" for some buyers (42954289), and VLMs are poor at boxes (42953466). Lamina's top-left page coordinates per block, cell and chunk region suit this. Keep boxes on lines and words too, so a viewer can highlight the source of any answer, a job the web reader can do.
5. **Get reading order and page furniture right, and say when you are unsure.** Multi-column order (226 comments) and headers and footers (95) are long-running complaints that models only partly fix. Lamina's XY-Cut++ ordering and furniture removal target exactly these. Users cannot see when the order is wrong. Lamina warns with `extract-reading-order-ambiguous` for mixed writing modes; add the same kind of warning for uncertain column splits and show it in the CLI, the MCP tools and the JSON.
6. **Win on local, private and permissive.** Privacy worries date from 2017 (14594467), and AGPL and non-commercial weights stop adoption (22201037, 44937078, 38487108). An `MIT OR Apache-2.0` library that runs in the browser, on a server or on a phone, with no upload, answers both. ADR 0030's rule that models are licence-checked plug-ins matches what users ask for.
7. **Make chunks follow structure, not size.** Users say fixed-size chunks hold RAG back (42955403) and ask for heading-aware splitting (41108477). Lamina's chunks already carry `headingPath`, pages and regions; heading detection, list items and footnotes are the parts users will judge. Use the tagged structure tree when a file has one, but expect most files to lack it: tagged PDF comes up in only 54 comments.
8. **Publish honest benchmark numbers and robustness figures.** Users distrust vendor scores (43285983) and ask for benchmarks they can rerun (163 comments). They also find that most parsers fail on part of a real corpus (44785629). Publish olmOCR-bench scores (Lamina already has `cargo xtask extract-bench`), the share of a public corpus opened without error, and the fuzzing record. Hard evidence is the strongest argument a deterministic parser has against a model.

## 7. Method and limits

**What I read.** I read about 900 comments in full or in long excerpts. They came from the main threads: "What's so hard about PDF text extraction?" (2020), "Ingesting PDFs and why Gemini 2.0 changes everything", Mistral OCR (2025) and 4.1 (2026), "So you want to parse a PDF?", "PDF to Text, a challenging problem", "Ask HN: What are you using to parse PDFs for RAG?", "LLM-aided OCR", Marker, Camelot (2018), PDFLayoutTextStripper (2017), Copyfish (2017), "Using Tesseract OCR with Python" (2017), Tesseract.js (2019), horse-racing charts with PDFBox (2017), olmOCR, LlamaParse, Zpdf, and Mistral OCR 3 and 4. I also read a random sample of up to 70 extraction comments per period from other threads.

**How I counted.** `tools/hn-research/counts/extraction/extraction_counts.py` loads `areas/parsing.jsonl`, drops hiring, freelancer and Typst threads, and keeps comments whose text or thread title matches an extraction regex (`REL`: extract, OCR, Tesseract, pdftotext, parse plus PDF, PDF to Markdown/JSON, RAG, tool names and others). Each theme is a case-insensitive regex over the comment text; a few also need an extraction word within 150 characters. `HN_DATA=<dir> python3 -I tools/hn-research/counts/extraction/extraction_counts.py` prints every count in this report, and `--samples <theme> <n>` prints random matches for checking. I read 6–12 random matches per theme and give the share on topic as "precision". The table and layout complaint counts are co-occurrences with a negative word, so they are rough. Quotes were checked by script to be exact substrings of the comment text, at most 40 words.

**Limits.**
- HN skews to developers, founders and people who build pipelines. Office workers who copy from PDFs are underrepresented, and many positive comments about hosted parsers come from their own staff (Reducto, doctly.ai, LLMWhisperer, Tensorlake).
- The data holds full threads only for stories with PDF-related titles, plus PDF-keyword comments elsewhere. Large threads such as the DeepSeek-OCR discussion are missing, and so are most launches not titled with "PDF".
- Comments are cut at 1,500 characters. 2016 covers only November and December, and 2026 runs to 5 November.
- Regexes miss paraphrases and catch other meanings ("table of contents", "marker", "figures" as numbers), so counts show relative size and trend, not exact totals. The low-precision themes (installs, determinism) should be read as signals only.
- One comment can count under several themes. Counts are comments, not people, and a few threads weigh heavily: the two 2020 "What's so hard about PDF text extraction?" threads hold 215 of the 1,424 comments for 2020–22 (15%), and the Gemini, Mistral OCR and "So you want to parse a PDF?" threads hold 579 of the 3,809 for 2023–26 (15%).
