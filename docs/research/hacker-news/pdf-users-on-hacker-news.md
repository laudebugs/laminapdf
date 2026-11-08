# Prove it, then close the trust gaps: what ten years of Hacker News say PDF users want

- **Researched:** 6 November 2026, from Hacker News stories and comments posted between 1 November 2016 and 6 November 2026
- **Notes:** [readers](notes/readers.md), [editors](notes/editors.md), [extraction](notes/extraction.md), [generation](notes/generation.md), [format and security](notes/format-and-security.md), [method](notes/method.md)
- **Scripts:** [`tools/hn-research/`](../../../tools/hn-research/README.md)
- **The work it leads to:** the [trust and proof plan](../../plans/trust-and-proof.md) (Round T)

PDF talk on Hacker News grew fourfold in ten years, from 204 stories in 2017 to 802 in 2026. People came to these threads to ask for help, recommend tools and complain, so the threads record what PDF users want from readers, editors, extraction libraries and generators, and what lets them down. This report reads 3,755 stories and 56,907 comments from those ten years. It covers what users ask for in each of the five areas, how the decade changed, and what Lamina PDF should build and say as a result. The finding is a paradox: the requests held steady while the tools serving them changed, so what users lack today is trust in the tools rather than features. **Lamina PDF should compete on correctness it can prove in public, and close the trust gaps that free tools leave open.**

## The data at a glance

The scripts collected every story with "PDF" or a PDF tool's name in its title, every comment in the 307 of those stories with 20 or more comments, and every comment anywhere on the site that matched one of 42 PDF search terms. [The method note](notes/method.md) gives the details.

| What | Count |
| --- | ---: |
| Stories found by title searches | 53,144 |
| Stories about PDF software, after dropping "[pdf]" link tags | 3,755 |
| Comments in the 307 stories with 20 or more comments | 27,185 |
| Comments found by 42 keyword searches across the site | 37,771 |
| Unique comments of 40 characters or more | 56,907 |
| Quotes in the notes and this report, each checked word for word against its comment | 472 |

The story filter, as run, matched tool names inside longer words, so 43 of the 3,755 stories are about acrobatics or names such as "Peritext". They move no year's count by more than 9, so the tables below keep them; [the method note §7](notes/method.md#7-limits) gives the full effect.

### PDF stories per year

| Year | PDF stories | Show HN launches | Show HN share | Median Show HN points |
| --- | ---: | ---: | ---: | ---: |
| 2017 | 204 | 48 | 24% | 3.5 |
| 2018 | 223 | 47 | 21% | 3 |
| 2019 | 220 | 51 | 23% | 2 |
| 2020 | 290 | 60 | 21% | 3 |
| 2021 | 279 | 37 | 13% | 3 |
| 2022 | 268 | 48 | 18% | 4.5 |
| 2023 | 372 | 85 | 23% | 3 |
| 2024 | 421 | 108 | 26% | 3 |
| 2025 | 643 | 234 | 36% | 2 |
| 2026 (to 6 November) | 802 | 359 | 45% | 2 |

### Stories per area per year

A story counts in an area when its title matches the area's pattern, so a story can count in several areas or in none.

| Area | 2017 | 2018 | 2019 | 2020 | 2021 | 2022 | 2023 | 2024 | 2025 | 2026 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Readers | 17 | 15 | 15 | 27 | 28 | 35 | 17 | 27 | 32 | 39 |
| Editors | 21 | 31 | 26 | 51 | 58 | 58 | 54 | 59 | 102 | 166 |
| Extraction | 49 | 55 | 60 | 64 | 54 | 61 | 121 | 144 | 246 | 258 |
| Generation | 48 | 46 | 56 | 51 | 54 | 72 | 72 | 100 | 142 | 193 |
| Format and security | 12 | 17 | 26 | 25 | 22 | 17 | 22 | 37 | 48 | 40 |

## Six patterns run through all five areas

One agent studied each area. The six patterns below appeared in the evidence of more than one area.

### 1. PDF talk quadrupled, and extraction and generation drove most of the growth

PDF stories rose from 204 in 2017 to 802 in 2026, counted only to 6 November. Extraction stories grew fivefold, from 49 to 258, and generation stories fourfold, from 48 to 193. Editor stories took off later, rising from 59 in 2024 to 166 in 2026, as in-browser tools arrived. Reader stories stayed between 15 and 39 a year, so readers are now the smallest area by volume, though their threads run long: the top reader story drew 375 comments.

### 2. The requests stayed put while the tools changed

Tables show the pattern most clearly. Extraction comments asked for tables at the same rate all decade, 115 to 144 per 1,000 comments, and reading order held near 40 per 1,000. Meanwhile the tools people named changed completely:

| Year | Extraction comments naming an LLM or vision model |
| --- | ---: |
| 2022 | 1.2% |
| 2023 | 21% |
| 2024 | 38% |
| 2025 | 47% |
| 2026 (to 5 November) | 44% |

Comments naming a classic library such as pdftotext, PDFBox, pdfminer or Tabula fell from 13.6% of extraction comments in 2016–22 to 7.6% in 2023–26, though their tone stayed warm. Editors show the same pattern. People asked to fill forms, sign and redact in 2017, and they ask for the same jobs in 2026; what changed is that Preview, Firefox, Edge and self-hosted suites now do most of them.

### 3. Typst passed LaTeX in 2025, so developers will switch engines

Typst went from no mentions to the decade's favourite generation tool:

| Comments naming | 2017 | 2018 | 2019 | 2020 | 2021 | 2022 | 2023 | 2024 | 2025 | 2026 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| LaTeX | 155 | 151 | 183 | 202 | 159 | 223 | 347 | 466 | 623 | 283 |
| Typst | 0 | 0 | 0 | 2 | 0 | 23 | 170 | 437 | 734 | 476 |

These counts cover all 56,907 comments; the generation note's smaller set shows the same crossover in 2025 (679 against 667). Typst stories scored 6,532 points in 2023–26, against 2,816 for LaTeX stories. Users praise its single binary, instant compiles and JSON input, and they keep LaTeX for maths and for journals that demand LaTeX sources. Developers left an entrenched engine once a faster one with a simpler install arrived, which is the opening a new library needs.

### 4. Free basics moved into browsers and the OS, and Acrobat lost its goodwill

Firefox's form filling was the top reader and editor story of the decade (["Embedded PDF viewer in Firefox 81 supports filling forms"](https://news.ycombinator.com/item?id=24554581), 1,093 points). Preview, Firefox and Edge now cover forms, signatures and page tools for most people. Over the same years Acrobat's goodwill drained away. In the readers agent's hand-labelled samples of 20 comments per period:

| Acrobat comments | 2016–19 | 2020–22 | 2023–26 |
| --- | ---: | ---: | ---: |
| Positive | 5 | 4 | 1 |
| Negative | 6 | 7 | 9 |
| Negative, but needed for a task | 2 | 3 | 2 |

Acrobat near sign-in, subscription or upsell went from 4 to 16 to 15 comments, and Acrobat near AI from 0 to 2 to 17. Acrobat remains the fallback for redaction, unusual forms and OCR.

> "I hate Acrobat but it is still the best pdf program on Windows." ([2025-10-18](https://news.ycombinator.com/item?id=45631155))

### 5. Local, in-browser tools became the new hello-world app

Show HN posts made up 45% of PDF stories in 2026, up from 24% in 2017, at a median of 2 points. Titles that promise local, in-browser or no-upload processing went from at most 3 a year to 16 in 2025 and 53 in 2026. Commenters have tired of them, and some suspect machine-written clones. Privacy now counts as table stakes, and it no longer sets a tool apart.

> "Seems like Clientside PDF editors are the new "hello world" app these days." ([2026-01-19](https://news.ycombinator.com/item?id=46675926))

### 6. Trust sits under the sharpest complaint in every area

Each area's worst complaint describes a broken promise:

| Area | The broken promise | Evidence |
| --- | --- | --- |
| Readers | Opening a file can compromise the machine | 370 reader comments on security; the Acrobat root exploit drew 871 points |
| Editors | Saving damages the file | "Preview in macOS Big Sur is destroying PDFs", 359 points and 317 comments |
| Editors, format | A black box leaves the text in the file | X-ray, a library that finds bad redactions, drew 709 points |
| Extraction | Models invent or drop content | hallucination talk rose from 1 to 44 per 1,000 extraction comments |
| Generation | New engines break non-Latin scripts | Arabic set as isolated letters, Devanagari with dotted circles |
| All | A free library carries an AGPL licence | AGPL warnings about Ghostscript, iText and MuPDF in every period |

Each failure is specific and testable, which favours a library that already tests for them.

## What each area asks for

Counts are comments that match a theme's pattern; agents read samples to check them. Generation and format and security give estimated on-topic counts, marked ~. Read all counts as relative sizes; each note gives the patterns, the spot-check scores and the trend by period.

### Readers

Readers want a quick, quiet viewer that handles long documents on any screen. SumatraPDF was the decade's most loved reader, with mentions up fivefold from 48 to 246, while Acrobat was the most discussed and the most resented. Browser viewers became the safe default for casual reading, and heavy readers still want a dedicated app for big files, printing and speed. [Full note](notes/readers.md).

| Most wanted | Comments | Most complained about | Comments |
| --- | ---: | --- | ---: |
| Tablet and e-ink reading | 1,020 | Readers as a security hole | 370 |
| Annotation and highlighting | 679 | Slow, bloated software | 260 |
| Reflow for phones | 501 | Crashes and memory use | 83 |
| Forms and signing without Acrobat | 283 | Rendering faults | 70 |
| Speed and a small install | 253 | Upsells and sign-in walls (rising) | 66 |
| Search, outline, link previews, last position | 47–87 each | Forms that open only in Acrobat | 62 |
| A dark page that keeps images (rising) | 90 | AI features nobody asked for (all since 2023) | 26 |

The biggest reader threads: Firefox 81's form filling (1,093 points), [SumatraPDF](https://news.ycombinator.com/item?id=37993575) (586), [Google Scholar PDF Reader](https://news.ycombinator.com/item?id=39768438) (422), [EmbedPDF](https://news.ycombinator.com/item?id=44901683), a PDFium viewer in WebAssembly (357), and three launches of [Sioyek](https://news.ycombinator.com/item?id=40313143), a viewer for papers (272, 238 and 336). AI readers drew little. Of 27 Show HN reader posts in 2023–26, the 11 with AI, chat or MCP in the title had a median of 2 points and a top score of 8. The other 16 had a median of 4 and a top of 357.

> "I like my reader to simply be a reader instead of a 13GB bloated piece of garbage." ([2025-10-16](https://news.ycombinator.com/item?id=45604604))

> "I counted no fewer than six buttons for the AI assistant but couldn't figure out how to see chapters for the document I was reading." ([2025-10-16](https://news.ycombinator.com/item?id=45604361))

### Editors

Editor users want small jobs done: sign, mark up, fill, OCR and reorder pages, for free and without uploading. Signing peaked with the pandemic, redaction became the headline failure in 2025, and the tools moved into the OS, the browser and self-hosted suites such as Stirling-PDF. Acrobat still sets the bar for redaction and unusual forms, and the gaps that remain concern correctness. [Full note](notes/editors.md).

| Most wanted | Comments | Most complained about | Comments |
| --- | ---: | --- | ---: |
| Sign documents | 1,129 | Acrobat's price and bloat | 258 |
| Annotate and highlight | 780 | No full editor on Linux | 139 |
| Fill in forms | 776 | Demands for a printed, signed scan | 124 |
| Make scans searchable | 509 | Forms that only work in Acrobat | 121 |
| Command-line tools | 433 | Annotations lost between apps | 119 |
| Merge, split and reorder pages | 356 | "Editors" that only add layers | 98 |
| Redaction that removes the text | 328 | Redactions that leave the text | 79 |

The biggest editor threads: five tools that fake a hand-signed scan, such as [FalsiScan](https://news.ycombinator.com/item?id=30024165) (3,050 points between them), a [free in-browser editor](https://news.ycombinator.com/item?id=43880962) (791), [X-ray](https://news.ycombinator.com/item?id=46369923) (709), [PDF Tool](https://news.ycombinator.com/item?id=37110628), which edits without uploading (522), and [Stirling-PDF](https://news.ycombinator.com/item?id=38765627) (507). Redaction comments rose from 2.8 to 8.0 per 1,000 between 2020–22 and 2023–26.

> "Placing a black box on the text isn’t a redaction any more than placing a sticky note would be." ([2025-12-24](https://news.ycombinator.com/item?id=46374309))

> "I'll stop trying to edit PDFs when people stop sending me PDFs that I want to edit." ([2023-09-04](https://news.ycombinator.com/item?id=37385504))

### Extraction

Extraction requests held steady: tables, scanned pages and reading order lead every period. The goal moved from text and CSV to input for language models: talk of Markdown, JSON, chunks and RAG went from 28 to 188 per 1,000 extraction comments. The tools moved from Tesseract and Tabula to Gemini, GPT and Mistral OCR, and the models brought new complaints about invented text, price and output that changes between runs. In 2016–22 the question was whether a tool could get the text at all; in 2023–26 it is whether a user can check what a model returned. [Full note](notes/extraction.md).

| Most wanted | Comments | Most complained about | Comments |
| --- | ---: | --- | ---: |
| Tables as rows and cells | 810 | Tables come out broken | 246 |
| Markdown, JSON and chunks for LLMs | 791 | Models invent or drop text | 205 |
| Text from scanned pages | 617 | Garbled text from fonts | 160 |
| Low cost per page | 570 | Slow tools | 112 |
| Local, private processing | 279 | Price | 98 |
| Reading order across columns | 226 | Headers and footers in the text | 95 |
| Bounding boxes for citations | 155 | Files the tool can't open | 85 |

The biggest extraction threads: [Mistral OCR](https://news.ycombinator.com/item?id=43282905) (1,756 points, the decade's top extraction story), ["Ingesting PDFs and why Gemini 2.0 changes everything"](https://news.ycombinator.com/item?id=42952605) (1,303), ["What's so hard about PDF text extraction?"](https://news.ycombinator.com/item?id=22473263) (733), [converting election PDFs to spreadsheets](https://news.ycombinator.com/item?id=35272227) (716) and [Marker](https://news.ycombinator.com/item?id=38482007), PDF to Markdown (683).

> "Tables are still the big unsolved problem for me." ([2024-03-30](https://news.ycombinator.com/item?id=39879481))

> "When a LLM hallucinates, there is no limit to how much text can be made up." ([2025-02-05](https://news.ycombinator.com/item?id=42956082))

### Generation

Developers want to write Markdown or fill a template from JSON and get correctly paginated pages, fast, under a licence they can ship. The talk moved from wrapping a browser (wkhtmltopdf, then headless Chrome) to skipping it (WeasyPrint, Prince, Typst as a back-end renderer), and from LaTeX to Typst. Compliance grew from a footnote to a requirement: comments naming PDF/UA or tagged PDF went from 5 to 9 to 36 by period, and e-invoicing from 1 to 3 to 14. [Full note](notes/generation.md).

| Most wanted | Comments | Most complained about | Comments |
| --- | ---: | --- | ---: |
| Markdown or other plain markup in | ~1,140 | Typst's gaps, and journals that want LaTeX | ~176 |
| A template filled from data | ~560 | LaTeX's install and errors | ~137 |
| Maths that looks right | ~470 | AGPL licences | ~116 |
| Charts and SVG on the page | ~370 | Pagination from HTML | ~100 |
| Speed at scale | ~350 | The weight of headless Chrome | ~92 |
| Control over page breaks and headers | ~345 | Inaccessible output | ~89 |
| Tagged, accessible output | ~200 | Arabic and Devanagari set wrong | ~50 |

Typst drew 130 stories in 2023–26, led by ["Typst: A Possible LaTeX Replacement"](https://news.ycombinator.com/item?id=45393842) (786 points). The other big generation threads: ["Show HN: PDFs from HTML"](https://news.ycombinator.com/item?id=26691626) (742), [Invoice Dragon](https://news.ycombinator.com/item?id=36860898) (564), [TinyPDF](https://news.ycombinator.com/item?id=46316968) (253), which commenters found handles ASCII only, and [PaperPilot](https://news.ycombinator.com/item?id=50238739) (236), a pure-Rust engine with WASM and MCP whose pitch is the closest to Lamina PDF's.

> "The holy grail of PDF generators, sensible page breaks. Never been done and makes peace in the middle east seem like an easy task." ([2017-08-22](https://news.ycombinator.com/item?id=15071383))

> "The Arabic is missing the joining (all forms are isolated), and the Devanagari doesn't have the vowels combining (so you see those dotted circles)." ([2026-03-02](https://news.ycombinator.com/item?id=47214003))

### Format and security

People praise PDF as a fixed copy that looks the same everywhere, and fear it as a carrier of exploits. Worry about exploits held steady at 10 to 13 per 1,000 comments all decade, while the culprit moved from Acrobat to every parser, browser viewers and server-side Ghostscript included. Interest moved from forging signatures in 2020–22 to forensics on released files in 2023–26. Hacks that push the format past its purpose, such as Tetris in a PDF, drew the biggest crowds. [Full note](notes/format-and-security.md).

| Most wanted | Comments | Most complained about | Comments |
| --- | ---: | --- | ---: |
| Safe opening of untrusted files | ~270 | Readers get exploited | ~360 |
| A file that looks the same everywhere | ~210 | Unreadable on phones | ~270 |
| PDF/A for archives | ~180 | Inaccessible to screen readers | ~270 |
| Processing on my own machine | ~150 | Code inside documents | ~170 |
| A permissive licence | ~150 | Easy to forge | ~160 |
| Signatures that prove a file is genuine | ~140 | Redactions that leave the text | ~150 |
| Redaction that removes the text | ~100 | AGPL traps in free libraries | ~90 |

The biggest threads: ["Show HN: Tetris in a PDF"](https://news.ycombinator.com/item?id=42645218) (1,289 points), [Acrobat Reader flaws that give root on macOS](https://news.ycombinator.com/item?id=23180975) (871), ["Making a PDF that's larger than Germany"](https://news.ycombinator.com/item?id=39210507) (712), X-ray (709) and [recreating Epstein PDFs from raw attachments](https://news.ycombinator.com/item?id=46890335) (544). Eleven forensics and leak threads hold 1,414 comments between them.

> "The problem with PDF is that it's a bag of needles disguised as a piece of paper." ([2020-05-14](https://news.ycombinator.com/item?id=23184955))

> "It would be better to strip active content like javascript and actions, without flattening the PDF and losing all the text data" ([2026-01-22](https://news.ycombinator.com/item?id=46722698))

## Ten recommendations, ranked by evidence and fit

Each recommendation gives the evidence, what Lamina PDF has today, the gap and the work that closes it. **Shipped** means the code exists on `main`; **Planned** means a todo on the board covers it; **Gap** means only this research's new todos cover it. The ranking weighs the size of the evidence, how well the gap fits Lamina PDF's core, and how little else serves it.

Four steps need docs, a CI rule or a small check rather than a new feature: publishing the evidence (R1), the fake-redaction check (R2), performance budgets in CI (R6) and the website changes (R9).

### R1. Prove correctness in public

**Status:** Shipped engine; the proof is a gap.

Features no longer set a PDF tool apart: 359 Show HN PDF posts appeared in 2026, at a median of 2 points. Users name specific failures instead: [a merge that drops links](https://news.ycombinator.com/item?id=43888208), [compression that turns every page into a picture](https://news.ycombinator.com/item?id=43895227), a save that scrambles the text layer, and parsers that fail on real files. They distrust vendor scores and ask for numbers they can rerun (163 extraction comments on benchmarks).

> "Only three of them can process all 2500 files i tried" ([2025-08-04](https://news.ycombinator.com/item?id=44785629))

- **Lamina PDF today:**
  - byte-identical output
  - fuzzing
  - tolerant reading with repair warnings (ADR 0003)
  - incremental saves that keep the original bytes (ADR 0007)
  - veraPDF on every example that claims a profile
  - round trips checked by qpdf, pdf.js and PDFium
  - the nightly differential against pdf.js, PDFium, qpdf and pypdf
- **The gap:** none of this evidence reaches users.
- **Work:** [T1a](../../todos/ready/T1a-nightly-proof-figures.md) records the nightly's figures as data, and [T1b](../../todos/ready/T1b-how-we-test-page.md) publishes them on the website with the licence and where the code comes from. [U5b](../../todos/ready/U5b-reader-benchmark.md) compares the reader with others.

### R2. Ship real redaction, and catch fake redaction in any file

**Status:** Planned; the check is a gap.

Redaction comments rose from 2.8 to 8.0 per 1,000 editor comments in 2023–26, driven by the Epstein releases. Users say no free tool redacts properly, on Linux least of all, and one in-browser editor [left redaction out because its library was AGPL](https://news.ycombinator.com/item?id=43882632). Black boxes are only half the failure: incremental saves keep earlier revisions in the file.

> "meaning the previous revision of the file is available inside of the PDF itself." ([2025-12-23](https://news.ycombinator.com/item?id=46370872))

- **Lamina PDF today:** [E12a](../../todos/ready/E12a-redaction-feature-plan.md)–[E12f](../../todos/ready/E12f-leak-tests.md) plan mark and review, removal of text, pixels and paths, a scrub with a full rewrite, sensitive-content search and leak tests.
- **The gap:** nothing finds a failed redaction in a file someone else made, and nothing stops a user from drawing a box and calling it done.
- **Work:** [T2a](../../todos/ready/T2a-covered-text-flag.md) teaches extraction to flag text hidden under later shapes and images, and [T2b](../../todos/ready/T2b-check-failed-redactions.md) makes `laminapdf check` warn about covered text, unapplied redaction annotations and text left in earlier revisions. [E4f](../../todos/ready/E4f-annotation-tools-ui.md) gains a notice when a user draws an opaque shape over text, offering the redaction tool instead ([38549034](https://news.ycombinator.com/item?id=38549034)).

### R3. Finish forms and signatures, and name XFA files plainly

**Status:** Planned.

Forms and signatures keep people on Acrobat. Signing (1,129 comments) and form filling (776) are the top editor requests, and Firefox's form filling was the decade's top reader story. XFA-only government forms are the most cited dead end (121 comments), and five tools that fake a hand-signed scan drew 3,050 points from people protesting print-sign-scan rules. A smaller, steady group wants certificate signatures they can verify offline.

> "they really need to provide a way to verify a document without the site being up and intact." ([2025-09-26](https://news.ycombinator.com/item?id=45381358))

- **Lamina PDF today:** E8 plans the AcroForm field model, value appearances checked in pdf.js and PDFium ([E8d](../../todos/ready/E8d-field-appearances.md)), data import and export, and XFA detection with a report. E10 plans visual signatures labelled as non-cryptographic, PAdES signing, and local validation with a diff of each revision, tested against shadow attacks.
- **The gap:** none beyond the plans. This research raises their priority.
- **Work:** the E8 and E10 rounds; their feature plans ([E8b](../../todos/ready/E8b-form-filling-feature-plan.md), [E10b](../../todos/ready/E10b-signatures-feature-plan.md)) should cite this evidence.

### R4. Make extraction checkable

**Status:** Shipped core; the trust signals are a gap.

Users once asked whether a tool could get the text at all; they now ask whether they can check what it returned. Hallucination talk rose from 1 to 44 per 1,000 extraction comments, and one vendor's customers treat bounding boxes as a must. Scanned pages come up in 617 comments, and users favour pipelines that send only the hard pages to a model.

> "Fields that aren't in the document are being made up 11 to 40% of the time depending on the API." ([2026-09-02](https://news.ycombinator.com/item?id=49536697))

> "we’ve found bounding box based attribution to be a non-negotiable for many of our current customers." ([2025-02-05](https://news.ycombinator.com/item?id=42954289))

- **Lamina PDF today** (ADR 0030):
  - native extraction with XY-Cut++ reading order
  - ruled and unruled tables
  - running headers and footers removed
  - boxes on blocks, cells and chunk regions
  - JSON that matches its goldens byte for byte
  - an extraction benchmark, `cargo xtask extract-bench`
- **The gap:** a pipeline can't tell which pages need OCR, can't see when the column order is a guess, and gets a table split in two at every page break. The benchmark scores sit in a crate README.
- **Work:** [T3a](../../todos/ready/T3a-text-layer-status.md) adds a per-page text-layer status, [T3b](../../todos/ready/T3b-reading-order-confidence.md) a warning when the column order is uncertain, [T3c](../../todos/done/T3c-tables-across-pages.md) joins tables across pages, and [T1c](../../todos/ready/T1c-publish-performance-numbers.md) publishes the scores and speed.

### R5. Decide for flow layout, then accept Markdown and JSON

**Status:** Gap.

Control over pages is the generation request with the clearest gap (~345 comments): page breaks that keep rows together, running headers and footers, "page X of Y", repeated table headers and footnotes. Browsers fail at these jobs, and Typst wins at them. The two largest generation requests are writing Markdown (~1,140) and filling a template from JSON (~560), and talk of AI near generation rose from 6 to 263 comments, so a format an LLM can write matters too.

> "I make invoices by having a Typst template read line items from a JSON file." ([2024-03-11](https://news.ycombinator.com/item?id=39672973))

- **Lamina PDF today:** `laminapdf-build` places text runs and breaks lines inside one box, reporting what overflows. The [library plan](../../plans/pdf-library.md) lists a flowing layout engine as a non-goal for now, and its open question 4 asks whether layout should become a library goal.
- **The gap:** the most common unmet generation need.
- **Work:** [T4a](../../todos/ready/T4a-adr-0032-layout-engine.md) answers open question 4 in ADR 0032, with a spike that weighs an engine of our own over `Surface` against embedding Typst. [T4b](../../todos/ready/T4b-flow-layout-core.md), [T4c](../../todos/ready/T4c-flow-tables-notes-contents.md), [T4d](../../todos/ready/T4d-markdown-documents.md) and [T4e](../../todos/ready/T4e-json-documents-and-templates.md) build what it decides. [T1c](../../todos/ready/T1c-publish-performance-numbers.md) publishes generation timings: a Typst user reports [under 100 ms for a two-page invoice](https://news.ycombinator.com/item?id=45697377).

### R6. Keep the reader fast, quiet and safe

**Status:** Shipped and planned.

Speed draws the most reader praise and slowness the second-largest complaint, 470 comments together. Security is the largest reader complaint (370). Acrobat's positive sample fell from 5 in 20 to 1 in 20 as upsells and AI prompts grew, while SumatraPDF's mentions rose fivefold for speed and silence.

> "I have ZERO idea why anyone would want AI inside a program used to view documents." ([2026-08-18](https://news.ycombinator.com/item?id=49344941))

- **Lamina PDF today:** PDFium in a document worker (ADRs 0004 and 0009), telemetry and hosted features only by opt-in (ADR 0011), and the viewer performance harness.
- **The gap:** none beyond the plans.
- **Work:** [C7c](../../todos/ready/C7c-performance-gates-and-launch.md) turns the performance targets into gates, [U4a](../../todos/ready/U4a-network-indicator-and-privacy-proof.md) shows the network indicator, and [U5a](../../todos/ready/U5a-pricing-and-promises-page.md) publishes the promises.

### R7. Serve long reading: reflow, a dark page, papers and manuals

**Status:** Planned; two features are a gap.

Reflow for phones comes up in 501 reader comments, and dark mode in 90, a rising share. [Veil](https://news.ycombinator.com/item?id=47529306) drew 100 points for one detail: it keeps images in their real colours. Readers of papers and manuals want a panel listing every search match, an outline, link previews with a way back, and reopening at the last page.

> "if I mouseover an internal link in a pdf, it shows a little preview of where that link would take me" ([2024-03-24](https://news.ycombinator.com/item?id=39809245))

> "when you re-open a PDF, opens it at the position you were last time(!!!)." ([2017-07-24](https://news.ycombinator.com/item?id=14839643))

- **Lamina PDF today:** ADR 0024 §5 fixes a dark page that recolours text and paths and never images ([U2c](../../todos/ready/U2c-page-appearance-modes.md)). Reader mode comes from the structure tree, then by approximation ([U4b](../../todos/ready/U4b-reader-mode-from-structure-tree.md), [U4c](../../todos/ready/U4c-approximate-reader-mode.md)). Find lists every match ([C5b](../../todos/ready/C5b-find-in-document.md)), and [C6a](../../todos/ready/C6a-thumbnails-outline-page-labels.md) and [C6b](../../todos/ready/C6b-links-and-history.md) plan the outline and links with history. The performance harness already scrolls 1,000 pages.
- **The gap:** previews of internal links and citations, and reopening each file at its last position.
- **Work:** [T5a](../../todos/ready/T5a-link-previews.md) and [T5b](../../todos/ready/T5b-reopen-at-last-position.md).

### R8. Sanitise without flattening, and report what a file reveals

**Status:** Planned; the report is a gap.

Safe opening of untrusted files is the top request about the format (~270 comments). Users want active content stripped without flattening the page to an image, which loses the text that [Dangerzone](https://news.ycombinator.com/item?id=22516496)'s render-and-OCR approach throws away. Forensics threads hold 1,414 comments, and a study of [Snowden PDFs' metadata](https://news.ycombinator.com/item?id=46564762) drew 324 points.

- **Lamina PDF today:** [E11d](../../todos/ready/E11d-sanitiser.md) plans a sanitiser that reports what it removed, and [E3g](../../todos/ready/E3g-inspectors.md) inspectors for objects and revisions.
- **The gap:** a read-only report, from the CLI and the MCP server, of what a file discloses beyond its visible pages, for journalists checking a released file and for publishers before release.
- **Work:** [T2c](../../todos/ready/T2c-reveal-command.md) adds `laminapdf reveal`.

### R9. Say loudly what Lamina already has

**Status:** Shipped; the message is a gap.

Licences decide which library people pick (~150 comments), and warnings about AGPL in Ghostscript, iText and MuPDF recur in every period. Commenters beg new generators for tagged output, and new engines get caught on Arabic and Devanagari within hours of launch. In October 2026, a commenter met a new Rust engine with doubt about where its code came from.

> "only commercial pdf libraries support PDF/A." ([2025-08-04](https://news.ycombinator.com/item?id=44782733))

> "Is this PDF engine vibe-coded and derived from reading the MuPDF source code?" ([2026-10-26](https://news.ycombinator.com/item?id=50239092))

> "every single release changed something in the output." ([2018-07-19](https://news.ycombinator.com/item?id=17567576))

- **Lamina PDF today:**
  - byte-identical output
  - MuPDF prohibited (ADR 0004)
  - `MIT OR Apache-2.0` with no AGPL or GPL dependencies (ADR 0001)
  - HarfRust shaping and the `multilingual` example
  - an invoice with embedded XML (`pdf_a3_invoice_with_xml`)
  - PDF/UA-1 and PDF/A-2b, 2u and 3b profiles, checked by veraPDF every night
- **The gap:** the website and docs don't say so where people decide.
- **Work:** [T1b](../../todos/ready/T1b-how-we-test-page.md) states the licence and where the code comes from; [T1d](../../todos/done/T1d-snapshot-testing-guide.md) shows users how to snapshot-test their own PDFs. Whether tagged output becomes the default is an open question in the [plan](../../plans/trust-and-proof.md).

### R10. Grow the CLI and MCP server for chained jobs

**Status:** Shipped CLI; the chained commands are a gap.

Developers already script pdftk, qpdf and Ghostscript (433 comments) and ask for chains of steps. Compression tools disappoint, and AI form filling [has started to appear](https://news.ycombinator.com/item?id=47984675).

> "First merge all files then depending on output size compress to fit the size and other requirements?" ([2026-01-19](https://news.ycombinator.com/item?id=46675707))

> "every “shrink” tool we tried barely compressed the size, and some even made it larger!" ([2025-05-04](https://news.ycombinator.com/item?id=43884344))

- **Lamina PDF today:** the `laminapdf` CLI's `info`, `check`, `pages rotate`, `extract` and `explain`; the MCP server, read-only unless started with `--allow-write` (ADR 0031). The library plans merge and split (E2), the optimiser (E16) and batch runs (E14).
- **The gap:** none of the planned page and size operations reaches the CLI.
- **Work:** [T6a](../../todos/ready/T6a-cli-page-commands.md) adds `laminapdf pages` merge, split, delete, extract, reorder and insert; [T6b](../../todos/ready/T6b-cli-optimize-target-size.md) adds `laminapdf optimize --target-size`. MCP tools that fill forms or generate files need an ADR first (an open question in the plan).

## Four things the evidence argues against

1. **Don't put AI in the reader by default.** AI reader launches had a median of 2 points; Acrobat's AI buttons drew anger in every thread that mentioned them.
2. **Don't lead with "local and private" alone.** In 2026, 53 Show HN titles made that promise, at a median of 2 points. Keep the guarantee, and lead with what the tool does correctly.
3. **Don't promise Word-style text editing.** Subset fonts often lack the glyphs a fix needs ([43883065](https://news.ycombinator.com/item?id=43883065)), and users mock "editors" that only add layers ([40398095](https://news.ycombinator.com/item?id=40398095)). E7 should ship small fix-a-word edits that check the font's glyphs and warn before substituting a font ([E7f](../../todos/ready/E7f-font-handling-and-styling.md)).
4. **Don't flatten pages to images to make files safe or small.** Rendering pages to pictures loses the text, search and accessibility users want to keep.

## How this fits the earlier product research

The [product and UX research](../product/pdf-app-product-and-ux.md) of October 2026 reached several of the same conclusions from competitor teardowns and reviews: win on trust, speed and restraint; redaction with a verification report (its R12); published benchmarks and a no-hostile-monetisation pledge (R18); page appearance modes (R8); and two-tier reflow (R15). Ten years of comments support each of them with counts. This research adds four points the earlier report didn't make: extraction must say when it is unsure, generation needs flowing layout and data inputs, the licence and the code's origin decide adoption, and AI inside a reader costs goodwill.

## Method and limits

Scripts in [`tools/hn-research/`](../../../tools/hn-research/README.md) collected the stories and comments through the HN Search API, sorted the comments into five areas, and counted themes with regular expressions. Five agents each read 900 to 1,400 comments of one area, wrote the theme patterns, and read 6 to 20 random matches per theme to measure how many were on topic. A script checked all 472 quotes, 446 in the notes and 26 here, word for word against their comments. [The method note](notes/method.md) gives every step.

The limits matter for how to read the numbers:

- Hacker News leans towards developers on Mac and Linux. Office workers, lawyers, government staff and blind readers appear mostly second-hand, so forms, signing and phone reading likely matter more to all users than these counts show.
- Counts measure comments, not people, and patterns miss paraphrases. Spot checks found between a third and all of the matches on topic, depending on the theme.
- Single events drive spikes: the pandemic lifted signing in 2020–22, and the Epstein releases lifted redaction in December 2025.
- The story set comes from title searches, so it misses stories that don't name PDF or a listed tool, and it includes 43 stories (1.1%) whose titles hold a tool's name inside another word.
- 2016 covers only November and December, and 2026 runs to 6 November.

## Conclusion: publish the proof, then close the trust gaps

People already have free tools for the everyday PDF jobs, and they distrust most of them, because those tools break files, leak redacted text, invent content or bind users to a licence. Lamina PDF already collects much of the proof that it behaves better, so publishing that proof is the cheapest next step. After that come the gaps that free tools leave open: real redaction with a check for fake ones, forms and signatures that work everywhere, extraction that says when it is unsure, and generated pages that flow. The [trust and proof plan](../../plans/trust-and-proof.md) turns each into todos.
