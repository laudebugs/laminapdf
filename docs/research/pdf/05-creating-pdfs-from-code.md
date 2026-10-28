# 05 — Creating, Processing & Rendering PDFs from Code: Approaches and Ecosystem

> Part of the lamina PDF research series (`docs/research/pdf/`). Earlier chapters cover the file syntax (01), the graphics model (02), text and fonts (03), and interactivity and security (04). This chapter covers **how software produces, changes and displays PDFs**: which approaches exist, which libraries implement them, how common processing operations work inside, how rendering engines compare, and how to test all of it. It ends with a recommendation for lamina's own architecture, for the team to discuss.
>
> **Status snapshot date: 2026-10-28.** Versions and dates in the survey tables come from the package registries (npm, PyPI, crates.io, Maven Central, NuGet, the Go module proxy) on that date, unless marked *(unverified)*. GitHub and several project sites could not be reached from the research sandbox, so a few native C/C++ tools carry versions from distribution trackers or search results. Those are marked as such.

---

## Contents

1. [The generation spectrum](#1-the-generation-spectrum)
2. [Ecosystem survey by language](#2-ecosystem-survey-by-language)
3. [Processing operations under the hood](#3-processing-operations-under-the-hood)
4. [Rendering engines compared](#4-rendering-engines-compared)
5. [Performance and scale](#5-performance-and-scale)
6. [Testing and validation](#6-testing-and-validation)
7. [A verified tiny end-to-end demo](#7-a-verified-tiny-end-to-end-demo)
8. [Implications for lamina](#8-implications-for-lamina)
9. [References](#9-references)

---

## 1. The generation spectrum

Every PDF producer eventually does the same thing. It emits a graph of COS objects (ch. 01), and in that graph each page's content stream holds path, text and image operators (ch. 02/03). Producers differ in **how much of the work between "what the author means" and "operators at absolute coordinates" the tool does for you**.

```
 abstraction ▲
             │  7. Office / print pipelines   (DOCX, ODT, OS print → PDF)
             │  6. Declarative templates      (JSON / JSX / fluent DSL → layout)
             │  5. Typesetting systems        (LaTeX, Typst, ConTeXt, SILE)
             │  4. HTML/CSS engines           (Chromium, WeasyPrint, Prince)
             │  3. Layout-engine libraries    (flowables, tables, pagination)
             │  2. Drawing-API libraries      (moveTo/lineTo/text at x,y)
             │  1. Object writers             (dicts, streams, xref)
             └──────────────────────────────────────────────────────────────►
                control over bytes / PDF features      ◄── ease / speed of authoring
```

Each layer can be built on the one below it. In practice the stacks look like this:

```
@react-pdf/renderer ──► own layout (Yoga flexbox) ──► fork of PDFKit (drawing) ──► object writer
pdfmake             ──► own layout                ──► PDFKit
Typst               ──► own layout                ──► krilla (high-level)    ──► pdf-writer (objects)
ReportLab PLATYPUS  ──► ReportLab canvas (drawing) ──► ReportLab pdfbase (objects)
Chromium page.pdf() ──► Blink layout (print mode)  ──► Skia SkPDF backend
WeasyPrint          ──► own CSS layout              ──► pydyf (objects)
QuestPDF            ──► own layout (fluent)         ──► Skia (SkPDF via native lib)
iText layout        ──► iText kernel (canvas + objects)
```

A useful way to judge any approach is to look at what it gets **for free** and what it can **never** do well. The sections below use these headings.

### 1.1 Low-level object writers

You (or a thin library) emit indirect objects, dictionaries and streams directly. You write content-stream operators yourself (`re`, `S`, `BT … Tj … ET`) and serialise a cross-reference table or stream. Section 7 has a complete 41-line Python example.

| Gets for free | Must do yourself |
|---|---|
| Byte-exact control: object streams, incremental updates, exact `/StructTreeRoot`, custom keys, any PDF 2.0 feature | Font embedding: subsetting, `/W` widths, `/ToUnicode` CMaps (ch. 03); text shaping (HarfBuzz-class work); line breaking; image encoding; colour management; pagination |
| Streaming output: write objects as they are produced and keep only the xref offsets | Correctness: the Arlington model has 500+ object types and thousands of key rules (§6) |

**Representative libraries:** `pdf-writer` (Rust, typed builder with no read support, used by Typst), `lopdf` (Rust, read and write object graph), qpdf's `QPDFWriter` (C++), PoDoFo (C++), `pikepdf` (Python over qpdf), PDFBox `COSWriter` (Java), and the iText `kernel` module.

**When to choose this:** when you *are* building a library (lamina's case), when you generate machine-produced documents with a fixed shape at very high volume, or when you need exact output, e.g. a specific PDF/A or PDF/UA structure.

### 1.2 Drawing-API libraries (canvas-like)

These expose an imperative 2D API: `moveTo/lineTo/fill`, `drawImage`, `text(x, y, str)`. Fonts, images and object bookkeeping are handled for you. The API maps almost 1:1 onto PDF operators. HTML Canvas, Cairo, Skia, Core Graphics and Java2D share this mental model, which is why the same code can often target screen and PDF.

```
 app code ── draw calls ──► graphics state stack ──► content-stream encoder ──► page objects
                                 │                        ▲
                                 └── font manager ────────┘ (embed + subset at save())
                                 └── image manager ─── (dedupe, encode DCT/Flate)
```

**Gets for free:** font embedding and subsetting (in the good ones), image XObjects, resource dictionaries, coordinate helpers. Many libraries flip the y-axis to a top-left origin; fpdf2 does, but pdf-lib keeps PDF's bottom-left origin, which surprises newcomers.

**Never does well:** text flow. At most you get "wrap this string into a box" (`multi_cell`, PDFKit's `text(…, {width})`). There are no tables that break across pages, no widows or orphans, no "page X of Y". Complex scripts (Arabic, Indic) need real shaping. pdf-lib, jsPDF and most "fpdf"-family libraries do **not** shape with HarfBuzz. PDFKit uses fontkit, which does OpenType layout. fpdf2 gained optional HarfBuzz shaping (`set_text_shaping`). Most do not emit tagged PDF.

**Representative libraries:** PDFKit, jsPDF, pdf-lib (Node/browser); ReportLab `canvas`, fpdf2 (Python); Cairo, Skia, libHaru (C/C++); `printpdf` (Rust); PDFsharp `XGraphics` (.NET); Core Graphics (Apple); gofpdf/go-pdf/fpdf, gopdf (Go).

### 1.3 Document / layout-engine libraries

These add a **box model and a pagination loop** on top of a drawing API. Content is a sequence of *flowables* (paragraphs, tables, images, spacers, keep-together groups). The engine measures each one, splits it if it overflows, and places it into frames on page templates.

```
 story: [Heading, Para, Table(300 rows), Image, Para]
          │
          ▼
  ┌──────────────┐  wrap(availW, availH) → (w,h)     ┌───────────────┐
  │ layout loop  │──────────────────────────────────►│  flowable      │
  │ (per frame)  │◄── fits? draw : split(availH) ────│  (measure/split)│
  └──────┬───────┘                                   └───────────────┘
         │ page full → onPage hooks (header/footer) → new page from template
         ▼
  drawing API ─► objects ─► file
```

The hard problems are much the same in every engine:

- **Line breaking.** Greedy first-fit (most libraries) versus Knuth–Plass total-fit (TeX, Typst, SILE). Hyphenation uses Liang patterns.
- **Splitting.** Tables repeat their header rows on each page. Rows can be allowed to split or not. Paragraphs need widow and orphan control. "Keep with next" applies to headings.
- **Forward references.** "Page 3 of **N**" and table-of-contents page numbers are unknown until layout finishes. There are two standard tricks: (a) **two-pass layout**, used by ReportLab's `multiBuild` and LaTeX's `.aux` files; (b) **deferred Form XObject**: draw a reference to an empty XObject on every page, then fill in its stream once the total is known. iText's and PDFKit-family examples use (b), and it costs nothing at runtime because the XObject is written last.
- **Tagging.** A layout engine *knows* the semantic structure (heading, paragraph, table cell), so it is the natural place to emit `/StructTreeRoot` and marked content (ch. 04). iText, QuestPDF (2025+), Typst/krilla and Prince do this. Most JS layout libraries do not.

**Representative libraries:** ReportLab PLATYPUS, borb (Python); iText `layout`, OpenPDF, JasperReports (JVM); QuestPDF, MigraDoc (.NET); pdfmake, @react-pdf/renderer (JS); `genpdf` (Rust, stale since 2021); maroto (Go). These have to reinvent CSS-like layout, and each does it partially.

### 1.4 HTML/CSS → PDF

Render a web document with a browser-class layout engine in *paged media* mode, then serialise the painted output as PDF. The appeal is clear: web teams already know HTML/CSS, design systems can be reused, and fonts, SVG, flexbox and grid come for free in Chromium.

```
             ┌────────────────────── Chromium (headless) ─────────────────────┐
 HTML+CSS ──►│ DOM/CSSOM → style → layout (paginated "print" mode) → paint   │──► PDF
 + JS        │   @page size/margins, break-*, margin boxes (Chrome ≥131)     │
             │   paint ops recorded → Skia SkPDF backend (fonts subset,      │
             │   tagged if requested, links→/Link annots, outline optional)  │
             └───────────────────────────────────────────────────────────────┘
 Puppeteer / Playwright drive it over the DevTools protocol: Page.printToPDF
```

**Engines in 2026:**

| Engine | Kind | License | Paged-media depth | JS | Tagged / PDF/UA | PDF/A | Notes |
|---|---|---|---|---|---|---|---|
| **Chromium** via Puppeteer 25.16 / Playwright 1.64 | Browser | BSD / Apache-2.0 | `@page` size/margins, `break-*`, page margin boxes with `counter(page)`/`counter(pages)` (Chrome 131+). **No** `string-set`/`running()`, `target-counter()`, footnotes or page floats | Full | Tagged by default (Puppeteer `tagged: true`, marked experimental); not PDF/UA-validated by default | No | Header and footer via `headerTemplate`/`footerTemplate` (isolated mini-documents with `.pageNumber`, `.totalPages`). `outline` (bookmarks) is opt-in and experimental |
| **Gotenberg** 8.39.0 (2026-10-26) | Docker API over Chromium + LibreOffice + PDF tools | MIT | = Chromium | Full | = Chromium | Via LibreOffice / post-processing | Batteries-included microservice: merge, split, metadata, webhooks |
| **WeasyPrint** 70.0 (2026-09-08) | Python, own layout engine | BSD-3 | Strong: margin boxes, `string-set`, `target-counter`, footnotes, bookmarks via `bookmark-level` | **None** | PDF/UA-1 (and UA-2 in recent manpages) via `--pdf-variant` | PDF/A-1…4 variants (marked experimental) | No JS is a feature for security, but it rules out client-rendered charts. Text via Pango/HarfBuzz; objects via `pydyf` |
| **Prince** 16.2 (Jan 2026) | Commercial, own engine | Proprietary (free with watermark for non-commercial use) | The reference implementation: everything above plus page floats, `prince-*` extensions | Limited | PDF/UA-1, PDF/A-1a/3a+UA-1 combos | Yes | Used by publishers and DocRaptor (SaaS) |
| **Vivliostyle** (core 2.45.2, CLI 11.4.0) | JS paged-media engine that runs *inside* Chromium | **AGPL-3.0** | Strong (CSS GCPM, footnotes, `target-counter`) | Full | = Chromium | No | ⚠ AGPL, see §8 |
| **Paged.js** 0.4.3 (last npm release 2023-07) | JS polyfill inside Chromium | MIT | Strong polyfill | Full | = Chromium | No | Stagnant on npm; uses the Chromium print path for output |
| **Flying Saucer** 10.7.0 / **openhtmltopdf** 1.1.103 (community fork `io.github.openhtmltopdf`, 2026-10-27) | Java, XHTML + CSS 2.1 + paged media | LGPL-2.1 | Good (running elements, page counters) | None | openhtmltopdf: PDF/UA | openhtmltopdf: PDF/A | Strict XHTML input; uses PDFBox (openhtmltopdf) or OpenPDF (Flying Saucer) |
| **iText pdfHTML** | Java/.NET add-on | **AGPL** / commercial | Good | None | Yes | Yes | Inherits iText licensing |
| **xhtml2pdf** 0.2.28 | Python on ReportLab | Apache-2.0 | Weak (CSS 2.1 subset) | None | No | No | Legacy Django use |

**Strengths:** designer productivity; the full web platform in Chromium (SVG, web fonts, grid, flexbox, JS charts); good typography for Latin and CJK; hyperlinks. Recent Chromium versions emit tagged structure from the accessibility tree.

**Weaknesses:**

- **Cost.** A headless Chromium is a ~150–300 MB RSS process tree, with cold start in the seconds and per-render CPU much higher than a native library (§5). Serverless setups need special builds (e.g. `@sparticuz/chromium` 153.0.0).
- **Fidelity drift.** Output changes with the Chromium version, fonts installed in the container, and timing (a page that is not "ready" when printed). Pin the browser and fonts, and wait on `document.fonts.ready` plus an explicit app-ready signal.
- **Paged-media gaps in Chromium.** No running headers taken from content, no cross-references, no footnotes, no `float: page-top`. People bolt on Paged.js or Vivliostyle, or switch to WeasyPrint or Prince.
- **Archival and compliance.** Chromium has no PDF/A output. Its PDF/UA conformance is not guaranteed even with tagging on. Teams post-process with Ghostscript, LibreOffice or a commercial converter, and that can break the tags.
- **Security.** Rendering user-controlled HTML is an SSRF and local-file-read risk (`file://`, internal URLs), so you need network allow-lists and sandboxing.

### 1.5 Typesetting systems

These are programming languages for documents, with research-grade line breaking, mathematics and cross-referencing.

| System | Engine/output path | Notes (2026) |
|---|---|---|
| **pdfTeX** | TeX → PDF directly | 8-bit fonts; the default for classic LaTeX; very stable |
| **XeTeX** | TeX → XDV → `xdvipdfmx` | System OpenType fonts, HarfBuzz shaping; maintenance-mode feel |
| **LuaTeX / LuaLaTeX** | TeX + embedded Lua → PDF | OpenType via `luaotfload`; the engine the **LaTeX Tagging Project** targets for tagged PDF / PDF/UA-2 |
| **ConTeXt LMTX** | LuaMetaTeX | Strong tagging and PDF/A heritage; smaller community |
| **Typst** 0.15.1 (crates.io, 2026-07-17) | Rust; incremental compilation; PDF via **krilla** → **pdf-writer** | Since 0.14 output is **tagged by default** and PDF/UA-1 export is supported. 0.15 can target multiple standards at once (e.g. PDF/A-2a + PDF/UA-1). Compiles in milliseconds to seconds, runs in WASM, Apache-2.0. Also usable as a library or CLI, or from JS via `typst.ts` |
| **SILE** | Lua; Knuth–Plass and more | Niche but innovative (grid typesetting, bidi) |
| **Pandoc** | Markdown/DOCX/… → `--pdf-engine=` {pdflatex, lualatex, xelatex, typst, weasyprint, prince, wkhtmltopdf, context…} | A **router**: one AST, many PDF backends |

**Trade-offs:** best typographic quality and math; deterministic, text-based sources that diff well in git. TeX is slow, needs a huge install (TeX Live is several GB) and gives poor error messages. Typst fixes most of this, and in 2026 it is the most relevant typesetting system for a new platform, both as a generation backend and as an example of a Rust PDF stack (§8).

### 1.6 Template / declarative approaches and office conversion

The document is *data*: a JSON tree, a JSX element tree or a fluent builder chain. An interpreter hands it to a layout engine.

```js
// pdfmake: JSON document definition
{ content: [ { text: 'Invoice', style: 'h1' },
             { table: { headerRows: 1, body: [['Item','Qty'],['Widget','2']] } } ],
  footer: (page, pages) => ({ text: `${page} / ${pages}`, alignment: 'right' }) }
```
```jsx
// @react-pdf/renderer: React reconciler → Yoga flexbox layout → PDFKit fork
<Document><Page size="A4" style={{ padding: 40 }}>
  <Text style={{ fontSize: 24 }}>Invoice</Text>
  <View style={{ flexDirection: 'row' }}>…</View>
</Page></Document>
```
```csharp
// QuestPDF: fluent C#
Document.Create(c => c.Page(p => {
    p.Margin(40);
    p.Header().Text("Invoice").FontSize(24);
    p.Content().Table(t => { /* … */ });
    p.Footer().AlignRight().Text(x => { x.CurrentPageNumber(); x.Span(" / "); x.TotalPages(); });
})).GeneratePdf("invoice.pdf");
```

- **pdfme** (6.2.8, MIT) is a WYSIWYG *template designer* plus a JSON schema plus a generator built on pdf-lib. Fields are positioned absolutely over a base PDF, which suits forms, labels and certificates.
- **Template + office converters:** fill a DOCX/ODT template (docxtemplater, Carbone, python-docx-template), then convert with **LibreOffice headless** (`soffice --headless --convert-to pdf`) or Gotenberg. LibreOffice can export tagged PDF and PDF/A-1/2/3 through filter options. Pitfalls: Word-layout fidelity depends on fonts, so ship the metric-compatible fonts (Carlito, Caladea, Liberation). Each `soffice` process locks its user profile, so pool processes with distinct `-env:UserInstallation=` directories, or use `unoserver`. Start-up costs seconds.
- **Microsoft Word / Office automation** gives the highest DOCX fidelity, but it is Windows-only and server use is unsupported. Commercial engines (Aspose, Syncfusion, Nutrient/PSPDFKit) re-implement DOCX layout.

**Trade-off:** templates separate content from code and give non-developers a designer. In return the expressiveness is bounded by the engine. Every JSON and JSX engine reinvents a slice of CSS and lacks the rest (no floats, limited text shaping, and usually no tagging; react-pdf and pdfmake produce untagged PDFs).

### 1.7 Print-to-PDF via OS drivers, Cairo and Skia backends

Several general-purpose 2D graphics libraries have a **PDF backend**. You record drawing commands and the library serialises them as PDF operators. Any application drawing through these libraries can therefore "print to PDF".

| Backend | Used by | Notes |
|---|---|---|
| **Skia SkPDF** (`SkPDF::MakeDocument`) | Chromium printing/`page.pdf()`, Android `PdfDocument`, QuestPDF (native Skia), Flutter printing paths | Supports font subsetting, tagged structure (`SkPDF::StructureElementNode`), links, metadata, PDF/A-ish options. BSD-3 |
| **Cairo PDF surface** (`cairo_pdf_surface_create`) | GTK printing, `pdftocairo`, Inkscape export, WeasyPrint (before it moved to pydyf) | LGPL-2.1/MPL-1.1. Good vector output, limited structure/tagging |
| **Core Graphics** (`CGPDFContext`) | macOS/iOS "Save as PDF", Apple PDFKit | Proprietary OS API |
| **OS printer drivers** | Windows "Microsoft Print to PDF", macOS print dialog, CUPS (`cups-pdf`; PDF is the native job format in cups-filters) | No control over structure or tagging; desktop-automation use only |

**Trade-off:** you get identical drawing code for screen and paper. But the output carries only the *visual* layer: no logical structure beyond what the backend exposes, no form fields, and limited control over fonts and colour spaces.

### 1.8 Choosing an approach: summary

| Need | Best-fit approach |
|---|---|
| Invoices/reports in an existing web app, designers own the layout | HTML/CSS (Chromium, or WeasyPrint/Prince for paged-media features) |
| High volume (10⁵–10⁷ docs/day), fixed layouts | Layout library (QuestPDF, iText, ReportLab) or drawing API plus your own templating |
| Books, technical docs, math | Typst / LaTeX |
| Fill or stamp existing PDFs | Object-level editing (pdf-lib, pikepdf, PDFBox, qpdf) |
| Archival (PDF/A) and accessibility (PDF/UA) by construction | Engines that emit them natively: Typst/krilla, iText, Prince, WeasyPrint (experimental), QuestPDF, LibreOffice; always validate with veraPDF |
| Office documents in, PDF out | LibreOffice/Gotenberg (open source) or commercial DOCX engines |

---

## 2. Ecosystem survey by language

Legend: **G** = generate, **P** = parse/extract, **E** = edit/manipulate, **R** = render/rasterise. "Status" means the latest registry release as of 2026-10-28. ⚠ marks a licensing trap for a permissively licensed project such as lamina.

### 2.1 JavaScript / TypeScript

| Library | Version (date) | License | Purpose | Status & notes |
|---|---|---|---|---|
| **pdf-lib** | 1.17.1 (2021-11-06) | MIT | G E | **Effectively unmaintained**: no npm release in ~5 years. Pure TS, runs everywhere; create, modify, fill forms, copy pages. No text layout, no shaping, no tagging, no rendering. Active fork **@cantoo/pdf-lib** 2.11.1 (2026-09-15) adds SVG, encryption and fixes |
| **PDFKit** | 0.20.2 (2026-08-30) | MIT | G | Drawing API with text wrapping, fontkit (OpenType layout), vector graphics, some AcroForm and basic tagging support. Node and browser (bundled) |
| **jsPDF** | 4.2.1 (2026-03-17) | MIT | G | Browser-first drawing API with plugins (AutoTable). `html()` plugin rasterises via html2canvas, which gives low fidelity and no selectable text. Has had a steady stream of CVEs (ReDoS, Node file-access issues), so keep it current |
| **pdfmake** | 0.3.13 (2026-10-25) | MIT | G | JSON doc definitions → layout on PDFKit. Tables, columns, headers/footers, TOC. 0.3 is a long-running beta line |
| **@react-pdf/renderer** | 4.9.0 (2026-08-27) | MIT | G | JSX + Yoga flexbox; server and browser. Hyphenation and custom fonts supported; no tagging |
| **pdf.js** (`pdfjs-dist`) | 6.4.299 (2026-10-03) | Apache-2.0 | P R (some E) | Mozilla's engine (Firefox's viewer). Canvas rendering, text-layer extraction, annotation/forms layer; recent versions add annotation **editing** (ink, free text, stamps, highlights) and saving |
| **unpdf** | 1.9.1 (2026-10-19) | MIT | P R | UnJS wrapper with a serverless build of pdf.js for edge, Workers and Deno: text, links, images, metadata, optional canvas rendering |
| **Puppeteer** | 25.16.0 (2026-10-22) | Apache-2.0 | G (HTML→PDF) | `page.pdf()`; `tagged` defaults to **true**, `outline` defaults to false (both marked experimental in typings) |
| **Playwright** | 1.64.1 (2026-10-14) | Apache-2.0 | G (HTML→PDF) | `page.pdf()` in Chromium only (not Firefox/WebKit) |
| **pdfme** | @pdfme/generator 6.2.8 (2026-10-21) | MIT | G | Template designer + JSON schema; built on pdf-lib |
| **mupdf** (mupdf.js) | 1.28.1 (2026-09-06) | ⚠ **AGPL-3.0** / commercial | P E R | Official Artifex WASM build: render, extract, edit, redact |
| **@embedpdf/pdfium** | 2.15.1 (2026-09-16) | MIT (wrapper) + PDFium Apache/BSD | P E R | PDFium compiled to WASM; powers the EmbedPDF viewer. Also `@hyzyla/pdfium` 3.0.0 (MIT) |
| **@libpdf/core** | 0.5.3 (2026-10-16) | MIT | G P E | New TypeScript parse+generate library; young, worth watching |
| **muhammara** | 6.0.6 (2026-08-23) | Apache-2.0 | G E | Native Node binding of PDF-Writer (HummusJS successor); fast streaming append/merge |
| **Vivliostyle** | core 2.45.2 / cli 11.4.0 | ⚠ **AGPL-3.0** | G | CSS paged-media typesetting inside Chromium |
| **Paged.js** | 0.4.3 (2023-07-06) | MIT | G | Paged-media polyfill; stagnant on npm |

### 2.2 Python

| Library | Version (date) | License | Purpose | Notes |
|---|---|---|---|---|
| **ReportLab** | 5.1.0 (2026-10-22) | BSD | G | Canvas + PLATYPUS layout; charts (`graphics`). The commercial ReportLab PLUS adds RML templates. Mature, fast |
| **fpdf2** | 2.8.9 (2026-09-29) | LGPL-3.0 | G (light E) | Pure Python; HarfBuzz shaping (via uharfbuzz), SVG, tables, basic tagging/outline, PDF/A *partial*. Very active |
| **WeasyPrint** | 70.0 (2026-09-08) | BSD-3 | G | HTML/CSS paged media (§1.4); system deps on Pango |
| **pypdf** | 6.21.3 (2026-10-27) | BSD-3 | P E | Pure Python: split, merge, crop, encrypt, form fill, text extraction. Frequent releases, many of them security fixes for malformed-input DoS |
| **pikepdf** | 10.17.0 (2026-10-11) | MPL-2.0 | P E | Pythonic qpdf binding; best-in-class for lossless object-level editing and repair |
| **PyMuPDF** | 2.0 (2026-10-19) | ⚠ **AGPL-3.0** / commercial (Artifex) | P E R G | Fastest all-rounder (MuPDF). The version jumped from 1.28.2 to 2.0; companion `pymupdf4llm` 2.0 serves LLM ingestion |
| **pypdfium2** | 5.14.0 (2026-10-04) | BSD-3 / Apache-2.0 | P R (E) | PDFium binding; the permissive alternative to PyMuPDF for rendering |
| **pdfminer.six** | 20260107 (2026-01-07) | MIT | P | Layout-analysis text extraction (LAParams); slow but precise character boxes |
| **pdfplumber** | 0.11.10 (2026-06-15) | MIT | P | Built on pdfminer.six; tables, visual debugging |
| **borb** | 3.0.9 (2026-08-03) | ⚠ **AGPL-3.0** / commercial | G P E | Pure-Python layout engine + reader |
| **xhtml2pdf** | 0.2.28 (2026-10-25) | Apache-2.0 | G | HTML→PDF on ReportLab; limited CSS |
| **pyHanko** | 0.38.0 (2026-10-17) | MIT | E (sign) | PAdES signing/validation, timestamps, LTV (ch. 04) |
| **OCRmyPDF** | 17.13.0 (2026-09-28) | MPL-2.0 | E | Adds OCR text layer (Tesseract), PDF/A output (via Ghostscript ⚠ AGPL at runtime), optimisation |

### 2.3 Java / JVM

| Library | Version (date) | License | Purpose | Notes |
|---|---|---|---|---|
| **Apache PDFBox** | 3.0.8 (2026-07-15) | Apache-2.0 | G P E R | Full-stack: parse, render (Java2D), text extract, forms, signing; preflight (PDF/A-1b). 4.0 in development |
| **iText Core** | 9.8.0 (2026-09-30) | ⚠ **AGPL-3.0** / commercial | G P E | iText 5 → 7 → rebranded "Core 8" → **Core 9** (current). Best-in-class layout/tagging/PDF-UA/PDF-A/signatures. Add-ons (pdfHTML, pdfSweep, pdfCalligraph…) |
| **OpenPDF** | 3.0.5 (2026-05-22) | LGPL-2.1 / MPL-2.0 | G E | Fork of iText 4 (pre-AGPL). Used by JasperReports, Flying Saucer |
| **Flying Saucer** | 10.7.0 (2026-10-24) | LGPL-2.1 | G | XHTML/CSS → PDF on OpenPDF |
| **openhtmltopdf** | 1.1.103 (io.github fork, 2026-10-27) | LGPL-2.1 | G | Flying Saucer descendant on PDFBox; PDF/UA, PDF/A, SVG (Batik). The original `com.openhtmltopdf` stopped at 1.0.10 (2021) |
| **veraPDF** | validation-model 1.30.3 (2026-10-08) | GPLv3+ / MPL-2.0 | Validate | Reference PDF/A + PDF/UA validator (§6) |

### 2.4 .NET

| Library | Version | License | Purpose | Notes |
|---|---|---|---|---|
| **QuestPDF** | 2026.9.1 | ⚠ **Source-available, revenue-gated**: free Community license for orgs < USD 1M annual gross revenue; Professional/Enterprise paid. Per the License Selection Guide (effective 2026-06-10) public-sector bodies and listed companies are not Community-eligible | G (+merge/edit) | Fluent layout API on Skia; PDF/A and PDF/UA, ZUGFeRD/Factur-X, merge. Excellent DX. **Not OSI open source** |
| **PDFsharp / MigraDoc** | 6.5.0; 7.0.0-preview-2 (2026-10-14) | ⚠ **Changed in 6.5.0: dual AGPL-3.0 / commercial** (≤ 6.2.x was MIT). The 6.5 runtime refuses `new PdfDocument()` until you call `PdfSharpLicense.SetAgplLicense()` or load a commercial key | G E | Classic drawing (`XGraphics`) + MigraDoc layout. Vendor states no further updates for the MIT 6.2 line. **Big 2026 change, easy to miss** |
| **iText .NET** | 9.8.0 (2026-09-30) | ⚠ AGPL / commercial | G P E | Same as Java (auto-ported) |
| **PdfPig** | 0.1.x (prereleases through Oct 2026) | Apache-2.0 | P (light G) | PDFBox-inspired pure C# parser; text with positions, layout analysis |
| **PDFium bindings** (PDFiumCore, Docnet) | — | Apache/BSD/MIT | R P | Docnet.Core is stale (2.7.0-alpha, 2023) |

### 2.5 Rust

| Crate | Version (date) | License | Purpose | Notes |
|---|---|---|---|---|
| **pdf-writer** | 0.15.0 (2026-05-27) | MIT/Apache-2.0 | G (objects) | Typst team. Typed, streaming, zero-parse writer; very fast; you bring fonts and layout |
| **krilla** | 0.8.2 (2026-06-04) | MIT/Apache-2.0 | G (high-level) | Typst's PDF backend: text with shaping input, font subsetting (via `subsetter`/`skrifa`), images, gradients, **tagging, PDF/A, PDF/UA** validators built in |
| **lopdf** | 0.46.0 (2026-10-21) | MIT | P E G | In-memory object graph; load, modify, save; decent for merges. Weaker on broken files and encryption edge cases |
| **printpdf** | 0.12.8 (2026-09-05) | MIT | G | Drawing API (the 0.8+ rewrite has an op-list model and HTML-ish layout experiments) |
| **pdf** (pdf-rs) | 0.10.0 (2026-03-02) | MIT | P | Typed object parser; slower cadence |
| **hayro** (+ hayro-syntax / -interpret / -svg / -font / -jbig2 / -jpeg2000) | 0.8.0 (2026-10-04) | MIT/Apache-2.0 | P R | **Pure-Rust, CPU-only PDF renderer** (Laurenz Stampfl, Typst). 1000+-file regression suite; self-described as experimental but the most complete pure-Rust rasteriser |
| **pdfium-render** | 0.9.4 (2026-09-06) | MIT/Apache-2.0 | P E R G | Idiomatic high-level binding to a dynamically or statically linked PDFium; also targets WASM |
| **typst** / **typst-pdf** | 0.15.1 (2026-07-17) | Apache-2.0 | G | Embeddable compiler (you implement the `World` trait for files and fonts) |
| **mupdf** (mupdf-rs) | 0.8.0 | ⚠ AGPL-3.0 | P E R | Binding to MuPDF |
| **pdf_oxide**, **oxidize-pdf** | 0.3.78 / 5.5.3 | MIT/Apache / MIT | P E G | Newer entrants (text extraction focus / all-round); evaluate maturity before depending on them |

### 2.6 Go

| Library | Version (date) | License | Purpose | Notes |
|---|---|---|---|---|
| **jung-kurt/gofpdf** | v1.16.2 (2019-11) | MIT | G | **Archived** by its author |
| **go-pdf/fpdf** | v0.9.0 (2023-09-13) | MIT | G | Community continuation; **no tagged release since 2023**. Development reportedly moved to Codeberg *(archive status unverified)* |
| **pdfcpu** | v0.16.4 (2026-10-27) | Apache-2.0 | P E (light G) | Pure Go processor + CLI: validate, optimise, merge, split, n-up, booklet, stamp/watermark, encrypt, forms, attachments. Used inside Gotenberg |
| **UniPDF** (unidoc) | v4.11.0 (2026-07-03) | ⚠ **Commercial** (source on GitHub, but use requires a paid/metered licence key) | G P E R | Full-featured but paid |
| **signintech/gopdf** | v0.38.3 (2026-10-25) | MIT | G | Active drawing API with TTF subsetting |
| **maroto v2** | v2.4.4 (2026-10-22) | MIT | G | Grid/row-based layout on gofpdf lineage |
| **go-pdfium** (klippa) | v1.21.4 (2026-10-21) | MIT | R P E | PDFium via CGo or **WebAssembly (wazero)**, so no CGo is needed |
| **Gotenberg** | v8.39.0 (2026-10-26) | MIT | G (service) | §1.4 |

### 2.7 C / C++

| Library | License | Purpose | Notes (versions from distro trackers/search, not fetched upstream) |
|---|---|---|---|
| **PDFium** | BSD-3 / Apache-2.0 | P R E (G limited) | Google/Foxit; Chrome's engine. No stable API/ABI or releases. Use prebuilt `pdfium-binaries` (incl. WASM) |
| **MuPDF** | ⚠ AGPL-3.0 / commercial | P R E G | Artifex. 1.28.x in 2026 (matches mupdf.js 1.28.1). Small, fast; also XPS/EPUB/CBZ; `mutool` |
| **Poppler** | ⚠ **GPL-2.0/3.0** (not LGPL) | P R | Xpdf 3.0 fork; Splash and Cairo backends; Evince/Okular/LibreOffice import. 26.x monthly releases (26.07.0 July 2026; 26.08 appears) |
| **Xpdf** | ⚠ GPL-2/3 | P R | Original engine; also `pdftotext` etc. |
| **qpdf** | Apache-2.0 | P E | Structural transformations, linearisation, encryption, repair, JSON v2 export. 12.x (12.4.0 docs dated June 2026) |
| **PoDoFo** | LGPL-2.0 (lib) | P E G | C++17 parse/modify/create; 1.0 released 2025 *(unverified date)* |
| **Cairo** | LGPL-2.1 / MPL-1.1 | G (backend) | §1.7 |
| **Skia** | BSD-3 | G (backend) R | SkPDF; Skia also rasterises for PDFium and Chrome |
| **libHaru** | zlib | G | Simple C drawing API; low activity |
| **Ghostscript** | ⚠ AGPL-3.0 / commercial | R E G | PostScript+PDF interpreter (new C PDF interpreter is default since 10.0). 10.06.0 released 2025-09; 2026 distro advisories reference 10.07–10.08 |
| **PDF-Writer** (Hummus) | Apache-2.0 | G E | Streaming writer/appender behind muhammara |

### 2.8 Swift / Apple platforms

- **PDFKit (Apple)**: `PDFDocument`/`PDFPage`/`PDFView`: viewing, annotation, text search and selection, page insert/remove, writing. It is built on **Core Graphics' PDF engine** (`CGPDFDocument` parsing; `CGPDFContext` writing). It is closed source and platform-only, and has a history of lossy saves: re-serialising can drop structure or incremental-update information, so treat it as a viewer/editor, not a preservation tool.
- **Core Graphics**: `UIGraphicsPDFRenderer` (iOS) / `CGContext(consumer:mediaBox:)` (macOS) for generation via the drawing API; `CGPDFScanner` for low-level content-stream parsing.
- Cross-platform Swift has no mature pure-Swift PDF library. Teams wrap PDFium or MuPDF.

### 2.9 Command-line tools

| Tool | License | Best at |
|---|---|---|
| `qpdf` | Apache-2.0 | `--check`, `--linearize`, `--object-streams=generate`, `--encrypt/--decrypt`, `--pages` merge/split, `--overlay/--underlay`, `--json` (structural dump), `--qdf` (human-editable normalised form) |
| `mutool` (MuPDF) | AGPL | `draw` (raster/SVG/text), `clean -gggz` (GC/compress/repair), `extract`, `show`, `merge`, `poster`, `run` (JS scripting) |
| `pdftk` (pdftk-java) | GPL-2.0 | Classic merge/split/burst/stamp/form fill (`fill_form`, `dump_data_fields`) |
| Ghostscript `gs` | AGPL | `-sDEVICE=pdfwrite` re-distillation (PDF/A, `-dPDFSETTINGS` compression), rasterisation (`png16m`, `tiffsep`), PS→PDF |
| `pdfcpu` | Apache-2.0 | validate/optimise/n-up/booklet/stamp/watermark/forms in a single static binary |
| `ocrmypdf` | MPL-2.0 | OCR + PDF/A + optimisation pipeline |
| `verapdf` | GPL/MPL | PDF/A-1…4, PDF/UA-1/2 validation, feature reports, Arlington checks |
| Poppler utils | GPL | `pdfinfo`, `pdftotext -layout`, `pdftoppm`/`pdftocairo`, `pdfimages -list/-all`, `pdffonts`, `pdfseparate`/`pdfunite`, `pdfsig`, `pdfattach` |

### 2.10 License landscape at a glance

```
Permissive (MIT/BSD/Apache/MPL):  pdf.js, PDFium, qpdf, pikepdf, pypdf, pypdfium2, PDFBox, pdfcpu, pdf-lib,
                                 PDFKit, pdfmake, react-pdf, ReportLab, WeasyPrint, Typst, krilla, pdf-writer,
                                 hayro, lopdf, Skia, PdfPig, go-pdfium, muhammara/PDF-Writer
Weak copyleft (LGPL):            fpdf2 (LGPL-3), OpenPDF, Flying Saucer, openhtmltopdf, PoDoFo, Cairo
Strong copyleft (GPL):           Poppler, Xpdf, pdftk, veraPDF (GPL/MPL dual), Ghostscript*
Network copyleft (AGPL) + commercial:  iText (+pdfHTML), MuPDF/PyMuPDF/mupdf.js/mupdf-rs, Ghostscript, borb,
                                 Vivliostyle, **PDFsharp/MigraDoc ≥ 6.5**
Proprietary / revenue-gated:     QuestPDF (Community < $1M), UniPDF, Prince, Aspose, Nutrient, Apryse
```

---

## 3. Processing operations under the hood

Almost every manipulation reduces to three primitives on the object graph (ch. 01): **(a)** copying a reachable subgraph between documents and renumbering it, **(b)** rewriting or wrapping content streams, **(c)** re-serialising with different writer options. The difficulty is almost always in the *document-level* structures that reference pages: outlines, destinations, forms, the structure tree, optional content, and page labels.

### 3.1 Merge and split

```
src A ─┐   for each selected page P:
       │     1. deep-copy P and everything reachable from it, EXCEPT /Parent
src B ─┤        (memo table oldRef→newRef so shared objects copy once per source)
       │     2. materialise inherited attrs (/Resources /MediaBox /CropBox /Rotate)
       ▼        from ancestor /Pages nodes onto the copied page
   dest doc  3. append to dest /Pages /Kids, fix /Count
             4. remap document-level structures (below)
             5. write; optionally dedupe identical objects by content hash
```

Pitfalls that separate toy mergers from production ones:

- **Inherited attributes.** A page can inherit `/Resources`, `/MediaBox`, `/CropBox` or `/Rotate` from its `/Pages` ancestors. If the page is copied without them, it renders blank or at the wrong size.
- **Resource deduplication.** Merging 100 invoices that each embed the same subset of the same font gives 100 font programs. Hashing stream data plus the dictionary finds exact duplicates. *Subsets* of one font differ, so they cannot be merged without re-subsetting, which means rewriting glyph IDs in content streams. Few tools do that.
- **Outlines and destinations.** Outline items and link annotations point to page objects (`[pageRef /XYZ …]`) or to *named* destinations. Copied pages need remapped refs. Named destinations from different sources collide in the `/Dests` name tree, so they need prefixing. Links to pages that were *not* copied must be dropped or retargeted.
- **AcroForm.** Field values are keyed by fully-qualified name, so two `invoice.total` fields from different sources become **one** field with shared value after a naïve merge. The `/AcroForm /Fields` array, `/DR` default resources and `/NeedAppearances` must also be merged. Tools rename fields (`pdftk`, iText `PdfPageFormCopier`, pypdf with care).
- **Structure tree (tagged PDF).** Each page's marked content links back through `/StructParents` → the `/ParentTree` number tree → struct elements. To merge you have to concatenate `/StructTreeRoot /K` children under a new `Document` element, **renumber every `/StructParents`/`/StructParent` key** and the ParentTree, and merge `/RoleMap`, `/ClassMap` and (PDF 2.0) namespaces. Most mergers silently drop tags, which destroys PDF/UA conformance. iText, PDFBox (with effort), qpdf (structurally preserves but does not merge the tree), and Acrobat handle it properly.
- **Other catalog-level entries:** `/OCProperties` (optional content groups), `/PageLabels` (number tree keyed by page index), `/Names /EmbeddedFiles`, `/Names /JavaScript`, `/OutputIntents` (conflicting PDF/A output intents), `/Metadata` XMP, `/Lang`.
- **Signatures.** Merging rewrites the document, so every signature becomes invalid (ch. 04).

**Split** is merge with one source. The important extra step is **garbage collection**: write only objects reachable from the new trailer. Otherwise every split part carries the whole original's fonts and images. `qpdf --empty --pages in.pdf 1-3 -- out.pdf`, `pdfcpu split`, `mutool clean -gg`.

### 3.2 Page imposition and n-up

Imposition places several source pages on one sheet (n-up, booklets, step-and-repeat labels).

```
source page ──► Form XObject:  << /Type /XObject /Subtype /Form
                                  /BBox [cropbox] /Resources <page res>
                                  /Matrix [rotation-normalising matrix] >>
                                stream: <page content streams concatenated>
sheet content:  q  0.5 0 0 0.5 0 396 cm  /Pg1 Do  Q
                q  0.5 0 0 0.5 306 396 cm /Pg2 Do Q   …
```

- `/Rotate` and non-zero CropBox origins must be folded into the matrix.
- **Annotations do not travel with Form XObjects.** Links and form widgets need their `/Rect` transformed and re-attached to the sheet, or they are flattened first. Most n-up tools drop them.
- Tools: `pdfcpu nup` / `booklet`, `qpdf --overlay/--underlay`, `mutool poster` (the reverse operation), LaTeX `pdfpages`/`pdfjam`, PDFBox `LayerUtility`, pdf-lib `embedPage`.

### 3.3 Stamping and watermarking

There are three mechanisms, which differ in reversibility and how viewers treat them:

1. **Content-stream overlay (most common).** Append a new content stream to the page's `/Contents` array: `q <cm> /Stamp Do Q`. The stamp is a Form XObject, so it is defined once and drawn on every page. **Wrap the original content in `q … Q` first** (prepend a stream containing `q`, append one containing `Q`), because the original may leave a modified CTM or clipping path in effect, which would displace the stamp. Underlays prepend instead (which needs opaque-background awareness). Text stamps need a font resource added to each page's `/Resources` (watch for name collisions such as `/F1`).
2. **Watermark annotation** (`/Subtype /Watermark`, PDF 1.6). The stamp lives in an annotation appearance stream, so it can be hidden or removed and is known to be "not content". The optional `/FixedPrint` dictionary makes it print at a fixed size and position regardless of the printed page's dimensions (ISO 32000-1 §12.5.6.22).
3. **Optional content.** Put the overlay inside `/OC /oc1 BDC … EMC` so viewers can toggle it and print-only watermarks are possible.

Tagged documents should mark watermarks as `/Artifact` (`/Artifact <</Type /Pagination /Subtype /Watermark>> BDC … EMC`) so screen readers skip them. Any stamping invalidates existing signatures unless it is done as an incremental update that the signature's DocMDP permissions allow (ch. 04).

### 3.4 Optimisation and compression

| Technique | Mechanism | Typical gain | Tools |
|---|---|---|---|
| Object streams + xref streams (PDF 1.5) | Pack non-stream objects into Flate-compressed `/ObjStm`; binary xref | 10–30% on object-heavy files | `qpdf --object-streams=generate`, pdf-lib (default), `mutool clean -z` |
| Re-Flate at level 9 / add Flate to uncompressed streams | — | small–medium | `qpdf --recompress-flate --compression-level=9` |
| Image downsampling + JPEG re-encoding | Decode, resample to target DPI (e.g. 150), re-encode DCT q≈75–85 | Often 50–90% for scans/photos | Ghostscript `-dPDFSETTINGS=/ebook`, OCRmyPDF `--optimize 2/3`, pikepdf + Pillow |
| Bilevel → JBIG2 (generic region; avoid lossy symbol mode, see note) | | 5–10× on B/W scans | `jbig2enc` via OCRmyPDF |
| Font subsetting / unembedding duplicates | Keep only used glyphs; dedupe identical programs | Large when full CJK fonts are embedded | Producers (at write time); Ghostscript re-distil; hard post-hoc |
| Object dedup | Hash-cons identical objects/streams | Large for merged batches | `qpdf` (partially), `pdfcpu optimize`, `mutool clean -ggg` |
| Garbage collection | Drop unreachable objects and orphaned incremental-update history | Varies | `mutool clean -gg`, `qpdf` (default when rewriting) |
| Strip extras | Thumbnails, piece info, embedded XMP history, unused named dests | Small | Various |

**Re-distilling** (Ghostscript `pdfwrite`) works differently: it *interprets* the page and writes a new PDF. That compresses well, but it can lose tags, form fields, optional content and object identity. It is the wrong tool for structure-preserving pipelines. **Lossy JBIG2 symbol mode** is notorious for substituting look-alike digits (the Xerox scanner incident), so keep it off for documents where digits matter.

### 3.5 Linearisation ("Fast Web View")

Annex F of ISO 32000 defines a layout that starts with the **linearisation parameter dictionary** (entirely within the first 1024 bytes), then the first-page cross-reference section, the catalog and other document-level objects, the primary **hint stream** (page offset and shared-object hint tables), and only then the first page's objects, followed by the remaining pages (ISO 32000-1 F.3). A viewer can therefore show page 1 before the download finishes.

- Writing it is a full rewrite: compute the object ordering and hint streams, then fix up offsets (two passes). `qpdf --linearize`, `mutool clean -l`, Ghostscript `-dFastWebView`.
- In practice, modern viewers (pdf.js, PDFium) use **HTTP range requests** for *any* PDF with a valid trailer, and they ignore hint tables or treat them as optional. Any incremental update breaks linearisation: it is still permitted, but the result is no longer linearized and must be treated as ordinary PDF (ISO 32000-1 F.1). **Low priority** for lamina, but the reader must tolerate it and `--check` should validate it.

### 3.6 Repair

Broken files are common: truncated downloads, wrong xref offsets, `\r\n` damage from text-mode FTP, junk before `%PDF`. Recovery works like this:

1. Try the trailer at EOF → `startxref` → xref table or stream.
2. On failure, **scan the whole file** for `N G obj` patterns and rebuild the xref (the last occurrence wins). Find the `/Root` by locating `/Type /Catalog`.
3. Repair stream `/Length` by searching for `endstream`, and resynchronise the lexer after garbage.

qpdf, MuPDF, PDFium, pdf.js, Poppler and Ghostscript each repair *differently*. A malformed file can therefore render differently, or show different text, in different viewers. These are *parser differentials*, a known attack surface (ch. 04, SafeDocs). A library should expose a "strict" mode that reports what it repaired.

### 3.7 Conversion to PDF/A

PDF/A (ISO 19005) is a *constrained* PDF: every font embedded, colour defined (OutputIntent with ICC profile), XMP metadata with `pdfaid:part/conformance`, no encryption, no JavaScript or external content, annotation appearance streams present, and (PDF/A-1 only) no transparency. Level "a" additionally requires tagging (ch. 04). Converting an *arbitrary* PDF means:

- embedding fonts that aren't embedded (possible only if the font is available, otherwise substitution changes appearance);
- converting DeviceRGB/CMYK use under an output intent, or replacing it with ICC-based colour;
- flattening transparency for PDF/A-1 (rasterises, which is lossy);
- removing or flattening forbidden features (JS, multimedia, encryption);
- writing XMP and validating.

Tools: Ghostscript (`-dPDFA=2 -dPDFACompatibilityPolicy=1` + ICC profile), OCRmyPDF (wraps Ghostscript), LibreOffice (export), Apryse/Adobe/callas (commercial, highest success rate). **Producing PDF/A at generation time is far more reliable than converting afterwards.** Typst/krilla, iText, Prince, WeasyPrint and QuestPDF do so. Always validate with veraPDF, because tools routinely emit "PDF/A" that fails it.

### 3.8 Image extraction

You have to walk *every* place an image can live:

- page `/Resources /XObject` entries with `/Subtype /Image`;
- **inline images** (`BI … ID <data> EI`) inside content streams, which can only be found by parsing the content stream properly; the `EI` terminator is ambiguous in binary data;
- images inside Form XObjects, Type 3 glyphs, patterns, annotation appearance streams and soft masks.

Output strategy:

- `DCTDecode` data → write the bytes as `.jpg` (lossless pass-through, but watch `/Decode` arrays and Adobe CMYK inversion).
- `JPXDecode` → `.jp2`.
- Everything else → decode filters (Flate with PNG predictors, LZW, CCITT, JBIG2 with `/JBIG2Globals`, RunLength), apply `/ColorSpace` (Indexed, ICCBased, Separation, DeviceN), `/BitsPerComponent`, `/Decode`, then encode PNG.
- `/SMask` or `/Mask` gives alpha, which has to be composited or saved separately.

Note that "the image as stored" differs from "the image as displayed" (the CTM can scale, rotate, clip or mask it). `pdfimages -list` is a good diagnostic.

### 3.9 PDF → image rasterisation

Rasterisation is page rendering (§4) into a bitmap at a chosen DPI. Production concerns:

- determinism (pin the renderer version, fonts for non-embedded fallbacks, and the anti-aliasing settings);
- colour management (ICC, overprint simulation for print previews; Ghostscript's `tiffsep` gives separations);
- memory (a 300-DPI A0 page is ~14000×19900 px ≈ 1.1 GB RGBA, so tile or band it);
- time limits (pathological content such as millions of tiny paths or huge shading meshes calls for per-page timeouts and process isolation).

Tools: `pdftoppm`/`pdftocairo`, `mutool draw`, PDFium (`FPDF_RenderPageBitmap`), Ghostscript `png16m`, pdf.js + node-canvas, hayro.

### 3.10 Diff and compare

| Level | How | Good for |
|---|---|---|
| **Visual** | Rasterise both at the same DPI with the same engine → pixel diff (exact, tolerance, or perceptual such as SSIM or `pixelmatch` with anti-aliasing detection) → highlight overlay | Regression testing of generators; "did anything visibly change" |
| **Text** | Extract text in reading order (needs good extraction; tagged PDFs help) → sequence diff | Contract and document review |
| **Structural** | Canonicalise the object graph (`qpdf --json` v2, `--qdf`), compare by *path* from the root, not by object number | Debugging writers; understanding what a tool changed |
| **Incremental-update aware** | Compare revisions inside one file (each `%%EOF` section) | Signature forensics (ch. 04) |

Off-the-shelf tools: `diff-pdf` (visual), DiffPDF/`comparepdf`, `pdf-visual-diff` (npm), Apryse/Adobe compare (commercial).

---

## 4. Rendering engines compared

### 4.1 Architecture overview

```
              ┌──────────── parse ───────────┐  ┌──── interpret ────┐  ┌──── rasterise ────┐
 pdf.js       │ JS lexer/parser (worker)     │→ │ operator list     │→ │ Canvas 2D (main)  │  + HTML text/annot layers
 PDFium       │ C++ CPDF_Parser              │→ │ CPDF_PageObjects  │→ │ AGG or Skia       │  in Chrome's sandbox
 MuPDF        │ C fitz/pdf                   │→ │ fz_device calls   │→ │ fitz draw device  │  (also SVG/text devices)
 Poppler      │ C++ (Xpdf heritage)          │→ │ Gfx → OutputDev   │→ │ Splash or Cairo   │
 Ghostscript  │ C PDF interpreter (≥10.0)    │→ │ graphics library  │→ │ devices (png, tiff, pdfwrite…) │
 Apple PDFKit │ CoreGraphics CGPDF (closed)  │→ │ Quartz            │→ │ Quartz / Metal    │
 hayro        │ hayro-syntax (Rust)          │→ │ hayro-interpret → Device trait │→ │ CPU rasteriser │ + hayro-svg
```

### 4.2 Comparison

| Engine | Language / License | Where it ships | Fidelity | Speed | Safety & sandboxing | WASM |
|---|---|---|---|---|---|---|
| **pdf.js** | JS / Apache-2.0 | Firefox built-in viewer; countless web viewers; Thunderbird | Very good on real-world files; huge regression corpus. Blend modes, shading and Type 3 edge cases occasionally differ; text rendering depends on the browser canvas | Slowest of the group (JS + canvas), but streaming and progressive | Memory-safe JS. Form JavaScript runs in a separate **QuickJS-in-WASM sandbox**. Historically had a font-handling code-exec bug (CVE-2024-4367, `FontMatrix` eval), fixed in v4.2.67 | Uses WASM for some decoders (e.g. JPEG 2000 via OpenJPEG, colour via qcms) in recent versions |
| **PDFium** | C++ / BSD-3 + Apache-2.0 | Chrome, Chromium-based browsers, Android (via Chrome and apps), many SDKs (PDF.js alternative in Electron apps), Foxit heritage | Excellent; tuned against Chrome's huge fuzzing and usage corpus; forms and XFA (optional, needs V8) | Fast | Runs in Chrome's renderer-process sandbox; continuously fuzzed on OSS-Fuzz/ClusterFuzz | **Yes**: `pdfium-binaries` WASM build, `@embedpdf/pdfium`, `@hyzyla/pdfium`, go-pdfium (wazero). ~4–6 MB WASM *(order-of-magnitude, build-dependent)* |
| **MuPDF** | C / AGPL + commercial | SumatraPDF, many mobile/e-reader apps, PyMuPDF | Excellent; strict but robust repair | **Fastest** in most informal benchmarks; small footprint | OSS-Fuzz; C memory-safety risk | **Yes**: official `mupdf` npm (mupdf.js), AGPL |
| **Poppler** | C++ / GPL | Linux desktop viewers (Evince, Okular), `pdftoppm`, LibreOffice import, many CLIs | Very good; Splash backend is the reference for Xpdf behaviour; Cairo backend for vector output | Good | OSS-Fuzz; frequent CVEs from fuzzing (several 2026 CVEs against 26.07) | Community builds only |
| **Ghostscript** | C / AGPL + commercial | Print pipelines, CUPS, prepress, PDF/A conversion | Best for PostScript, overprint, separations and print-specific features | Moderate | Long history of `-dSAFER` sandbox bypasses (PostScript is Turing-complete). The new PDF interpreter is in C, not PostScript. Isolate it | Experimental builds exist |
| **Apple PDFKit / Core Graphics** | ObjC/Swift / proprietary | Safari, Preview, iOS | Good; occasional divergence on edge cases | Fast, GPU-assisted | OS sandbox (BlastDoor-style isolation for some content). Closed source | No |
| **hayro** | Rust / MIT+Apache | New. Used in the Typst ecosystem (e.g. rendering PDF images for PNG/SVG export) *(integration detail unverified)*; GPUI viewer crates | Good and improving fast; 1000+ file regression suite; own JBIG2 / JPEG 2000 / CFF / Type1 decoders | CPU-only; competitive for a young project, not yet at MuPDF/PDFium level *(no independent benchmark found)* | **Memory-safe** pure Rust, a strong argument for untrusted input; fuzzable with cargo-fuzz | **Yes** (pure Rust → `wasm32` without emscripten) |

Microsoft Edge has used PDFium historically and has been migrating its built-in viewer to an Adobe Acrobat-based engine on Windows since 2023 *(rollout status in 2026 not verified)*.

### 4.3 Running PDF tooling in the browser and at the edge

| Option | Size / constraints | License | Notes |
|---|---|---|---|
| pdf.js (`pdfjs-dist`, `unpdf` serverless build) | ~1–3 MB JS (worker + main) | Apache-2.0 | Default choice for viewing/extracting in JS; `unpdf` targets Workers/Deno/edge |
| PDFium WASM (`@embedpdf/pdfium`, `@hyzyla/pdfium`) | Several MB WASM, emscripten runtime | Apache/BSD + MIT wrappers | Native-grade rendering + editing (annotations, forms, page ops) in browser |
| mupdf.js | Several MB WASM | **AGPL** | Feature-rich (redaction, conversion), but AGPL makes it unusable for a permissive library |
| Rust → `wasm32-unknown-unknown` (hayro, krilla, pdf-writer, lopdf, typst) | Sub-MB to few MB; no emscripten; works in Workers | MIT/Apache | Best path for **one codebase that runs native and in the browser** |
| Typst in browser (`typst.ts`) | Compiler WASM ~ tens of MB with fonts | Apache-2.0 | Shows that full layout + PDF in the browser is viable |
| Headless Chromium | Not in browser; serverless needs 50–60 MB compressed binary layers | — | Edge runtimes (Workers) can't run it; use remote browser services |

Edge constraints that matter: no filesystem (fonts must be bundled or fetched), CPU-time limits (often 10–50 ms on free tiers, up to minutes on paid tiers), 128 MB memory caps on some platforms, and WASM module size limits. Pure-Rust or pure-JS libraries without native dependencies win here.

---

## 5. Performance and scale

### 5.1 Writer architecture: streaming versus in-memory object graph

```
Streaming writer (pdf-writer, PDFKit, ReportLab, fpdf2*)       In-memory graph (pdf-lib, lopdf, pypdf, PDFBox, iText(edit))
 page 1 objs ──► write ──► [offset table]                        load all objects ──► mutate ──► serialise all
 page 2 objs ──► write ──► [offset table]                        memory ∝ document size (often 3–10× file size)
 ...           memory ∝ one page + font/image tables             random access, edit anything, easy dedup
 finish: fonts (subsetted at end), xref, trailer                 save = rewrite (or incremental append)
```

- **Streaming generation** is O(page) memory. One catch: font subsets can only be written once all text is known. Streaming writers therefore reserve object numbers for fonts and emit them at the end. PDF lets objects appear in any order because the xref gives the offsets.
- **"Page X of Y"** with streaming: use the deferred-XObject trick from §1.3.
- **In-memory editing** of a 2 GB scanned PDF needs **lazy loading**: parse the xref, load objects on demand, and stream unchanged objects through to the output. qpdf, PDFium and MuPDF do this. pdf-lib and pypdf (largely) do not, so they struggle beyond hundreds of MB.
- **Incremental writing.** Append only changed objects plus a new xref section after the original `%%EOF`. Saving costs O(changes), it is required for signatures (ch. 04), and it is the cheapest way to save edits to huge files. The trade-off is file growth and a visible edit history.

### 5.2 Concurrency

- Most PDF libraries are **not thread-safe per document**, but separate documents are independent. The model is "one document per worker".
- **PDFium has global state.** Calls must be serialised process-wide (pdfium-render and pypdfium2 document this), so scale with processes, not threads. MuPDF needs explicit locking callbacks (`fz_locks_context`) per context.
- Python's GIL limits pure-Python generators, so use process pools. Rust and Go generate pages in parallel easily when the writer can accept pre-encoded page content (encode content streams in parallel, then serialise in order).
- **Font subsetting and image encoding** are usually the hot spots; cache subsetted fonts and encoded images across documents in batch jobs.

### 5.3 The cost of headless Chrome

Indicative numbers *(typical community-reported ranges, not measured here; Chromium was not available in the sandbox)*:

| Metric | Native library (e.g. QuestPDF, pdf-writer, ReportLab) | Headless Chromium |
|---|---|---|
| Cold start | ms | 0.5–3 s (browser launch) |
| Simple 1–2 page invoice | 1–20 ms | 100–500 ms warm (new page + layout + print) |
| Memory | tens of MB | 150–300 MB per browser + 20–100 MB per tab |
| Determinism | High (pinned fonts) | Medium (browser version, font fallback, timing) |
| Container image | small | +300–600 MB (Chromium + fonts) |

Mitigations: keep a **browser pool** (reuse the browser, open a fresh context per job); disable unneeded features; use `waitUntil: 'networkidle0'` sparingly in favour of an explicit ready signal; inline assets; cap concurrency per core; recycle browsers every N jobs to limit leaks. At high volume a native layout library is often 10–100× cheaper per document.

### 5.4 Large documents

Practical rules:

- reuse resources across pages (one font dictionary, one image XObject referenced N times);
- flush pages as soon as they are complete;
- avoid one giant content stream; split per page;
- prefer page-tree balancing (a flat `/Kids` of 100k entries is legal but slow for some viewers; qpdf and iText build balanced trees);
- for massive tables, beware layout engines that measure the entire table before pagination (quadratic behaviour).

---

## 6. Testing and validation

### 6.1 Validators and checkers

| Tool | What it checks | Notes |
|---|---|---|
| **veraPDF** (1.30.x, Oct 2026) | PDF/A-1/2/3/4 (all conformance levels), PDF/UA-1 and PDF/UA-2 / WTPDF, plus feature extraction | The PDF Association-endorsed open-source reference validator; CLI, REST, Java API. Rules map to ISO clauses and the **Matterhorn Protocol** for PDF/UA |
| `qpdf --check` | Structural soundness: xref, streams decode, object syntax, linearisation hints | Fast, good CI gate for writers |
| `mutool info` / `mutool clean` | Parse + repair warnings | — |
| `pdfcpu validate` (relaxed/strict) | Syntax against the ISO 32000 model | — |
| Poppler `pdfinfo`, `pdffonts` | Basic metadata, fonts embedded / subset / ToUnicode present | `pdffonts` is a quick "will text extract?" check |
| **Arlington PDF Model** checkers | Every dictionary key, type, version and predicate against the machine-readable spec | See below |
| PAC (PDF Accessibility Checker) | PDF/UA, WCAG | Free, Windows GUI; often used alongside veraPDF |
| Adobe Preflight / callas pdfToolbox | Commercial, very deep | Reference "second opinion" |

**The Arlington PDF Model** is the PDF Association's open (Apache-2.0) machine-readable definition of PDF 2.0 (ISO 32000-2 with errata). It has **one TSV file per object type**, about 500+ objects and thousands of keys and array entries. Each row gives the key, allowed types, introduced/deprecated PDF version, required-ness, indirect-reference rules, possible values and cross-key **predicates** such as `fn:IsRequired(@Subtype==Link)`. It originated in the DARPA **SafeDocs** program and ships with Python/Java/C++ tooling (including a TestGrammar checker) and integration into veraPDF. **For lamina this is a gift.** The object model, its typed accessors and a validator can be *code-generated* from Arlington, so the library stays aligned with the spec mechanically.

### 6.2 Visual regression testing

```
fixtures (source → PDF)
     │ generate with lamina at commit X
     ▼
 rasterise (pinned renderer, fixed DPI, fixed fonts) ──► PNG
     │                                       ▲
     ▼                                       │ baseline PNGs in git-LFS
 compare: exact → if differ: perceptual (SSIM / pixelmatch AA-aware) with threshold
     │
     ▼
 report: overlay diff images; fail CI above threshold; "accept" updates baseline
```

- **Use at least two renderers** (e.g. PDFium + pdf.js, or hayro + PDFium). A file that looks right in one and wrong in another usually has a spec violation that one renderer tolerates. This is differential testing, and it is cheap.
- Also snapshot **semantic** outputs: extracted text, the structure tree (tags) and the qpdf JSON of key objects. These are more stable than pixels and catch tagging regressions that pixel tests can't see.
- Pin fonts. Never let a test depend on system fonts.

### 6.3 Test corpora

| Corpus | Contents | Use |
|---|---|---|
| **pdf.js test corpus** (`test/pdfs` + linked external files) | Thousands of real-world and bug-repro PDFs with reference snapshots | Parser/renderer robustness, regression |
| **PDFium test corpus** (`testing/resources`, ClusterFuzz corpora) | Targeted feature and crash cases | Robustness |
| **GovDocs1** | ~1 million files crawled from US `.gov` sites, a large share PDFs | Real-world distribution, extraction quality |
| **SafeDocs / DARPA corpora** (e.g. the Common Crawl `CC-MAIN-2021-31-PDF-UNTRUNCATED` set, ~8M PDFs re-fetched untruncated; plus targeted "issue tracker" corpora) | Huge, realistic, many malformed | Fuzz seeds, parser differentials, statistics |
| **veraPDF corpus**, **Isartor** (PDF/A-1b), **Matterhorn reference suite** (PDF/UA) | Purpose-built pass/fail files per rule | Validator correctness, conformance tests |
| **PDF Association PDF 2.0 examples**, **Arlington-derived synthetic files** | Small spec-feature files | Writer conformance |
| qpdf, Poppler, MuPDF test suites | Edge cases | — |

### 6.4 Fuzzing

PDF parsers are among the most fuzzed code on earth. PDFium, Poppler, MuPDF, qpdf, Xpdf and Ghostscript all run continuously on **OSS-Fuzz**. Lamina should do the same from day one:

- `cargo-fuzz` / libFuzzer targets for lexer, xref recovery, each filter decoder (Flate, LZW, CCITT, JBIG2, DCT, JPX), content-stream parser, font parsers, and "parse → write → parse" round-trip invariants;
- structure-aware fuzzing (grammar-based mutators that keep `obj`/`endobj` and xref plausible) reaches deeper;
- seed from SafeDocs and pdf.js corpora;
- enforce resource limits (decompression-bomb ratio caps, recursion depth, object count, time per page), because DoS bugs are the most common fuzz finding (pypdf's 2025–26 security releases are mostly in this class).

---

## 7. A verified tiny end-to-end demo

The same one-page document (title, paragraph, rectangle; US Letter) was produced four ways in the research sandbox on 2026-10-28 and checked with `pdfinfo` (Poppler 24.02.0), `pdftotext` and `pypdf` 6.21.3 in **strict** mode. All four parse cleanly, report one 612×792 pt page and extract the expected text. All four were also rasterised with `pdftoppm` and visually inspected. Sources are in [`examples/05-generation-approaches/`](examples/05-generation-approaches/).

Toolchain: Node 22.22.0 + pdf-lib 1.17.1; Python 3.13 + fpdf2 2.8.9 and ReportLab 5.1.0 (pip in a venv). No install failed. Chromium/Puppeteer and Typst were not attempted, to keep the demo small.

### 7.1 Raw writer: no dependencies, 41 lines of Python

```python
# Minimal dependency-free PDF writer: one page, title, paragraph, rectangle.
import sys, zlib

def esc(s):  # escape a PDF literal string
    return s.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")

TITLE = "Hello from lamina"
PARA = ["This one-page document was produced by hand-emitting PDF objects:",
        "a catalog, a page tree, one page, a font and a content stream."]

# 1. Content stream: graphics + text operators (see chapters 02/03).
ops = ["0.2 0.4 0.8 RG 2 w 72 600 468 120 re S",           # stroked rectangle
       "BT /F2 24 Tf 90 690 Td (%s) Tj ET" % esc(TITLE)]  # title
ops += ["BT /F1 12 Tf 14 TL 90 660 Td"] + ["(%s) '" % esc(l) for l in PARA] + ["ET"]
content = zlib.compress("\n".join(ops).encode("latin-1"))

# 2. Objects, numbered 1..n. Standard-14 fonts need no embedding (not PDF/A!).
objs = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
    b"/Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R >>",
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>",
    b"<< /Length %d /Filter /FlateDecode >>\nstream\n" % len(content)
        + content + b"\nendstream",
    b"<< /Title (%s) /Producer (raw_writer.py) >>" % esc(TITLE).encode(),
]

# 3. Serialize: header, objects (recording byte offsets), xref, trailer.
out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offsets = []
for i, body in enumerate(objs, 1):
    offsets.append(len(out))
    out += b"%d 0 obj\n" % i + body + b"\nendobj\n"
xref = len(out)
out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
out += b"".join(b"%010d 00000 n \n" % o for o in offsets)  # 20-byte entries
out += (b"trailer\n<< /Size %d /Root 1 0 R /Info 7 0 R >>\nstartxref\n%d\n%%%%EOF\n"
        % (len(objs) + 1, xref))
open(sys.argv[1] if len(sys.argv) > 1 else "raw.pdf", "wb").write(out)
```

Note what is *missing*. There is no embedded font, which works only with the standard 14 fonts and Latin-1 text, and fails PDF/A. There are no tags, no XMP, and no text measurement: line positions are hard-coded. Each of these is a chapter of work (ch. 03/04). The shape of the writer, however, is exactly what a real streaming writer does.

### 7.2 Drawing API: pdf-lib (Node)

```js
import { PDFDocument, StandardFonts, rgb } from 'pdf-lib';
import { writeFileSync } from 'node:fs';

const doc = await PDFDocument.create();
doc.setTitle('Hello from lamina');
const page = doc.addPage([612, 792]);                 // US Letter, points
const regular = await doc.embedFont(StandardFonts.Helvetica);
const bold = await doc.embedFont(StandardFonts.HelveticaBold);

page.drawRectangle({ x: 72, y: 600, width: 468, height: 120,
  borderColor: rgb(0.2, 0.4, 0.8), borderWidth: 2 });
page.drawText('Hello from lamina', { x: 90, y: 690, size: 24, font: bold });
page.drawText(
  'This one-page document was produced with pdf-lib, a drawing-API library.\n' +
  'Line breaks are explicit: pdf-lib does no paragraph layout.',
  { x: 90, y: 660, size: 12, font: regular, lineHeight: 14 });

writeFileSync(process.argv[2] ?? 'pdflib.pdf', await doc.save());
```

### 7.3 Drawing API with wrapping: fpdf2 (Python)

```python
import sys
from fpdf import FPDF

pdf = FPDF(unit="pt", format="letter")
pdf.set_title("Hello from lamina")
pdf.add_page()
pdf.set_draw_color(51, 102, 204); pdf.set_line_width(2)
pdf.rect(72, 72, 468, 120)                     # fpdf2: origin top-left, y down
pdf.set_font("Helvetica", "B", 24)
pdf.set_xy(90, 90); pdf.cell(text="Hello from lamina")
pdf.set_font("Helvetica", size=12)
pdf.set_xy(90, 125)
pdf.multi_cell(w=432, h=14, text=(            # automatic line wrapping
    "This one-page document was produced with fpdf2. multi_cell() wraps "
    "the paragraph to the given width, a first step towards layout."))
pdf.output(sys.argv[1] if len(sys.argv) > 1 else "fpdf2.pdf")
```

### 7.4 Layout engine: ReportLab PLATYPUS (Python)

```python
import sys
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import getSampleStyleSheet
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Flowable

class Box(Flowable):                     # custom flowable: a stroked rectangle
    def wrap(self, aw, ah): return (468, 40)
    def draw(self):
        self.canv.setStrokeColorRGB(0.2, 0.4, 0.8); self.canv.setLineWidth(2)
        self.canv.rect(0, 0, 468, 40)

ss = getSampleStyleSheet()
story = [Paragraph("Hello from lamina", ss["Title"]),
         Paragraph("This document was produced with ReportLab's PLATYPUS "
                   "layout engine: flowables are measured, wrapped and "
                   "paginated into frames automatically.", ss["BodyText"]),
         Spacer(1, 12), Box()]
SimpleDocTemplate(sys.argv[1] if len(sys.argv) > 1 else "reportlab.pdf",
                  pagesize=letter, title="Hello from lamina").build(story)
```

### 7.5 Results

| Approach | Lines | Output size | PDF header | xref form | `pdfinfo` | Notes |
|---|---|---|---|---|---|---|
| Raw writer (Python stdlib) | 41 | **983 B** | 1.7 | classic table | OK, 1 page | Flate content stream; standard-14 fonts |
| pdf-lib 1.17.1 | 18 | 1,129 B | 1.7 | **xref stream + object stream** | OK, 1 page | Object streams by default; random font resource names (`/Helvetica-9742682568`) |
| fpdf2 2.8.9 | 16 | 1,307 B | 1.3 | classic | OK, 1 page | Auto-wrapped, justified paragraph; top-left coordinates |
| ReportLab 5.1.0 | 19 | 1,763 B | 1.4 | classic | OK, 1 page | Layout engine chose positions; rectangle flowed below text |

The sizes are all within 2× of each other because none embeds a font. With a real embedded OpenType subset, each would grow by roughly 10–40 KB, and the quality of the font subsetter becomes the dominant size factor. None of the four outputs is tagged, and none would pass PDF/A or PDF/UA. That gap, more than raw generation, is where a new library can stand out.

---

## 8. Implications for lamina

> **This section is a recommendation for discussion, not a decision.** It draws on this chapter and the earlier ones.

### 8.1 What the landscape tells us

1. **The permissive, full-stack slot is empty.** The engines that do *everything* (parse, render, edit, generate, tag, validate) are either AGPL/commercial (MuPDF, iText, Ghostscript, and **since 6.5 also PDFsharp**) or proprietary (Apryse, Nutrient, Aspose, Adobe). Permissive projects each cover a slice: PDFium renders and edits but doesn't lay out or tag, and has no stable API; qpdf transforms structure; pdf.js renders and extracts in JS; PDFBox is Java-only; pdf-lib is stalled. **Licence drift is real**: QuestPDF moved to a revenue-gated licence in 2023 and PDFsharp to AGPL in 2026. A credible, permissively licensed, multi-language core has clear value, provided the project commits to keeping that licence.
2. **Accessibility and archival output are the gap.** Generation libraries outside iText, Typst/krilla, Prince, QuestPDF and WeasyPrint rarely produce tagged PDF/UA or valid PDF/A. Our own demo (§7) shows how easy it is to make a PDF and how far that is from a *compliant* one.
3. **Rust has become the place where modern permissive PDF infrastructure is built.** The Typst team's `pdf-writer`, `krilla`, `subsetter` and `hayro`, plus `lopdf`, the `skrifa`/`read-fonts` font stack, and HarfBuzz ports such as `rustybuzz`/`harfrust`, are all MIT/Apache, maintained in 2026, and compile to WASM without emscripten.
4. **One core, many front-ends is the proven pattern.** PDFium and MuPDF each serve every language through bindings. Typst runs as a CLI, a library and in the browser from one Rust codebase.

### 8.2 Candidate core languages

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| **A. Rust core** → native libs + `wasm32` + bindings (napi-rs for Node, PyO3/maturin for Python, UniFFI/C ABI for Swift/Kotlin/Go/.NET, wasm-bindgen for browser/edge) | Memory safety for hostile input (the main CVE source in C engines). Native speed. Real WASM without emscripten. Permissive ecosystem to build on (§8.3). One codebase for all targets | Higher contributor barrier than TS. Binding maintenance across 4–5 ecosystems. Compile times. Must build pieces that are missing in Rust (a mature layout engine outside Typst, full AcroForm/XFA, signatures) | **Recommended** |
| **B. TypeScript core** | Largest contributor pool; runs natively in browser, Node, Deno and edge; fast iteration | Weak for CPU-heavy work (rendering, image codecs, subsetting at scale). Hard to serve Python/Java/.NET users except by embedding a JS runtime. pdf-lib's stagnation shows the maintenance risk | Good for an SDK *layer*, weak as the core |
| **C. Wrap C/C++ engines** (PDFium for render/edit, qpdf for structure, HarfBuzz/FreeType for text) | Fastest route to broad feature coverage and top-tier rendering fidelity | No stable PDFium API; multi-MB WASM via emscripten; C/C++ memory-safety risk on untrusted input; thin glue gives little differentiation; build and packaging pain | Use **as a test oracle and optional backend**, not the core |
| **D. Go core** | Simple, great CLIs and servers, pdfcpu as prior art | Weak WASM (large binaries, GC) and FFI story; no browser story; small PDF ecosystem | Not recommended as core |
| **E. JVM / .NET core** | Mature prior art (PDFBox, iText, QuestPDF) | Not portable to browser/edge or to Python/JS without heavy runtimes | Not recommended |
| **F. Zig / C** | Small binaries, easy C ABI | Memory safety, small ecosystem | Not recommended |

### 8.3 Build versus wrap: which existing work to stand on

| Layer | Build | Wrap / depend on (permissive only) | Notes |
|---|---|---|---|
| Lexer/parser/xref recovery, object model, incremental save | **Build** (core IP; must be lossless and strict-mode capable) | Study `hayro-syntax`, `lopdf`, qpdf design | Code-generate typed dictionaries + validation from **Arlington** TSVs |
| Low-level writer | Depend | `pdf-writer` (MIT/Apache) | Or build a thin equivalent if the object model needs to share types |
| High-level generation (text, fonts, images, tags, PDF/A, PDF/UA) | Evaluate depending on, or contributing upstream to, `krilla` | `krilla`, `subsetter`, `skrifa`, `rustybuzz`/`harfrust` | krilla already has the tagging and validators lamina needs. Building a parallel stack would be wasteful |
| Layout engine (flowing text, tables, pagination) | Phase 2. Options: own engine, Typst as an embedded backend, or Taffy (flex/grid) + custom pagination | `typst` (Apache-2.0), `taffy` | Decide after the core ships. "Typst as the layout backend" is a strong option to evaluate |
| HTML/CSS → PDF | **Don't build** (a browser-grade CSS engine is out of scope) | Integrate with Chromium/Gotenberg/WeasyPrint as external converters; post-process their output (tag repair, PDF/A) | Post-processing is where lamina adds value |
| Rendering | Depend on `hayro` (contribute upstream); optional PDFium backend behind a feature flag | `hayro`, `pdfium-render` | PDFium doubles as a differential-testing oracle |
| Forms, annotations, signatures | Build on the core object model; crypto via RustCrypto / `cms` | — | See ch. 04 for signature requirements |
| Image codecs | Depend | `zune-jpeg`, `image`, `png`, hayro's JBIG2/JPX decoders | JPX/JBIG2 *encoding* is rarely needed |
| Validation | Build Arlington-driven syntax checks; **integrate** veraPDF (external, GPL/MPL) in CI rather than linking it | veraPDF CLI | Keep the GPL tool out of the shipped artefact |

### 8.4 Licensing traps to avoid

- **Do not link or vendor** MuPDF / PyMuPDF / mupdf.js / mupdf-rs, iText (incl. pdfHTML), Ghostscript, borb, Vivliostyle, or PDFsharp ≥ 6.5. All are AGPL (or AGPL/commercial). An AGPL dependency would force AGPL on lamina and its users' network services.
- **GPL**: Poppler, Xpdf and pdftk are fine as *external test tools* invoked as processes in CI, never as linked libraries in shipped artefacts. veraPDF (GPL/MPL dual) is likewise a CI tool.
- **LGPL** (fpdf2, OpenPDF, Cairo, PoDoFo): avoid statically linking into a permissive core. Mostly irrelevant if the core is Rust.
- **Revenue-gated / source-available** (QuestPDF) and **commercial** (UniPDF, Prince): not dependencies. Note that QuestPDF's own licence change is a cautionary tale for lamina's sustainability model.
- **Fonts and ICC profiles** are licensed too. Bundle only OFL/Apache fonts (e.g. Noto, Liberation) and freely redistributable ICC profiles (e.g. from the ICC or `colord` sets), and check each one.
- **Patents**: JBIG2/JPX are generally considered unencumbered now; LZW patents expired long ago. Low risk, but worth a legal skim.
- Recommended licence for lamina: **MIT OR Apache-2.0** dual licence (the Rust convention, compatible with every dependency listed in §8.3), with a CLA/DCO decision made up front.

### 8.5 Proposed architecture (for discussion)

```
                 ┌───────────────────────── lamina-core (Rust, no_std-friendly where possible) ─────────────────────────┐
                 │ syntax: lexer · parser · xref/recovery · filters · crypto(decrypt)                                   │
                 │ cos:    lossless object graph · lazy load · incremental + full writer · object streams              │
                 │ model:  typed accessors generated from Arlington · page tree · resources · outlines · names        │
                 │ content: content-stream parser/IR/encoder (shared by render, extract, edit, generate)              │
                 │ text:   font loading (skrifa) · shaping (rustybuzz/harfrust) · subsetting · ToUnicode              │
                 │ build:  high-level generation (krilla or own) · tagging · PDF/A · PDF/UA                            │
                 │ ops:    merge/split (with struct-tree + forms remap) · stamp · n-up · optimise · repair · redact   │
                 │ render: hayro adapter (default) · pdfium adapter (feature flag)                                     │
                 │ check:  Arlington syntax validator · conformance pre-checks                                         │
                 └──────────────┬──────────────┬──────────────┬──────────────┬──────────────┬─────────────────────────┘
                                │              │              │              │              │
                         lamina-wasm     lamina-node    lamina-python   lamina-ffi (C ABI)   lamina-cli
                         (browser/edge)  (napi-rs)      (PyO3)          → Swift/Kotlin/Go/.NET
                                │
                       @lamina/js (TS API, idiomatic; optional React/JSX layer later)
```

### 8.6 Recommendation

1. **Core in Rust**, dual MIT/Apache-2.0, first-class **WASM** and **TypeScript** packaging (the largest audience), plus Python early (the second-largest PDF audience, and the one most exposed to the PyMuPDF AGPL trap).
2. **First milestone: "lossless read/modify/write + processing".** Parsing with repair, incremental save, merge/split that preserves forms and the structure tree, stamping, optimisation and qpdf-class `--check`. This slot is the least well served under permissive licences, it underpins everything else, and the differentiators (correct struct-tree merge, strict mode) are tractable.
3. **Second milestone: generation with tagging and PDF/A/UA by default**, building on or contributing to `krilla` rather than forking it. Then layout, with "embed Typst" evaluated as an alternative to building an engine.
4. **Rendering via `hayro`**, contributing upstream. Offer PDFium as an optional backend and use it as a CI oracle.
5. **Testing infrastructure from week one:** Arlington-driven validation, veraPDF and qpdf in CI, a two-renderer visual diff, OSS-Fuzz-style fuzzing, and corpora (pdf.js, SafeDocs samples, veraPDF suites).

### 8.7 Open questions for the team

1. **Target users first:** web developers (TS-first, browser/edge) or backend/data teams (Python/CLI)? This decides which binding ships first.
2. **krilla:** depend on it, co-maintain with the Typst team, or build our own generation layer for tighter integration with an editable object model? (krilla is generation-only and does not read PDFs.)
3. **Layout scope:** do we build a document/layout engine at all, or position lamina as the PDF *substrate* (parse/edit/render/validate/post-process) under existing generators such as Typst, Chromium and WeasyPrint?
4. **HTML → PDF story:** officially integrate (wrapper over Chromium/Gotenberg with post-processing for tags and PDF/A), or leave it out?
5. **How much do we invest in rendering fidelity?** Is contributing to hayro enough, or do we need PDFium parity for a viewer product?
6. **Signatures and crypto:** pure-Rust (RustCrypto, `cms`) versus binding to platform crypto/HSMs for qualified signatures (ch. 04)?
7. **Governance and sustainability:** how do we avoid the QuestPDF/PDFsharp licence-drift pattern (foundation hosting, CLA versus DCO, funding model)?
8. **API stability:** given bindings to 4–5 ecosystems, what is the versioning and deprecation policy, and do we expose a low-level COS API publicly or keep it internal at first?
9. **Minimum supported PDF features for v1:** XFA (probably never), JBIG2/JPX decoding (yes for rendering), PDF 2.0-specific features (namespaces, UA-2), encryption write support (AES-256 only?).

---

## 9. References

**Registries (all versions in this chapter checked 2026-10-28)**
- npm registry: https://registry.npmjs.org/ (pdf-lib, @cantoo/pdf-lib, pdfkit, jspdf, pdfmake, @react-pdf/renderer, pdfjs-dist, puppeteer, playwright, @pdfme/generator, unpdf, mupdf, @embedpdf/pdfium, @hyzyla/pdfium, @libpdf/core, muhammara, @vivliostyle/core, @vivliostyle/cli, pagedjs, @sparticuz/chromium)
- PyPI JSON API: https://pypi.org/pypi/{name}/json (reportlab, fpdf2, weasyprint, pypdf, pikepdf, PyMuPDF, pypdfium2, pdfminer.six, pdfplumber, borb, xhtml2pdf, pyHanko, ocrmypdf)
- crates.io API: https://crates.io/api/v1/crates/{name} (lopdf, printpdf, pdf-writer, krilla, pdf, hayro, pdfium-render, typst, typst-pdf, mupdf, pdf_oxide, oxidize-pdf, genpdf)
- Maven Central: https://repo1.maven.org/maven2/ (org.apache.pdfbox:pdfbox, com.itextpdf:itext-core, com.github.librepdf:openpdf, org.xhtmlrenderer:flying-saucer-pdf, io.github.openhtmltopdf:openhtmltopdf-pdfbox, org.verapdf:validation-model-jakarta)
- NuGet: https://api.nuget.org/v3-flatcontainer/ (QuestPDF, PDFsharp, PDFsharp-MigraDoc, itext, PdfPig)
- Go module proxy: https://proxy.golang.org/ (gofpdf, go-pdf/fpdf, pdfcpu, unipdf/v4, gopdf, maroto/v2, go-pdfium, gotenberg/v8)
- Puppeteer `PDFOptions` typings (puppeteer-core 25.16.0, `lib/types.d.ts`): `tagged` default `true`, `outline` default `false`

**Licensing**
- PDFsharp licence (6.5.0+ dual AGPL/commercial): https://docs.pdfsharp.net/General/License/License.html ; FAQ https://docs.pdfsharp.net/General/License/License-FAQ.html ; repo LICENSE https://github.com/empira/PDFsharp/blob/master/LICENSE ; downstream reports https://github.com/Tharga/Reporter/issues/27
- QuestPDF licence and guide: https://www.questpdf.com/license/guide.html ; https://questpdf.com/pricing.html
- iText licensing: https://itextpdf.com/sales ; https://itextpdf.com/how-buy/legal/source-code-header
- Artifex (MuPDF/PyMuPDF/Ghostscript) licensing: https://artifex.com/licensing
- EmbedPDF PDFium WASM: https://www.embedpdf.com/docs/react/engine ; https://cdn.jsdelivr.net/npm/@embedpdf/pdfium@2.14.3/README.md

**Engines and projects**
- pdf.js: https://github.com/mozilla/pdf.js ; https://mozilla.github.io/pdf.js/
- PDFium: https://pdfium.googlesource.com/pdfium/ ; prebuilt binaries https://github.com/bblanchon/pdfium-binaries
- MuPDF: https://mupdf.com/ ; mupdf.js https://github.com/ArtifexSoftware/mupdf.js
- Poppler: https://poppler.freedesktop.org/ ; releases https://poppler.freedesktop.org/releases.html
- qpdf: https://qpdf.readthedocs.io/ ; https://github.com/qpdf/qpdf
- Ghostscript: https://ghostscript.com/ ; https://ghostscript.readthedocs.io/en/gs10.06.0/News.html
- hayro: https://github.com/LaurenzV/hayro ; https://docs.rs/hayro
- krilla: https://github.com/LaurenzV/krilla ; pdf-writer: https://github.com/typst/pdf-writer
- Typst: https://typst.app/ ; 0.14 blog https://typst.app/blog/2025/typst-0.14 ; 0.15 release https://github.com/typst/typst/releases/tag/v0.15.0 ; 0.15 RC notes https://forum.typst.app/t/typst-0-15-just-entered-its-final-testing-period-your-help-is-wanted/9047
- pdfium-render: https://github.com/ajrcarey/pdfium-render
- lopdf: https://github.com/J-F-Liu/lopdf ; printpdf: https://github.com/fschutt/printpdf ; pdf-rs: https://github.com/pdf-rs/pdf
- pdfcpu: https://pdfcpu.io/ ; https://github.com/pdfcpu/pdfcpu
- Gotenberg: https://gotenberg.dev/ ; https://pkg.go.dev/github.com/gotenberg/gotenberg/v8
- WeasyPrint: https://doc.courtbouillon.org/weasyprint/ ; manpage with PDF variants https://dyn.manpages.debian.org/unstable/weasyprint/weasyprint.1.en.html
- Prince: https://www.princexml.com/releases/16/ ; output docs https://princexml.com/doc/prince-output
- Vivliostyle: https://vivliostyle.org/ ; Paged.js: https://pagedjs.org/
- openhtmltopdf (community fork): https://github.com/openhtmltopdf/openhtmltopdf ; Flying Saucer: https://github.com/flyingsaucerproject/flyingsaucer
- Apache PDFBox: https://pdfbox.apache.org/ ; OpenPDF: https://github.com/LibrePDF/OpenPDF
- QuestPDF: https://www.questpdf.com/ ; PDFsharp: https://www.pdfsharp.com/
- ReportLab: https://docs.reportlab.com/ ; fpdf2: https://py-pdf.github.io/fpdf2/ ; pypdf: https://pypdf.readthedocs.io/ ; pikepdf: https://pikepdf.readthedocs.io/ ; PyMuPDF: https://pymupdf.readthedocs.io/ ; pdfplumber: https://github.com/jsvine/pdfplumber ; pyHanko: https://pyhanko.readthedocs.io/ ; OCRmyPDF: https://ocrmypdf.readthedocs.io/
- pdf-lib: https://github.com/Hopding/pdf-lib ; Cantoo fork https://github.com/cantoo-scribe/pdf-lib ; PDFKit: https://pdfkit.org/ ; pdfmake: https://pdfmake.github.io/docs/ ; react-pdf: https://react-pdf.org/ ; pdfme: https://pdfme.com/ ; unpdf: https://github.com/unjs/unpdf
- Skia PDF backend: https://skia.org/docs/user/sample/pdf/ ; Cairo PDF surface: https://www.cairographics.org/manual/cairo-PDF-Surfaces.html
- Apple PDFKit: https://developer.apple.com/documentation/pdfkit
- Chromium headless PDF / tagged PDF: https://blog.chromium.org/2020/07/using-chrome-to-generate-more.html ; DevTools `Page.printToPDF`: https://chromedevtools.github.io/devtools-protocol/tot/Page/#method-printToPDF
- CSS Paged Media: https://www.w3.org/TR/css-page-3/ ; CSS GCPM: https://www.w3.org/TR/css-gcpm-3/ ; CSS Fragmentation: https://www.w3.org/TR/css-break-3/
- LibreOffice PDF export options: https://help.libreoffice.org/latest/en-US/text/shared/guide/pdf_params.html
- Pandoc PDF engines: https://pandoc.org/MANUAL.html#option--pdf-engine
- LaTeX Tagging Project: https://latex3.github.io/tagging-project/

**Validation, corpora, fuzzing**
- veraPDF: https://verapdf.org/ ; https://github.com/veraPDF/veraPDF-library
- Arlington PDF Model: https://pdfa.org/resource/arlington-pdf-model/ ; https://github.com/pdf-association/arlington-pdf-model ; SafeDocs research summary https://pdfa.org/safedocs-latest-research-into-securing-pdf/
- Matterhorn Protocol: https://pdfa.org/resource/the-matterhorn-protocol/
- PDF Association sample files: https://pdfa.org/resource/pdf-2-0-examples/ (and https://github.com/pdf-association/pdf20examples)
- GovDocs1: https://digitalcorpora.org/corpora/file-corpora/files/
- SafeDocs CC-MAIN-2021-31-PDF-UNTRUNCATED: https://digitalcorpora.org/corpora/file-corpora/cc-main-2021-31-pdf-untruncated/
- OSS-Fuzz: https://github.com/google/oss-fuzz (projects: pdfium, poppler, mupdf, qpdf, ghostscript, xpdf)
- pdf.js CVE-2024-4367: https://codeanlabs.com/blog/research/cve-2024-4367-arbitrary-js-execution-in-pdf-js/
- ISO 32000-2:2020 (PDF 2.0) and ISO 19005 (PDF/A), ISO 14289 (PDF/UA): via https://pdfa.org/sponsored-standards/ (no-cost PDF 2.0 access)

---

## Verification log

**2026-10-28 — spec fact-check pass (PDF-format claims only).** Sources: ISO 32000-1:2008 (PDF 1.7) full text. Library versions, licences, benchmarks and tool behaviour were **out of scope** for this pass and were not re-verified.

Corrections (was → now):

- §3.5 Linearisation: "the first page's objects come first, followed by a linearisation dictionary and hint tables" → the file starts with the linearisation parameter dictionary (within the first 1024 bytes), then the first-page xref section, catalog/document-level objects and the primary hint stream, and only then the first page's objects (ISO 32000-1 F.3, F.3.3).
- §3.5: "any incremental update breaks linearisation" kept, with the spec wording added — updates remain permitted, but the result is no longer linearized (F.1).
- §3.3 Watermark annotation: "`/FixedPrint`, so it can be hidden or removed" → `/FixedPrint` is optional and controls fixed-size/fixed-position printing; hideability comes from being an annotation (§12.5.6.22, Tables 190–191).

Confirmed without change: inheritable page attributes `/Resources /MediaBox /CropBox /Rotate` (§7.7.3.4, Table 30); `/PageLabels` number tree keyed by zero-based page index (§12.4.2); `/StructParents`/`/StructParent` → `/ParentTree`, `/RoleMap`, `/ClassMap` (§14.7); same fully-qualified field name ⇒ one shared value (§12.7.3.2); `/AcroForm /Fields /DR /NeedAppearances` (Table 218); Artifact `/Type /Pagination /Subtype /Watermark` (Table 330); object streams and xref streams PDF 1.5 (§7.5.7–7.5.8); hint tables = page offset and shared object (F.4); inline images `BI … ID … EI`, filters and `/JBIG2Globals` (§7.4, §8.9.7); classic xref entries are 20 bytes and the demo's header/xref/trailer layout (§7.5.2–7.5.5); incremental updates append after `%%EOF` with `/Prev` (§7.5.6).

Still unverified: PDF/A, PDF/UA and PDF 2.0 claims (namespaces, Arlington coverage) — other standards, not in the sources used; renderer architecture and viewer behaviour (§3.5 range requests, §4); all library versions/licences (out of scope).
