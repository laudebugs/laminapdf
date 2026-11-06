# PDF generation on Hacker News, Nov 2016 – Nov 2026

*Part of [What PDF users ask for: ten years of Hacker News](../pdf-users-on-hacker-news.md). How the data was collected: [method](method.md). The work it leads to: the [trust and proof plan](../../../plans/trust-and-proof.md).*

Reader: the Lamina PDF team, who know the product and want to know what developers say when they need to produce PDFs. Over ten years the talk moved from "which tool turns my HTML into a PDF" to "which engine lets me skip the browser", and typesetting moved from LaTeX to Typst. The demand underneath stayed the same: write content or data in a format people already know, get a correctly paginated, well-set PDF fast, and stay out of licence and infrastructure trouble.

All counts come from the scripts in `tools/hn-research/counts/generation/` (`generation_core.py`, `generation_counts.py`, `generation_extra.py`, `generation_stories.py`; their output is in `output.txt` there). `generation_verify_quotes.py` checks every quote against the data: all 89 are verbatim, at most 40 words, and carry the right date. Section 7 explains the method.

**The base for every count** is a "core" set of 9,523 generation comments: 1,708 in 2016–19, 1,944 in 2020–22 and 5,871 in 2023–26. Each comment falls in one or both of two lanes:

- **Lane A (HTML and code generation):** 5,129 comments.
- **Lane B (typesetting: LaTeX, Typst, Pandoc and others):** 5,400 comments.

Lane B grew from 40% of the core in 2016–19 to 64% in 2023–26, mostly because of Typst threads. That shift pulls the per-1,000 rates of lane A topics down in the last period.

## 1. What users want

Ranked by an estimate of on-topic comments: regex matches × the share that a hand spot-check found on topic. "Per 1,000" figures are per 1,000 core comments in 2016–19 / 2020–22 / 2023–26.

### 1.1 Write in plain-text markup, get a PDF

People want to write Markdown (or AsciiDoc, reStructuredText, Org) and get a typeset PDF without writing layout code.

- **Count:** 1,424 comments in 657 threads; spot-check 8/10 on topic, so about 1,140. Per 1,000: 154.6 / 160.0 / 144.6. 448 are in lane A; the rest sit in LaTeX, Typst and Pandoc threads.
- **Regex:** `markdown|\bmd\b|commonmark|asciidoc|\brst\b|restructuredtext|org[- ]mode`.

> "I find Markdown most natural for writing because I do not have to worry about formatting or syntax." — [2018-04-30, 16960256](https://news.ycombinator.com/item?id=16960256)

> "I switched from using MD-->(Pandoc-->(latex))--> PDF to using MD-->(Pandoc-->(typst))--> PDF." — [2025-03-02, 43232394](https://news.ycombinator.com/item?id=43232394)

> "I think if you have a markdown->PDF function included, where I can send in markdown and get PDF, that would solve quite many needs, and would be useful." — [2025-12-19, 46325132](https://news.ycombinator.com/item?id=46325132)

### 1.2 Fill a template from data

Invoices, reports, resumes and statements start as rows in a database or a JSON file, and people want a template that turns that data into pages.

- **Count:** 799 comments in 416 threads; spot-check 7/10, so about 560. Per 1,000: 82.6 / 78.7 / 86.0. 424 are in lane A.
- **Regex:** `templat|jinja|handlebars|liquid|mustache|razor|placeholder|mail merge|from (a |the )?(database|csv|json|yaml|spreadsheet)|(json|yaml|csv) (data|files?|input)`.

> "I have a database of content and then use SQL queries to extract data and then generate beautiful PDFs through paged.js." — [2021-04-04, 26691872](https://news.ycombinator.com/item?id=26691872)

> "I make invoices by having a Typst template read line items from a JSON file." — [2024-03-11, 39672973](https://news.ycombinator.com/item?id=39672973)

> "It’s absolutely trivial for example to build a Lambda function that parses some json input into a PDF. We’re using it to generate PDF invoices for our customers." — [2025-09-27, 45394409](https://news.ycombinator.com/item?id=45394409)

### 1.3 Maths that looks right

Scientists and students want equations set as well as TeX sets them. This want is the main reason LaTeX survives.

- **Count:** 755 comments in 374 threads; spot-check 5/8, so about 470. Per 1,000: 77.3 / 96.7 / 74.1. Only 113 are in lane A.
- **Regex:** `\bmaths?\b|equations?|formula|mathjax|katex|mathml`.

> "Well, LaTeX is great for mathematical symbols. I have not seen a typesetting in the web space that can compare itself to LaTeX." — [2020-09-11, 24445557](https://news.ycombinator.com/item?id=24445557)

> "LaTeX has no suitable alternative in its class (at least until now :p), meaning a FOSS software to produce high quality typesetting with an emphasis on maths." — [2025-06-22, 44351096](https://news.ycombinator.com/item?id=44351096)

### 1.4 Charts, SVG and images in the page

Reports carry charts, and people want the chart they already draw for the web (SVG, a JavaScript chart library) to land in the PDF as it is.

- **Count:** 494 comments in 312 threads; spot-check 6/8, so about 370. Per 1,000: 60.3 / 72.5 / 42.6.
- **Regex:** `\bsvg|charts?\b|\bplots?\b|diagrams?\b|tikz|\bd3\b|matplotlib|vector graphics|images? (in|into)|embed\w* images?|graphs? (in|into)`.

> "For instance, you display a chart using Javascript, and get exactly the same chart in the generated report." — [2018-06-29, 17424948](https://news.ycombinator.com/item?id=17424948)

> "we can use our existing frontend code/styling (including SVG graph generators) which is a huge bonus." — [2024-03-11, 39670235](https://news.ycombinator.com/item?id=39670235)

> "I use SVG, JavaScript to dynamically resize the text to fit a page, etc. Just works." — [2025-08-20, 44967038](https://news.ycombinator.com/item?id=44967038)

### 1.5 Speed and scale

Teams that generate thousands of documents measure milliseconds per document and documents per hour. This want grew over the decade.

- **Count:** 586 comments in 331 threads; spot-check 6/10, so about 350. Per 1,000: 53.9 / 50.4 / 67.5.
- **Regex:** a speed or scale word (`fast`, `speed`, `ms`, `at scale`, `thousands of`, `throughput`, `latency`, `compile time`, `instant`, `performance`, `quick`) within 80 characters of a PDF or engine word.

> "In one project we crunched out 150k PDF documents in less than forty minutes from roughly 6 GB input data, on a mid laptop" — [2025-02-20, 43118041](https://news.ycombinator.com/item?id=43118041)

> "We're getting around 500ms for a single document, which is (last I tested) quicker than any headless chrome setup." — [2024-03-11, 39669987](https://news.ycombinator.com/item?id=39669987)

> "I need to generate a 2 page invoice and i can generate it under 100 ms." — [2025-10-24, 45697377](https://news.ycombinator.com/item?id=45697377)

### 1.6 Control over pages

The PDF-specific features are what people ask for once content runs past one page:

- page breaks, and orphans and widows;
- running headers and footers;
- page numbers and a table of contents with page numbers;
- footnotes;
- table headers that repeat on each page.

- **Count:** 493 comments in 221 threads (page-layout terms or tables that span pages); spot-check 7/10, so about 345. Per 1,000 for the page-layout terms alone: 66.7 / 67.4 / 37.6.
- **Regex:** `page[- ]?breaks?|break-(before|after|inside)|headers? (and|&) footers?|running (head|foot)|page numb|counter\(page|footnotes?|widows?|orphans?|table of contents|\btoc\b|@page|paged[- ]media|margin box|paginat`, plus a tables-across-pages pattern.

> "the biggest gripe with generating (long) PDFs from HTML are things such as page numbering, orphans and widows, semantically correct word-wrapping, page margins, etc..." — [2018-04-30, 16959758](https://news.ycombinator.com/item?id=16959758)

> "Specifically, I'm hoping for repeating headers per page, and not having any problems with rows printing half on one page, and half on the next page." — [2017-10-05, 15410166](https://news.ycombinator.com/item?id=15410166)

> "If you want granular control over how your PDF will look with content that is more than one page long, you will have a hard time using html." — [2024-03-11, 39670140](https://news.ycombinator.com/item?id=39670140)

### 1.7 Reuse HTML and CSS skills

Web developers want to lay out documents with the HTML, CSS and component code they already know. A vocal minority disagrees.

- **Count:** 353 comments in 212 threads; spot-check 6/10, so about 210. Per 1,000: 38.6 / 43.2 / 34.6. 280 are in lane A.
- **Regex:** `html ?(\+|and|&|/) ?css|web (tech|developers)|front[- ]?end (dev|code|skills)|tailwind|\breact\b|\bvue\b|svelte|(same|existing) (html|css|templates?)`.

> "It makes it super easy for front end devs to layout and generate PDFs using tools they already know." — [2021-03-26, 26589210](https://news.ycombinator.com/item?id=26589210)

> "This is a task which is just crying out for a markup language and a stylesheet, not hundreds of lines of declarative C# code." — [2023-01-18, 34432018](https://news.ycombinator.com/item?id=34432018)

> "Using HTML/CSS for PDFs really just isn't a good idea in my experience. It makes layout extremely cumbersome." — [2023-01-18, 34432228](https://news.ycombinator.com/item?id=34432228)

### 1.8 Accessible, tagged output

Public-sector and university buyers need tagged PDF, often by law, and people see accessible generation as an unserved market.

- **Count:** 284 comments in 153 threads; spot-check 7/10, so about 200. Per 1,000: 23.4 / 44.2 / 26.9. The 2020–22 peak comes from two threads: "The sad state of PDF-Accessibility of LaTeX Documents" and "Ask HN: Why is the PDF format so inaccessible?"
- **Regex:** `accessib|tagged|pdf/ua|pdf-ua|screen[- ]?readers?|a11y|wcag|section 508|alt[- ]text|reading order|\bblind\b`.
- **Trend:** comments that name PDF/UA or tagged PDF outright rose from 5 to 9 to 36 per period (`generation_extra.py`).

> "I don't see any support for tagged PDF output. That's important for accessibility, particularly for screen reader users." — [2023-01-18, 34431828](https://news.ycombinator.com/item?id=34431828)

> "If you can nail accessible PDFs then you'd open up a very big government market." — [2024-03-11, 39670933](https://news.ycombinator.com/item?id=39670933)

> "We are required by accessibility law to make our PDFs WCAG compliant" — [2025-10-01, 45439507](https://news.ycombinator.com/item?id=45439507)

### 1.9 Generate in-process, from code

Many back-end developers want a library they call from their own language, not a separate browser process or a hosted API.

- **Count:** 390 comments in 224 threads; spot-check 5/10, so about 195. Per 1,000: 41.6 / 38.1 / 41.7. 283 are in lane A.
- **Regex:** `programmatic|from code|in code|code[- ]first|scripting|programmab|\bapi\b` within 80 characters of a PDF or engine word.
- **Typst signal:** a separate count found 94 comments in 2023–26, and none before, describing Typst run in production from data (invoices, reports, JSON, servers); spot-check 9/10.

> "As a web developer who uses C# on the backend, QuestPDF is exactly what I want." — [2023-01-18, 34432139](https://news.ycombinator.com/item?id=34432139)

> "Puppeteer or any nonembedded process is just unnecessary hassle/overhead in a lot of cases." — [2023-01-18, 34431468](https://news.ycombinator.com/item?id=34431468)

> "I embed the typst binary in a go binary deployed to cloud run. I use this to generate pdfs on the fly." — [2025-10-24, 45697377](https://news.ycombinator.com/item?id=45697377)

### 1.10 Good typography

People want hyphenation, justification, kerning and line breaking at TeX quality. Browser output is the usual counter-example.

- **Count:** 377 comments in 210 threads; spot-check 5/10, so about 190. Per 1,000: 53.3 / 46.8 / 33.2.
- **Regex:** `hyphenat|kerning|justif|microtyp|ligature|knuth|plass|line[- ]?break|typograph|letterspac|font features`.

> "The examples lack hyphenation, which partly explains the too-variable interword spacing." — [2018-04-30, 16960246](https://news.ycombinator.com/item?id=16960246)

> "You'd get more control over things like page numbering and TOCs, plus good justification/microtypography, which is important to most publishers." — [2018-04-30, 16960244](https://news.ycombinator.com/item?id=16960244)

> "Typesetting quality was the main reason we chose not to go down the Puppeteer/headless browser route" — [2024-03-11, 39669561](https://news.ycombinator.com/item?id=39669561)

### 1.11 Instant preview

Typst users praise compile-on-save, and developers want the same tight loop for their templates.

- **Count:** 181 comments in 131 threads; spot-check 6/8, so about 135. Per 1,000: 17.6 / 20.1 / 19.1.
- **Regex:** `live preview|hot[- ]?reload|watch mode|incremental (compil|render|layout|build)|wysiwyg|instant (preview|feedback|compil)|real[- ]?time preview|as you type|side[- ]by[- ]side`.

> "Instant compile. It just sits there waiting, and once you save your .typ, boom, your .pdf is ready." — [2025-09-27, 45394112](https://news.ycombinator.com/item?id=45394112)

> "love how efficient it is; it makes previewing trivial and iteration very fast." — [2025-06-22, 44351066](https://news.ycombinator.com/item?id=44351066)

### 1.12 Generate in the browser or at the edge

People want to make PDFs client-side, offline or inside a Worker, without a server. Bundle size matters there.

- **Count:** 334 comments in 235 threads; spot-check 4/10, so about 135. Per 1,000: 36.3 / 36.5 / 34.2.
- **Regex:** `client[- ]side|in[- ]the[- ]browser|in[- ]browser|browser[- ]based|\bwasm\b|webassembly|\boffline\b|no server|local[- ]first|runs? locally|jspdf|pdf-lib|pdfmake|react-pdf|typst\.ts`.

> "I've moved all my pdf to client side. Less security and processing on the server." — [2017-10-05, 15409715](https://news.ycombinator.com/item?id=15409715)

> "on cloudflare workers, the bundle limit makes 226KB massive." — [2025-12-20, 46336747](https://news.ycombinator.com/item?id=46336747)

> "the compiler can be built into a wasm component that runs locally in the browser" — [2026-05-08, 48061605](https://news.ycombinator.com/item?id=48061605)

### 1.13 Standard outputs: PDF/A, PDF/X, e-invoices

Some buyers must deliver archival PDF/A, print-ready PDF/X with CMYK, or invoices with embedded XML (ZUGFeRD, Factur-X). Each group is small, but they appear in the same threads and grew late in the decade.

- **Count:** 151 comments in 117 threads for the three combined: PDF/A and archiving 79, print production 62, e-invoicing 18. Spot-checks: 8/8, 5/8 and 4/8, so about 107 in all. By period: 34 / 39 / 78. E-invoicing alone went 1 / 3 / 14.

> "A PDF generator library that only generated guarenteed PDF/A compliant PDFs would actually be really good selling point for a new PDF library." — [2022-11-18, 33654190](https://news.ycombinator.com/item?id=33654190)

> "The invoice will be PDF with an embedded XML blob containing the machine-readable data part, signed with a PDF signature" — [2020-12-09, 25356446](https://news.ycombinator.com/item?id=25356446)

> "I tried make my PDFs X-1a compliant with WeasyPrint, then ran them through Adobe's PDF/X validator and they kept failing." — [2026-05-27, 48288349](https://news.ycombinator.com/item?id=48288349)

### 1.14 Scripts beyond Latin, and emoji

People who serve Arabic, Hebrew, Indian, CJK or Cyrillic users want text shaped correctly and fonts embedded.

- **Count:** 154 comments in 107 threads; spot-check 7/10, so about 108. Per 1,000: 13.5 / 13.4 / 17.9.
- **Regex:** `unicode|cjk|chinese|japanese|korean|arabic|hebrew|persian|devanagari|hindi|sanskrit|thai|cyrillic|emoji|right[- ]to[- ]left|\brtl\b|bidi|harfbuzz|shaping|complex script|non[- ]latin|non[- ]english`.

> "Also consider non-Western languages (CJK, Russian, Arabic). My team got bit by this when we discovered we couldn't render emoji's client-side." — [2017-10-05, 15411225](https://news.ycombinator.com/item?id=15411225)

> "The hard part of PDF generation is support for complex script (Arabic, Indian languages etc.), including embedding a font subset." — [2023-01-18, 34431292](https://news.ycombinator.com/item?id=34431292)

> "Unicode/Dēvanāgarī support out-of-the-box, no installing gigabytes of packages, near-instant compilation." — [2025-09-27, 45394163](https://news.ycombinator.com/item?id=45394163)

### 1.15 Output that stays the same and can be tested

Teams want the same input to give the same PDF across versions, so that snapshot tests catch real changes.

- **Count:** 136 comments in 100 threads; spot-check 4/10, so about 54. Per 1,000: 9.4 / 11.8 / 16.5.
- **Regex:** `determinis|reproducib|byte[- ](for[- ]byte|identical)|pixel[- ]perfect|bit[- ]perfect|same output|identical output|snapshot test|golden|regression test|visual diff|unit test|consistent (output|results|rendering)`.

> "every single release changed something in the output." — [2018-07-19, 17567576](https://news.ycombinator.com/item?id=17567576)

> "the proposed approach of rendering to an image and comparing pixels is best." — [2023-02-27, 34960757](https://news.ycombinator.com/item?id=34960757)

> "Do you have a comprehensive integration test suite that can validate the robustness of your implementation?" — [2026-03-02, 47214653](https://news.ycombinator.com/item?id=47214653)

## 2. What they complain about most

Ranked the same way as section 1. Items 2.1 and 2.2 come almost entirely from typesetting threads. In the HTML and code lane, the top complaints are licences, pagination and the weight of a headless browser.

### 2.1 Typst is not accepted yet, and has gaps

Researchers like Typst but cannot submit it, because journals and conferences ask for LaTeX sources and templates. Other complaints are missing microtypography and a new maths syntax.

- **Count:** 196 comments in 60 threads; spot-check 9/10, so about 176. Almost all fall in 2023–26 (195); only 11 are in lane A.
- **Regex:** `typst` within 150 characters of `journals?|conferences?|publishers?|arxiv|accept|not (yet|ready|mature)|immature|missing|lack|microtyp|math (mode|syntax)|backslash|ecosystem`.

> "A big con is that there are no typst templates for journals and conferences that academics submit papers to. For me, this is a show-stopper." — [2025-05-05, 43893617](https://news.ycombinator.com/item?id=43893617)

> "I'd have loved to use Typst but all journals I submit to ask for LaTeX." — [2025-08-18, 44939538](https://news.ycombinator.com/item?id=44939538)

> "Typst doesn't (yet) have one of the features that make LaTex stand out: microtypography." — [2025-05-05, 43892733](https://news.ycombinator.com/item?id=43892733)

### 2.2 LaTeX is painful to install, run and debug

The usual complaints are a multi-gigabyte distribution, multiple compile passes, package clashes and cryptic errors. The share of comments making them rose in the Typst years.

- **Count:** 229 comments in 129 threads; spot-check 9/15, so about 137. Per 1,000: 14.6 / 24.7 / 26.6.
- **Regex:** `latex|tex|pdflatex|xelatex|lualatex` within 150 characters of a pain term (`error messages`, `cryptic`, `multiple passes`, `package conflict`, `tex live … GB`, `aux files`, `arcane`, `learning curve`, `hell`, `pain`, `archaic`, `clunky`, `1980s`, `frustrat`, `nightmare`, `debugging`).

> "same issues hinder the user experience like multiple decades ago - cryptic error messages, complex pipeline, lack of the proper Unicode symbols support out of the box" — [2026-06-13, 48513549](https://news.ycombinator.com/item?id=48513549)

> "Not having to deal with the insanity of the LaTeX distribution system alone is worth the switch. Everything is contained in a single binary." — [2025-09-27, 45393964](https://news.ycombinator.com/item?id=45393964)

> "MacTeX is 4.7GB which matches the 5GB he's talking about and..." — [2022-08-20, 32535247](https://news.ycombinator.com/item?id=32535247)

### 2.3 Licences: AGPL libraries and aggressive enforcement

iText's AGPL licence, and its enforcement, come up whenever someone asks for a library. Permissive licences are a selection criterion.

- **Count:** 124 comments in 85 threads; spot-check 14/15, so about 116. Per 1,000: 15.8 / 13.9 / 11.9. 115 are in lane A.
- **Regex:** `\bagpl|\bgpl\b|licen[cs]\w*` within 80 characters of `pdf|itext|ghostscript|mupdf|prince|aspose|pspdfkit|apryse|closed[- ]source|proprietary|commercial|fonts?|librar`.

> "How have you avoided the AGPL headache that comes with almost all the open source libraries for PDF editing?" — [2022-03-17, 30710288](https://news.ycombinator.com/item?id=30710288)

> "Unfortunately the PDF generation ecosystem seems to be full of libraries that aren't permissively licensed" — [2024-01-07, 38905231](https://news.ycombinator.com/item?id=38905231)

> "They hit up a company I know because their web-crawler found a PDF that someone generated using their library over a decade ago." — [2025-02-20, 43117429](https://news.ycombinator.com/item?id=43117429)

### 2.4 Pagination from HTML is hard

Browsers implement little of CSS Paged Media, so rows split across pages, headers fail to repeat and totals get stranded. People reach for Prince or Paged.js, or give up on HTML.

- **Count:** 140 comments in 75 threads (paged media missing, or page breaks hard); spot-checks 8/10 and 5/8, so about 100. By period: 44 / 35 / 61. 124 of the 140 are in lane A.
- **Regex:** paged-media terms (`paged media|@page|margin box|target-counter|string-set|running\(|print css|@media print`) within 150 characters of a problem word, or page-break terms within 100 characters of one.

> "The holy grail of PDF generators, sensible page breaks. Never been done and makes peace in the middle east seem like an easy task." — [2017-08-22, 15071383](https://news.ycombinator.com/item?id=15071383)

> "No browser supports CSS paged media features. So there's no proper control over repeated elements, paging etc" — [2018-06-29, 17425353](https://news.ycombinator.com/item?id=17425353)

> "Invoices: Totals get pushed to a new page with no repeated <thead> header. This is a classic failure of CSS table rendering across page breaks." — [2025-08-21, 44968804](https://news.ycombinator.com/item?id=44968804)

### 2.5 A headless browser is heavy

Running Chrome per PDF costs memory, cold-start time and hundreds of megabytes, which hurts in Lambda, Azure Functions and Docker. People accept it because the output matches the browser.

- **Count:** 132 comments in 87 threads; spot-check 7/10, so about 92. Per 1,000: 20.5 / 11.3 / 12.8. 119 are in lane A.
- **Regex:** a browser word (`puppeteer|playwright|headless|chrome|chromium|electron|browserless|gotenberg|browser`) within 120 characters of a weight word (`memory`, `RAM`, `heavy`, `bloat`, `lambda`, `serverless`, `cold start`, `docker`, `leak`, `crash`, `N MB/GB`, `layer`, `overhead`, `spin up`, `whole browser`).

> "I find it pretty sad that you have to fire up a headless browser + its gigaton of code, to convert a SVG to PNG." — [2017-08-22, 15070903](https://news.ycombinator.com/item?id=15070903)

> "I had reimplemented with Puppeteer, but it’s very ram heavy for the 200-500 page PDFs I generate." — [2025-08-20, 44967658](https://news.ycombinator.com/item?id=44967658)

> "Most .NET HTML-to-PDF options are either a headless-Chromium wrapper (150-400 MB, multi-second cold starts, painful or blocked on Azure Functions Windows Consumption)" — [2026-08-27, 49465380](https://news.ycombinator.com/item?id=49465380)

### 2.6 Generated PDFs are not accessible

LaTeX output fails screen readers, Typst was criticised for not designing for accessibility, and many libraries cannot emit tags at all.

- **Count:** 127 comments in 79 threads; spot-check 7/10, so about 89. Per 1,000: 14.6 / 20.6 / 10.6.
- **Regex:** `accessib|tagged|pdf/ua|screen[- ]?readers?` within 100 characters of a problem word.

> "A LaTeX-generated PDF does not render correctly at all for a blind user." — [2020-09-11, 24445991](https://news.ycombinator.com/item?id=24445991)

> "LaTeX cannot even produce accessible PDF today. Believe me, I tried." — [2024-06-08, 40616863](https://news.ycombinator.com/item?id=40616863)

> "It's shocking how inaccessible latex's output is, and yet Typst manages to be even worse!" — [2024-02-14, 39372226](https://news.ycombinator.com/item?id=39372226)

### 2.7 Commercial engines and APIs cost too much

Prince's server licence and per-document API prices draw the sharpest reactions, especially from small teams.

- **Count:** 118 comments in 79 threads; spot-check 7/15, so about 55. Per 1,000: 18.7 / 14.9 / 9.7. 101 are in lane A.
- **Regex:** a price word (`expensive`, `costly`, `overpriced`, `pricey`, `pricing`, `$N per month/document/page`, `per (pdf|document|page)`, `licence fee`) within 80 characters of a PDF, tool or service word.

> "Prince is a 3800$ software." — [2018-04-30, 16959946](https://news.ycombinator.com/item?id=16959946)

> "It will cost $2,500 to generate 50,000 PDFs." — [2024-03-11, 39669425](https://news.ycombinator.com/item?id=39669425)

> "To avoid that pain we asked some commercial PDF generator software for a quote, but their per document pricing was insane." — [2026-06-15, 48548341](https://news.ycombinator.com/item?id=48548341)

### 2.8 Scripts and fonts break

Small and new engines often handle only ASCII or Latin, skip shaping, or drop fonts and emoji. People who need Arabic or Devanagari notice at once.

- **Count:** 87 comments in 65 threads (non-Latin terms near a problem word, or font problems); spot-checks 5/10 and 8/12, so about 50. By period: 16 / 17 / 54.

> "The Arabic is missing the joining (all forms are isolated), and the Devanagari doesn't have the vowels combining (so you see those dotted circles)." — [2026-03-02, 47214003](https://news.ycombinator.com/item?id=47214003)

> "For example, Unicode support is atrocious." — [2025-06-22, 44351083](https://news.ycombinator.com/item?id=44351083), on LaTeX

> "switched to Chromium due to emojis not rendering properly, poor flexbox support, no custom fonts" — [2024-01-17, 39028644](https://news.ycombinator.com/item?id=39028644), on WeasyPrint

### 2.9 Security and privacy of renderers and hosted APIs

URL-to-PDF services leak local files, Chrome on a server makes people nervous, and sending invoices to a third-party API raises privacy questions.

- **Count:** 120 comments in 87 threads (security terms near a renderer word, or privacy terms near an API word in lane A); spot-checks 6/12 and 5/15, so about 50. By period: 27 / 31 / 62.

> "Just try ?url=file:///etc/passwd on the demo instance." — [2017-10-05, 15409590](https://news.ycombinator.com/item?id=15409590)

> "To the extent that those PDFs could include sensitive information, there's a huge security/privacy headache there" — [2022-03-17, 30709984](https://news.ycombinator.com/item?id=30709984)

> "It makes me nervous having Chrome running on the server, even inside a container without root." — [2023-01-18, 34434343](https://news.ycombinator.com/item?id=34434343)

### 2.10 wkhtmltopdf is old and unmaintained

Most complaints come from 2017–19, when wkhtmltopdf was still the default. Later comments treat it as a legacy dependency to replace.

- **Count:** 61 comments in 36 threads; spot-check 6/8, so about 46. Per 1,000: 18.1 / 5.1 / 3.4.
- **Regex:** `wkhtml` within 200 characters of `old|outdated|ancient|abandon|unmaintained|no longer|maintain|deprecat|archived|qt ?webkit|dead|flexbox|css grid|buggy|bugs?|security|vulnerab|stale|years`.

> "wkhtmltopdf development has slowed a lot, it is very unstable and you need to run a 2 year old alpha version to support flexbox" — [2017-08-22, 15071434](https://news.ycombinator.com/item?id=15071434)

> "I've used wkhtmltopdf heavily for years and it is a disaster today." — [2018-07-19, 17568106](https://news.ycombinator.com/item?id=17568106)

> "I previously used WKHTMLTOPDF, but it hasn't been supported for years and doesn't support the latest CSS, etc." — [2025-10-01, 45440007](https://news.ycombinator.com/item?id=45440007)

### 2.11 Slow engines

WeasyPrint, Chrome and LaTeX all draw complaints about speed. The rate nearly doubled in 2023–26.

- **Count:** 61 comments in 46 threads; spot-check 8/12, so about 41. Per 1,000: 4.1 / 4.1 / 7.8.
- **Regex:** `slow|takes forever|sluggish` within 60 characters of an engine or compile word.

> "I love it but it's bloody slow." — [2018-06-29, 17426781](https://news.ycombinator.com/item?id=17426781), on WeasyPrint

> "Weasyprint was much slower than headless Chrome and also required a lot of memory to process the HTML." — [2024-03-22, 39789102](https://news.ycombinator.com/item?id=39789102)

> "LaTeX comping/rendering is generally extremely slow." — [2025-09-27, 45394688](https://news.ycombinator.com/item?id=45394688)

**Weak themes.** These complaints exist but scored low on spot-checks:

- output that changes across versions: 68 matches, 3/8 on topic;
- file size: 51 matches, 3/8;
- text that cannot be selected or copied: 30 matches, 3/8.

The text-selection problem drew a 2026 thread of its own: "The surprisingly complex journey to text-selectable client-side generated PDFs", [48045778](https://news.ycombinator.com/item?id=48045778), 86 points.

## 3. What excites them

Typst drew more attention than any other generation tool. HTML-to-PDF tools came next, and from 2024 on, engines that skip the browser.

**Stories by family.** Counts come from story titles (`generation_stories.py`):

| family | stories 2016–19 / 2020–22 / 2023–26 | points 2016–19 / 2020–22 / 2023–26 |
|---|---|---|
| Typst | 0 / 2 / 130 | 0 / 79 / 6,532 |
| LaTeX/TeX | 8 / 8 / 59 | 401 / 381 / 2,816 |
| HTML/CSS to PDF | 41 / 28 / 64 | 2,331 / 960 / 1,248 |
| code libraries | 18 / 19 / 65 | 758 / 147 / 1,295 |

**Typst.** Each release and each "I switched" post reached the front page:

- [Typst, a new markup-based typesetting system, is now open source](https://news.ycombinator.com/item?id=35250210): 2023-03-21, 520 points.
- [Typst: An easy to learn alternative for LaTex](https://news.ycombinator.com/item?id=41014941): 2024-07-20, 649 points.
- [Exploring Typst, a new typesetting system similar to LaTeX](https://news.ycombinator.com/item?id=41821361): 2024-10-12, 593 points.
- [I wrote my PhD Thesis in Typst](https://news.ycombinator.com/item?id=44350322): 2025-06-22, 532 points, 328 comments.
- [Typst: A Possible LaTeX Replacement](https://news.ycombinator.com/item?id=45393842): 2025-09-27, 786 points, 384 comments: the top generation story of the decade.
- [Typst 0.14](https://news.ycombinator.com/item?id=45693978) (2025-10-24, 653 points) and [Typst 0.15.0](https://news.ycombinator.com/item?id=48544396) (2026-06-15, 336 points).
- [Gribouille 0.3.0: A Grammar of Graphics for Typst](https://news.ycombinator.com/item?id=48541062): 2026-06-15, 215 points.

**HTML and CSS to PDF.** The attention moved from browser wrappers to paged-media tools:

- [Show HN: Pdf-Bot, an API/CLI for Generating PDFs Using Headless Chrome](https://news.ycombinator.com/item?id=15070686): 2017-08-22, 235 points.
- [Show HN: ReLaXed – High-quality PDFs using web technologies](https://news.ycombinator.com/item?id=16959550): 2018-04-30, 344 points.
- [Paged.js: Paginating content and making books in the browser](https://news.ycombinator.com/item?id=21499052): 2019-11-10, 179 points.
- [Show HN: PDFs from HTML](https://news.ycombinator.com/item?id=26691626): 2021-04-04, 742 points, a Paged.js tutorial and the top HTML-to-PDF story.
- [Show HN: Htmldocs](https://news.ycombinator.com/item?id=39027543) (2024-01-17, 226 points) and [Launch HN: Onedoc (YC W24)](https://news.ycombinator.com/item?id=39668962) (2024-03-11, 293 points). Onedoc built on Prince rather than a browser, for typesetting quality.
- [Show HN: PlutoPrint](https://news.ycombinator.com/item?id=44966170): 2025-08-20, 161 points, a C++ HTML engine with no browser.
- [Ask HN: What is nowadays (opensource) way of converting HTML to PDF?](https://news.ycombinator.com/item?id=45404760): 2025-09-28, 176 points, 165 comments.

**Small libraries and new engines:**

- [Gofpdf](https://news.ycombinator.com/item?id=15767878): 2017-11-23, 168 points.
- [OpenPDF](https://news.ycombinator.com/item?id=17416671): 2018-06-28, 213 points, the LGPL/MPL fork of iText.
- [QuestPDF](https://news.ycombinator.com/item?id=34429791): 2023-01-18, 195 points.
- [Show HN: TinyPDF – 3kb pdf library](https://news.ycombinator.com/item?id=46316968): 2025-12-18, 253 points. Commenters found it ASCII-only.
- [Show HN: a zero-browser, pure-JS typesetting engine](https://news.ycombinator.com/item?id=47206082): 2026-03-01, 78 points. Commenters found broken Arabic and Devanagari, and complained about AI-written text.
- [PaperPilot – Zero-Cloud, Pure-Rust PDF Engine with WASM and MCP](https://news.ycombinator.com/item?id=50238739): 2026-10-26, 236 points. Its pitch (Rust, WASM, MCP) is the closest to Lamina PDF's. Commenters liked the WASM sandbox, asked about licence provenance and pointed to existing Rust crates.

**Apps and documents:**

- [Show HN: Invoice Dragon](https://news.ycombinator.com/item?id=36860898): 2023-07-25, 564 points.
- [The Overengineered Resume with Zola, JSON Resume, Weasyprint, and Nix](https://news.ycombinator.com/item?id=40934582): 2024-07-11, 182 points.
- [Show HN: RenderCV – YAML to PDF](https://news.ycombinator.com/item?id=46344616): 2025-12-21, 100 points.
- [Making a PDF that's larger than Germany](https://news.ycombinator.com/item?id=39210507): 2024-01-31, 712 points, a curiosity about page-size limits.
- The "Make your PDF look scanned" tools: [2020-05-12, 607 points](https://news.ycombinator.com/item?id=23157408), [2022-04-19, 579](https://news.ycombinator.com/item?id=31083432), [2026-06-20, 155](https://news.ycombinator.com/item?id=48611513).

**Accessibility:**

- [LaTeX is the first PDF/UA-2 compliance accessible PDF producer](https://news.ycombinator.com/item?id=40587934): 2024-06-05, 219 points.
- [How to create accessible PDFs from the start](https://news.ycombinator.com/item?id=45781725): 2025-11-01, 106 points.

**Browser-side typesetting:** [TeXbrain, pdfTeX in the browser via WASM](https://news.ycombinator.com/item?id=49441375) (2026-08-25, 118 points) and [a graph paper generator that renders vector PDFs in the browser](https://news.ycombinator.com/item?id=48761294) (2026-07-02, 107 points).

## 4. Tools and sentiment over time

Opinion moved most on wkhtmltopdf (down), Typst (up from nothing) and WeasyPrint (up after a rough 2020–22). Headless Chrome and Prince held steady: Chrome tolerated, Prince respected but priced out.

In the table, mentions count core comments that name each tool. "Net cue" is the share of those comments with a positive word (love, great, works well, reliable, recommend, breath of fresh air, …) minus the share with a negative word (pain, buggy, broken, outdated, slow, bloat, leak, nightmare, …), in points. A dash means fewer than 15 mentions. The lexicon is crude, so read the net cue as a direction, not a score.

| tool | mentions 2016–19 / 2020–22 / 2023–26 | net cue 2016–19 / 2020–22 / 2023–26 |
|---|---|---|
| LaTeX/TeX | 548 / 638 / 1,850 | +9 / +4 / +5 |
| Typst | 0 / 24 / 1,640 | – / +4 / +10 |
| Pandoc | 134 / 162 / 382 | +18 / +10 / +6 |
| Headless Chrome/Puppeteer/Playwright | 125 / 82 / 197 | +8 / −6 / +10 |
| wkhtmltopdf | 136 / 55 / 81 | +10 / 0 / −1 |
| WeasyPrint | 39 / 40 / 107 | +28 / +2 / +19 |
| Prince/DocRaptor | 59 / 34 / 45 | +19 / +21 / +22 |
| Paged.js | 9 / 42 / 36 | – / +21 / +31 |
| iText/OpenPDF | 35 / 42 / 76 | +9 / −2 / +12 |
| jsPDF/pdf-lib/pdfmake | 11 / 18 / 48 | – / +6 / +12 |
| Quarto | 0 / 4 / 82 | – / – / +13 |
| Gotenberg | 0 / 1 / 17 | – / – / +12 |

**Typst overtook LaTeX.** Per year in the core set, Typst passed LaTeX in 2025 (679 against 667 mentions) and led in 2026 (411 against 305). Typst's positive-cue share rose from 12% to 22%, while LaTeX's fell from 31% to 24%. Many LaTeX defenders remain:

> "LaTeX typesetting is a solved problem." — [2025-06-22, 44351138](https://news.ycombinator.com/item?id=44351138)

> "At work we recently switched from Wkhtmltopdf to Typst, which is a breath of fresh air." — [2025-05-18, 44025100](https://news.ycombinator.com/item?id=44025100)

**wkhtmltopdf went from default to legacy.** In 2017–19 it was the most-named HTML engine (136 mentions), and users defended it against the "considered harmful" post. By 2023–26 its negative-cue share had reached 25%, and commenters called it unsupported (quote in 2.10).

> "wkhtmltopdf has been working fine in production for many months, reliably producing thousands of 10+ page PDFs with cover pages, headers, footers, page numbering, and images." — [2018-07-19, 17567514](https://news.ycombinator.com/item?id=17567514)

**Headless Chrome became the default but not a favourite.** Mentions passed wkhtmltopdf's by 2023–26 (197 against 81). Its net cue dipped to −6 in 2020–22, when 28% of its mentions carried a negative word, then recovered.

> "Yep, this is the way to go now." — [2021-03-26, 26590830](https://news.ycombinator.com/item?id=26590830)

> "Headless Chromium is the way. Bloated maybe, but does the job." — [2023-05-06, 35844143](https://news.ycombinator.com/item?id=35844143)

**WeasyPrint grew, and Prince kept its reputation.** WeasyPrint mentions nearly tripled in 2023–26, praised as a set-and-forget engine and faulted for speed (quotes in 2.11). Prince had the steadiest positive net cue (+19 to +22), and its price was the main objection (2.7).

> "I have used weasyprint with a django production system for a few years now, and it works well enough that I never have to think about it." — [2025-10-01, 45439904](https://news.ycombinator.com/item?id=45439904)

> "I’ve had excellent experience with Prince XML and poor experience with everything else I’ve tried. Prince is fast, robust and reliable." — [2025-10-01, 45440728](https://news.ycombinator.com/item?id=45440728)

**Paged.js and Gotenberg are the open-source answers people recommend.** Paged.js peaked in 2021 with the "PDFs from HTML" thread. Gotenberg appears only from 2022 on, and the comments I read praised its reliability.

> "FYI: the open source state of the art in this area is Playwright (the successor to Puppeteer) with Paged.js" — [2024-03-11, 39670314](https://news.ycombinator.com/item?id=39670314)

> "gotenberg is really rock solid for us." — [2025-10-01, 45439192](https://news.ycombinator.com/item?id=45439192)

**Code libraries split by licence and size.** iText draws licence anger (2.3), and Java users moved to OpenPDF or PDFBox. jsPDF, pdf-lib and pdfmake mentions rose late (25 in 2026 alone), with complaints about size and layout effort.

> "we've all abandoned itext for the fork" — [2025-02-20, 43116723](https://news.ycombinator.com/item?id=43116723)

> "Something that has surprised me is how hard it is to keep the file size small with tools like this." — [2024-01-17, 39029434](https://news.ycombinator.com/item?id=39029434), on jsPDF

**Pandoc is the glue, now often in front of Typst.** Pandoc mentions nearly tripled, but its net cue fell from +18 to +6 as people met its limits. Many 2025–26 comments describe Markdown → Pandoc → Typst pipelines (quote 43232394 in 1.1).

## 5. How the decade changed

- **The browser replaced wkhtmltopdf in 2017–18.** wkhtmltopdf had 62 and 50 core mentions in 2017 and 2018. Headless Chrome tools had 55 and 60 in those years and 197 against wkhtmltopdf's 81 in 2023–26.
- **From 2024, people looked for engines without a browser.** Examples are WeasyPrint's growth, Onedoc on Prince, PlutoBook, the pure-JS and pure-Rust Show HNs and Typst used as a back-end renderer. Typst-in-production comments went from 0 before 2023 to 94 in 2023–26.
- **LaTeX gave way to Typst.** Typst went from 0 to 1,640 core mentions per period and passed LaTeX in 2025. Typst stories scored 6,532 points in 2023–26 against 2,816 for LaTeX stories.
- **AI entered the workflow.** AI and agent terms near a generation term rose from 6 to 11 to 263 comments (3.5 → 5.7 → 44.8 per 1,000; spot-check 6/10). The 2026 threads cover a "markup language for AI" (Papermill), MCP servers and LLMs writing Typst. Show HN readers also pushed back on AI-written READMEs.

  > "Frontier LLMs work great with Typst." — [2026-06-15, 48548712](https://news.ycombinator.com/item?id=48548712)

  > "I made it because Claude makes really bad PDFs so I made an editor to fix them." — [2026-05-17, 48164944](https://news.ycombinator.com/item?id=48164944)

- **Compliance grew from a footnote to a requirement.** PDF/UA or tagged PDF named outright went from 5 to 9 to 36 comments, e-invoicing from 1 to 3 to 14, and accessibility law and government markets appear in 2024–25 comments (1.8).
- **"Just print it from the browser" faded.** Mentions of the browser's print-to-PDF fell from 28.7 to 16.5 to 13.6 per 1,000 core comments (spot-check 5/10), as generation moved to servers, pipelines and agents.

## 6. What this means for a PDF generation library built today

1. **Make multi-page flow the core feature.** The want with the clearest gap is pagination: page breaks with keep-together rules, running headers and footers, "page X of Y", tables that split with repeated headers, footnotes and a TOC with page numbers (1.6: about 345 comments; 2.4: about 100). Browsers fail here, and Prince and Typst win here. Lamina's builder lays text into a box on one page and reports what overflows (or fails with `text-overflow`), so a flow layer on top of `Surface` would meet the most common unmet need.
2. **Be the engine that fits in a function or a browser tab.** Headless Chrome's weight is the top HTML-lane complaint (2.5), speed is a growing want (1.5: 53.9 → 67.5 per 1,000), and edge bundle limits matter (46336747). Publish numbers people already quote: milliseconds per two-page invoice (Typst users cite under 100 ms, 45697377), WASM module size and cold-start time.
3. **Lead with correct shaping for complex scripts.** New engines get caught on Arabic and Devanagari within hours of launch (2.8, 47214003), and TinyPDF was found ASCII-only. Lamina already shapes with HarfRust and handles bidi, so its examples and docs should show Arabic, Hebrew, Devanagari and CJK on page one.
4. **Make tagged, accessible output the default.** Buyers need it by law (1.8), LaTeX and Typst are criticised for lacking it (2.6), and a commenter on the QuestPDF launch found tags missing (34431828). Lamina's PDF/UA-1 profile and tagging diagnostics answer this complaint, so tagging on by default and a clear "why this failed" message would turn a niche feature into a reason to choose Lamina.
5. **Ship standard outputs as recipes.** PDF/A, PDF/X and e-invoices are small but rising (1.13: 34 → 39 → 78 by period), and one commenter named guaranteed PDF/A as a selling point (33654190). Lamina has PDF/A-2b/2u/3b and attachments with an AF relationship, which is what Factur-X and ZUGFeRD need. A tested ZUGFeRD/Factur-X invoice example would meet a regulated need with no competitor on HN offering it out of the box.
6. **State the licence loudly.** AGPL fear decides library choice (2.3: about 116 comments, 14/15 on topic), and Prince's price decides against it (2.7). MIT OR Apache-2.0 belongs on the front page and in every "compared with" table, next to iText's AGPL and Prince's commercial licence.
7. **Sell byte-identical output and test tooling.** Teams complain that upgrades change output (17567576) and build pixel-diff tests by hand (34960757), and the deterministic-output want rose from 9.4 to 16.5 per 1,000. Lamina's output is byte-deterministic, so a snapshot-testing guide for users' own templates would turn an internal property into a feature people can use.
8. **Accept data, Markdown and agents as inputs.** The two largest wants are "write Markdown" and "fill a template from JSON" (1.1, 1.2), and Typst wins business users by reading JSON directly (39672973, 45394409). AI-near-generation talk grew from 6 to 263 comments, so a declarative document format that an LLM can write, plus a Markdown front end, would meet both. Lamina's MCP server is read-only by default (ADR 0031), so a generation tool there needs its own ADR.

## 7. Method and limits

**What I read.** I read about 1,150 comments, each to its first 300–420 characters, from 40 threads spread across the decade:

- 2017–19: Pdf-Bot, URL to PDF Microservice, the headless-Chrome API, Gofpdf, ReLaXed, ThePDFApi, BreezyPDF, "Wkhtmltopdf Considered Harmful", Paged.js and three Ask HNs;
- 2020–22: LaTeX accessibility, "Wkhtmltopdf", "Show HN: PDFs from HTML", PDF API and Lago invoices;
- 2023–26: QuestPDF, Unit Testing PDF Generation, Ask HN back-end PDFs, Invoice Dragon, Typst open source, Htmldocs, Onedoc, LaTeX PDF/UA-2, the Overengineered Resume, iText at 25, Typst: A Possible LaTeX Replacement, the PhD thesis in Typst, PlutoPrint, Ask HN HTML to PDF 2025, Markdown-to-PDF tools, the permissive C# library quest, TinyPDF, the zero-browser JS engine, client-side text-selectable PDFs, Papermill, Rubrol and PaperPilot.

About 450 more short excerpts came from the spot-checks. I used `generation_read_thread.py` for the threads and the `--sample` options of the count scripts for the excerpts.

**What I counted.** The core set has 9,523 comments.

- **Sources:** comments in `areas/generation.jsonl` that pass lane A (an HTML-to-PDF tool, a generation library or a "generate/create/render a PDF", invoice or paged-media phrase) or lane B (a typesetting system plus a document cue). I added every other comment from threads whose titles are about generation (2,932 comments), with titles about OCR, extraction or parsing left out.
- **Exclusions:** I dropped exact duplicate texts and "Who wants to be hired" and freelancer threads.
- **Themes:** each theme is a case-insensitive regex. Some need a second regex within N characters of the first (`generation_counts.py`), and a comment counts once per theme.
- **Spot-checks:** for each theme I hand-read 8 to 15 random matches and recorded how many were on topic. The "about N" estimates multiply matches by that share, and items are ranked by the estimates. With samples this small the shares are rough (±20 points), so read the ranking in bands, not as exact positions.
- **Sentiment:** I counted positive and negative words in comments that name each tool. The lexicon misses sarcasm and licence or price complaints.

**Limits:**

- **Developer skew.** HN commenters are developers who generate PDFs from code, so designers, publishers and office users are underrepresented.
- **Typst mix.** Typst threads dominate 2023–26 (lane B rose from 40% to 64% of the core), which lowers per-1,000 rates for HTML-lane topics in that period. The lane A columns in `tools/hn-research/counts/generation/output.txt` correct for part of this.
- **Area filter.** The area file was built with a broad regex (`generate`, `latex`, `template`, …). It holds noise, such as LLM "generate" talk and LaTeX as notation, and it misses comments in non-generation threads that use other words. The lane rules and spot-checks reduce the noise but do not remove it.
- **Thread size.** A few large threads carry many counts (for example, 60 threads hold all 196 Typst-gap comments), so I give thread counts beside comment counts.
- **Data window.** The data ends in early November 2026, so 2026 is a partial year.
