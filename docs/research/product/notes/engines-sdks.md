# PDF rendering engines and SDKs: a 2026 comparison for lamina

Scope: rendering engines, SDKs and Rust crates that matter to lamina (Rust core, MIT OR Apache-2.0, no AGPL/GPL, core crates bytes-in/bytes-out for wasm32, iOS and Android). Research date: 2026-10-29.

Network limits: docs.rs, www.embedpdf.com and apryse.com could not be reached from this session (DNS blocked). Facts from those hosts come only from search-result snippets and are marked as such. These notes cover about 20 searches and fetches, so many per-engine spec-coverage cells are still gaps (see each Gaps section).

**Legend.** "Measured" means a number produced by a benchmark run (the author's own or a third party's). "Vendor claim" means a statement from the project or vendor that has not been checked independently. "Third-party estimate" means aggregator or buyer data.

### Summary table

| Engine | Licence (lamina-compatible?) | Cost | Size / cold start | Memory / mobile | Spec coverage notes | Incremental save | Maturity 2025–26 |
|---|---|---|---|---|---|---|---|
| **PDFium** (upstream; [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries); [EmbedPDF](https://github.com/embedpdf/embed-pdf-viewer) fork) | Upstream is Apache-2.0 per [EmbedPDF](https://github.com/embedpdf/embed-pdf-viewer). pdfium-binaries build scripts are MIT ([repo](https://github.com/bblanchon/pdfium-binaries)). **Yes** | Free | pdfium.wasm about 2.7 MB gzipped (one anecdotal report, [HN mirror](https://hn.nuxt.dev/item/44126177)). WASM compilation takes hundreds of ms at cold start ([pdfium.js docs, snippet](https://pdfium.js.org/docs/init-library)) | Some WASM builds have a non-growable heap and hit an unrecoverable OOM on long documents ([pdfium-render](https://github.com/ajrcarey/pdfium-render)). Not thread-safe: calls are serialised behind a mutex (same source) | Forms, annotations, signature introspection ([pdfium-render](https://github.com/ajrcarey/pdfium-render)). XFA needs V8 builds, and no V8 WASM build exists ([pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)) | Save added in pdfium-render 0.7.0; incremental save not mentioned ([pdfium-render](https://github.com/ajrcarey/pdfium-render)) | Active. pdfium-render 0.9.4 targets PDFium 7881. pdfium-binaries ships WASM as "experimental" |
| **pdfium-render** (Rust crate) | Licence file linked but not named on the page ([repo](https://github.com/ajrcarey/pdfium-render)) | Free | Loads PDFium dynamically, so a separately packaged PDFium WASM is needed | 0.9.4 fixed double-frees and a WASM memory-growth edge case | Same as PDFium | See PDFium | 717 stars, 562 commits |
| **MuPDF / mupdf.js** | AGPL or commercial ([MuPDF licence](https://mupdf.readthedocs.io/en/1.27.0/license.html)). **No** | Commercial: quote only, no public price ([search summary](https://www.file2markdown.ai/blog/is-pymupdf-free-for-commercial-use)) | npm `mupdf` 1.26.4 unpacked: 13.5 MB ([npm](https://npmjs.com/package/mupdf)). WASM about 8–12 MB per an unverified FAQ ([mupdfjs FAQ](https://mupdfjs.readthedocs.io/en/latest/faq/index.html)) | Not found | Not verified here | Not verified here | Active (1.26/1.27) |
| **PDF.js** | Apache-2.0 (not checked this session; see Gaps) | Free | Not measured | Draws a page onto one canvas. iOS Safari limits: about 16.7 Mpx per canvas and 224–384 MB total canvas memory ([pqina](https://pqina.nl/blog/canvas-area-exceeds-the-maximum-limit/), [pqina](https://www.pqina.nl/blog/total-canvas-memory-use-exceeds-the-maximum-limit/), [Apple forum](https://developer.apple.com/forums/thread/112218)). Apryse says it struggles with blueprints and maps (vendor claim) ([Apryse, snippet](https://apryse.com/blog/pdf-js/pdf-js-reliability-benchmark)) | XFA since 2.8.335, on by default since 2.11.338 ([2.8.335](https://newreleases.io/project/npm/pdfjs-dist/release/2.8.335), [2.11.338](https://newreleases.io/project/npm/pdfjs-dist/release/2.11.338)). Annotation editor since 2.15.349 ([notes](https://newreleases.io/project/npm/pdfjs-dist/release/2.15.349)) | Not verified | Very active: 5.x in 2025 ([5.0.375](https://newreleases.io/project/npm/pdfjs-dist/release/5.0.375)); comment popup added Sept 2025 ([bug 1988516](https://bugzilla.mozilla.org/show_bug.cgi?id=1988516)) |
| **Poppler** | GPL (not sourced this session). **No** | Free | – | – | – | – | Its text tool appears in the py-pdf benchmark (pdftotext) |
| **Apryse WebViewer** | Proprietary. **No** (as a dependency) | Quote only. Median buyer pays $25,140/yr (n=39; range $6.9k–$92.6k) ([Vendr](https://www.vendr.com/marketplace/apryse)). "From $1,500" ([Verdocs](https://www.verdocs.com/blog/apryse-pricing)). Base tier includes 5,000 doc events/yr ([Apryse page, Fall 2024](https://apryse.com/pricing/webviewer)) | Not found | Canvas-memory-limit blank pages reported on WebViewer 10.7.3 ([Apryse forum](https://community.apryse.com/t/exceed-canvas-memory-limit-make-pages-document-render-as-blank-pages/10160)) | Not verified | Not verified | Summer 2026 release ([blog](https://apryse.com/blog/embed-secure-document-viewer)) |
| **Nutrient (PSPDFKit)** | Proprietary. **No** | Quote only. "From EUR 5,000" ([Capterra](https://www.capterra.com/p/171174/PSPDFKit-SDK/pricing/)). Web about $10–15k/yr per a competitor (biased) ([SimplePDF](https://simplepdf.com/alternatives/pspdfkit)). Reviewers report multi-year lock-in ([Capterra](https://www.capterra.com/p/171174/PSPDFKit-SDK/)) | Not found | Not found | Not verified | Not verified | Active |
| **Foxit PDF SDK** | Proprietary. **No** | Not published. $3,000 on-prem plan listed ([TrustRadius](https://tr-web.dev.trustradius.com/products/foxit-pdf-sdk/pricing)). Web SDK reseller SKUs show no price ([SHI](https://www.shi.com/product/48964840/SERVER-DISTRIBUTION-FEE-FOR-PDF-SDK-FOR-WEB)) | Not found | Not found | Not verified | Not verified | Active |
| **hayro** (LaurenzV) | MIT OR Apache-2.0 ([repo](https://github.com/LaurenzV/hayro)). **Yes** | Free | CPU-only, no GPU dependency ([crate listing](https://crates.io/crates/hayro)) | Timeouts and OOM still possible on untrusted input ([repo](https://github.com/LaurenzV/hayro)) | Own JBIG2, JPX, CCITT and CMap crates. Missing: knockout groups, CMYK blending, 3D ([repo](https://github.com/LaurenzV/hayro)). An older README listed encryption and blend modes as unsupported too ([note](https://histre.com/public/notes/reorx/note/hgyjigoo)) | Read-only (no editing) | 0.7.x; first published 2025-07-22; about 1,992 commits, 828 stars |
| **pdf_oxide** | MIT OR Apache-2.0, plus trademark limits on the name ([repo](https://github.com/yfedoseev/pdf_oxide)). **Yes** | Free | npm `pdf-oxide-wasm` | – | Mainly extraction; no rendering API documented | `SaveOptions::incremental()` exists | v0.3.70; 1.1k stars; 342 open issues |
| **lopdf** | MIT (except Montserrat font) ([repo](https://github.com/J-F-Liu/lopdf)). **Yes** | Free | Partial wasm notes | – | Decrypts empty passwords only. Object streams and xref streams supported | Not mentioned | about 2.3k stars; active |
| **pdf-rs** | MIT ([repo](https://github.com/pdf-rs/pdf)). **Yes** | Free | – | – | Writing is "experimental". Renders through the separate pdf_render (Pathfinder) | Not mentioned | 868 commits; 36 issues |
| **krilla** (generation) | MIT and Apache-2.0 ([repo](https://github.com/LaurenzV/krilla)). **Yes** | Free | – | – | OpenType including colour fonts; CFF/TTF subsetting; tagged PDF; PDF/A-1 to A-4, PDF/UA-1; PDF 1.4–2.0 | Creation only; points to pdf-rs for editing | MSRV 1.92; builds on pdf-writer |

---

## Licence and commercial cost

### Takeaway
Six options fit lamina's licence rules: PDFium (with its binary builds and pdfium-render), hayro, krilla, lopdf, pdf-rs and pdf_oxide. PDF.js probably fits too, but its licence was not checked this session. MuPDF is AGPL-or-commercial. Poppler is GPL (unsourced here). Apryse, Nutrient and Foxit are proprietary and quote-only, with buyer data putting Apryse at a median of about $25k a year.

### Cited Findings
- EmbedPDF Runtime is a fork of Google's PDFium, "which is Apache-2.0 licensed". The EmbedPDF repo is Apache-2.0 except `cloudpdf/server`, which is FCL-1.0-ALv2 and becomes Apache-2.0 two years after each release — [EmbedPDF](https://github.com/embedpdf/embed-pdf-viewer)
- pdfium-binaries is MIT-licensed and "isn't affiliated with Google or Foxit" — [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)
- MuPDF is dual-licensed AGPL or Artifex commercial — [MuPDF docs](https://mupdf.readthedocs.io/en/1.27.0/license.html). Commercial pricing is set per project by deployment size, revenue and use case, through sales — [file2markdown summary](https://www.file2markdown.ai/blog/is-pymupdf-free-for-commercial-use) (secondary source)
- Apryse: quote-only. Vendr buyer data gives a median of $25,140/yr from 39 purchases, ranging from $6,913 to $92,641 (third-party estimate) — [Vendr](https://www.vendr.com/marketplace/apryse). Packages start "as low as $1,500" — [Verdocs](https://www.verdocs.com/blog/apryse-pricing). SpendHound tracks $43,260/yr, not specific to WebViewer — [SpendHound](https://www.spendhound.com/marketplace/apryse-pricing). WebViewer Base includes 5,000 document events/yr (page labelled Fall 2024) — [Apryse pricing](https://apryse.com/pricing/webviewer)
- Nutrient/PSPDFKit: quote-based, starting at EUR 5,000 per Capterra — [Capterra](https://www.capterra.com/p/171174/PSPDFKit-SDK/pricing/). A competitor estimates $10k–15k/yr for web-only, with Document Engine and mobile extra (biased source) — [SimplePDF](https://simplepdf.com/alternatives/pspdfkit). Reviewers complain of multi-year lock-in — [Capterra](https://www.capterra.com/p/171174/PSPDFKit-SDK/)
- Foxit: no published SDK price. TrustRadius lists a $3,000 on-prem "Licensing PDF SDK" plan — [TrustRadius](https://tr-web.dev.trustradius.com/products/foxit-pdf-sdk/pricing). SHI lists web SDK SKUs (technology access fee, server distribution fee) with no price — [SHI](https://www.shi.com/product/48964840/SERVER-DISTRIBUTION-FEE-FOR-PDF-SDK-FOR-WEB), [SHI TA](https://www.publicsector.shidirect.com/product/44587433/FOXIT-PDF-SDK-TECHNOLOGY-ACCESS-FEE-FOR-PDF-SDK-FOR-WEB)
- hayro: Apache-2.0 or MIT — [hayro](https://github.com/LaurenzV/hayro). krilla: MIT and Apache-2.0 — [krilla](https://github.com/LaurenzV/krilla). lopdf: MIT, except the bundled Montserrat font — [lopdf](https://github.com/J-F-Liu/lopdf). pdf-rs: MIT — [pdf-rs](https://github.com/pdf-rs/pdf)
- pdf_oxide is MIT OR Apache-2.0, but "PDFOxide" is the author's trademark and forks must not reuse the name — [pdf_oxide](https://github.com/yfedoseev/pdf_oxide)
- Licences of the libraries in pdf_oxide's benchmark table: PyMuPDF AGPL-3.0, pypdfium2 Apache-2.0, pdftext GPL-3.0, pdfminer MIT, pypdf BSD-3 — [pdf_oxide](https://github.com/yfedoseev/pdf_oxide)

### Inferences
- The commercial SDKs only matter to lamina as competitors or benchmarks, since lamina cannot depend on them. Their price ($5k–$90k/yr, quote-only, multi-year contracts) leaves room for a free, permissively licensed alternative.
- The MIT licence of pdfium-binaries covers its build scripts. The PDFium code inside the binaries keeps its own licence. lamina should check PDFium's upstream LICENSE and third-party notices (FreeType, libjpeg-turbo, OpenJPEG, lcms2 and others) during the C1b spike.
- The Montserrat font in lopdf is an extra licence item if lopdf is ever pulled in.

### Gaps
- I did not fetch the upstream PDFium LICENSE file. The Apache-2.0 claim comes only from EmbedPDF. My background understanding is a BSD-3-style licence plus third-party code; this needs checking.
- The licences of PDF.js (Apache-2.0) and Poppler (GPL-2/3) are from background knowledge and were not fetched this session.
- No official published price exists for Apryse, Nutrient, Foxit or MuPDF commercial; all figures above are third-party.

## Size, cold start, memory, threading and mobile OOM

### Takeaway
The only size figures found are anecdotal: PDFium WASM at about 2.7 MB gzipped, and a MuPDF npm package of 13.5 MB unpacked. The main mobile risks are WASM builds with a non-growable heap (PDFium, per pdfium-render) and iOS Safari's canvas caps, which hit any canvas renderer (PDF.js, Apryse) on large pages. This supports tiled rendering into fixed-size bitmaps.

### Cited Findings
- pdfium.wasm is about 2.7 MB gzipped and the main JS about 77 kB in one viewer's build (anecdote, not measured by us) — [HN mirror](https://hn.nuxt.dev/item/44126177)
- WASM compilation is the expensive part of cold start, at hundreds of ms (doc snippet) — [pdfium.js docs](https://pdfium.js.org/docs/init-library). Packages such as @hyzyla/pdfium ship the wasm bundled — [@hyzyla/pdfium README](https://cdn.jsdelivr.net/npm/@hyzyla/pdfium@2.1.13/README.md)
- pdfium-render warns that some PDFium WASM builds use a "non-growable WASM heap memory allocator", so long documents can cause unrecoverable OOM. It recommends the paulocoutinhox builds instead. PDFium is treated as not thread-safe, and the `thread_safe` feature serialises calls behind a mutex. V8 and win32 features are unavailable in WASM — [pdfium-render](https://github.com/ajrcarey/pdfium-render)
- pdfium-binaries lists WebAssembly as "experimental" and offers no V8 build for wasm. Android (arm, arm64, x86, x64) and iOS (device, simulator, Catalyst) are built — [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)
- The npm `mupdf` package (1.26.4) is 13.5 MB unpacked — [npm](https://npmjs.com/package/mupdf). An unverified FAQ says the WASM is 8–12 MB plus a 50 KB wrapper — [mupdfjs FAQ](https://mupdfjs.readthedocs.io/en/latest/faq/index.html)
- iOS Safari caps one canvas at 16,777,216 pixels — [pqina](https://pqina.nl/blog/canvas-area-exceeds-the-maximum-limit/). Total canvas memory is capped at 384 MB on iOS 15 and lower on earlier versions — [pqina](https://www.pqina.nl/blog/total-canvas-memory-use-exceeds-the-maximum-limit/). "Total canvas memory use exceeds the maximum limit (224 MB)" was reported on iOS 12 — [Apple forum](https://developer.apple.com/forums/thread/112218)
- Apryse (a competitor) says PDF.js renders page content onto one large canvas, which can exceed browser limits, and struggles with "design documents, maps, and blueprints, especially on mobile browsers" (vendor claim, 2019 benchmark; search snippet only) — [Apryse blog](https://apryse.com/blog/pdf-js/pdf-js-reliability-benchmark)
- Apryse WebViewer 10.7.3 users report pages rendering blank once the canvas memory limit is exceeded — [Apryse forum](https://community.apryse.com/t/exceed-canvas-memory-limit-make-pages-document-render-as-blank-pages/10160)
- hayro renders on the CPU with no GPU dependency — [crate listing](https://crates.io/crates/hayro). Timeouts and OOMs are still possible on untrusted input, glyph caching is not implemented yet, and "many PDFs render faster than in other viewers" (vendor claim, no figures) — [hayro](https://github.com/LaurenzV/hayro)

### Inferences
- lamina's `RenderBackend` should take a tile (page, scale, clip rect) and return a bounded bitmap, so the backend never allocates a full-page canvas above iOS limits.
- With PDFium-WASM, lamina should choose (or build) a growable-heap build and run one PDFium instance per worker for parallelism, since PDFium itself is single-threaded.
- hayro's pure-Rust design lets it run under `Limits` (ADR 0003) like the rest of the core. Its own OOM and timeout risk on untrusted input means lamina should run it inside a worker with a watchdog.

### Gaps
- No measured cold-start times or WASM sizes for EmbedPDF's `@embedpdf/pdfium` (embedpdf.com was unreachable), hayro-in-wasm, PDF.js bundle size, or Apryse, Nutrient and Foxit WebViewer payloads.
- No measured memory figures on 1,000+ page books or vector-heavy blueprints for any engine. The C1a and C1b spikes should measure these.

## Spec coverage (XFA, CMYK/ICC, overprint, blend modes, Type3/CID, JBIG2/JPX, encryption, annotations/forms)

### Takeaway
PDFium (with V8/XFA builds) and PDF.js are the only free options found to handle XFA, and PDFium's XFA needs V8, which has no WASM build. hayro covers the image codecs with its own crates but lacks knockout groups and CMYK blending (and earlier encryption). That fits the plan to keep it as the alternative rather than the default.

### Cited Findings
- PDF.js: initial XFA support in 2.8.335 — [release](https://newreleases.io/project/npm/pdfjs-dist/release/2.8.335); XFA enabled by default in 2.11.338 — [release](https://newreleases.io/project/npm/pdfjs-dist/release/2.11.338); experimental annotation editor in 2.15.349 — [release](https://newreleases.io/project/npm/pdfjs-dist/release/2.15.349); 5.1.91 improved SMask rendering and font conversion — [release](https://newreleases.io/project/npm/pdfjs-dist/release/5.1.91)
- PDFium V8 builds (needed for XFA/JS) are offered for native platforms, but not for WebAssembly — [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries). `pdfium_enable_v8` is unsupported in WASM — [pdfium-render](https://github.com/ajrcarey/pdfium-render)
- pdfium-render exposes form field introspection and filling, annotation rendering and creation, signature introspection, and attachments — [pdfium-render](https://github.com/ajrcarey/pdfium-render)
- EmbedPDF adds highlight, sticky-note, free-text and ink annotations and "true redaction" that removes underlying content — [EmbedPDF](https://github.com/embedpdf/embed-pdf-viewer)
- hayro ships its own JPEG2000, JBIG2, CCITT, CMap and PostScript decoders. Knockout groups, CMYK blending and 3D are excluded. Annotations and forms are out of scope for the demo viewer — [hayro](https://github.com/LaurenzV/hayro). An earlier README also listed encrypted PDFs and blend modes as unsupported (date uncertain, probably superseded) — [histre note](https://histre.com/public/notes/reorx/note/hgyjigoo)
- lopdf decrypts only empty-password files — [lopdf](https://github.com/J-F-Liu/lopdf)

### Inferences
- XFA in lamina on the web would come only from PDF.js or a non-WASM PDFium-V8 build. Since XFA is deprecated in PDF 2.0, a "not supported, show the fallback page" warning is probably acceptable (recommendation).
- Overprint and CMYK/ICC fidelity is where hayro is weakest and PDFium is reasonable. lamina's own corpus should include output-intent and overprint files to measure this.

### Gaps
- Per-feature coverage of overprint, ICC, Type3 and CID for PDFium, MuPDF, Apryse, Nutrient and Foxit was not checked this session. Vendor feature matrices were not fetched.
- hayro's current encryption status is unclear. The current README did not mention it, and docs.rs was unreachable.

## Incremental save and signature preservation

### Takeaway
None of the open-source renderers found documents an append-only incremental save that preserves signatures. pdf_oxide claims a `SaveOptions::incremental()` API. This confirms lamina's own save planner (ADR 0007) as a differentiator.

### Cited Findings
- pdfium-render added document saving in 0.7.0; incremental saving is not mentioned. It offers signature introspection, not signing — [pdfium-render](https://github.com/ajrcarey/pdfium-render)
- pdf_oxide's Rust example uses `save_with_options(SaveOptions::incremental())`, but the README gives no further detail — [pdf_oxide](https://github.com/yfedoseev/pdf_oxide)
- lopdf supports `save_modern()` with object and xref streams (11–61% smaller files, its own claim); incremental save is not mentioned — [lopdf](https://github.com/J-F-Liu/lopdf)
- pdf-rs says modifying and writing are experimental — [pdf-rs](https://github.com/pdf-rs/pdf)
- hayro's parsing layer is read-only — [hayro](https://github.com/LaurenzV/hayro). krilla only creates documents and points to pdf-rs for manipulation — [krilla](https://github.com/LaurenzV/krilla)

### Inferences
- Keeping the renderer read-only and owning all writes in lamina's save planner keeps signature bytes safe. lamina should never let PDFium save the document (recommendation).

### Gaps
- Commercial SDK incremental-save and signature-preservation behaviour was not verified (vendor docs not fetched). Upstream PDFium's `FPDF_SaveAsCopy` incremental flag was not verified from a primary source this session.

## Text extraction accuracy (benchmarks)

### Takeaway
Two benchmarks exist, both self-run with small or opaque methods. In py-pdf's 14-document benchmark, pypdfium2 (PDFium) and pypdf tie at 95% quality and PyMuPDF scores 94%. pdf_oxide's 3,830-file suite reports pass rates, not accuracy: pdf_oxide 100%, PyMuPDF 99.3%, pypdfium2 99.2%. Neither benchmark measures reading order or ligatures directly.

### Cited Findings
- py-pdf benchmarks, quality: pypdfium2 95%, pypdf 95%, PyMuPDF 94%, Tika 93%, pdftotext (Poppler) 90%, pdfminer.six 88%, pdfplumber 74%. Speed: PyMuPDF and pypdfium2 0.1 s, Tika and pdftotext 0.3 s, pypdf 3.3 s, pdfminer 5.6 s, pdfplumber 8.9 s. Corpus: 14 PDFs, mostly arXiv plus a 117-page book, on an i7-6700HQ. The quality metric is not explained, pdf.js is not included, and no update date is shown — [py-pdf/benchmarks](https://github.com/py-pdf/benchmarks)
- pdf_oxide benchmark (the author's own, single-threaded, 60 s timeout, no warm-up, 3,830 PDFs: veraPDF 2,907, pdf.js 897, SafeDocs 26), mean / p99 / pass rate:

  | Library | Mean | p99 | Pass rate |
  |---|---|---|---|
  | pdf_oxide | 0.8 ms | 9 ms | 100% |
  | PyMuPDF | 4.6 ms | 28 ms | 99.3% |
  | pypdfium2 | 4.1 ms | 42 ms | 99.2% |
  | pdfminer | 16.8 ms | 124 ms | 98.8% |
  | pypdf | 12.1 ms | 97 ms | 98.4% |
  | oxidize_pdf (Rust) | 13.5 ms | – | 99.1% |
  | pdf_extract (Rust) | 4.08 ms | – | 91.5% |
  | lopdf (Rust) | – | – | 80.2% |

  The "100%" excludes 7 deliberately broken files. The author also claims "99.5% text parity" with PyMuPDF and pypdfium2 — [pdf_oxide](https://github.com/yfedoseev/pdf_oxide)

### Inferences
- A "pass rate" means the library extracted something without failing; it does not mean the text was correct. For reading order, ligatures and ToUnicode, lamina needs its own ground-truth corpus. Using PDFium's text output as a reference (via parity, as pdf_oxide does) is a cheap first signal.
- The pdf_oxide corpus mix (veraPDF, pdf.js, SafeDocs) is freely available and reusable for lamina's corpus tests (ADR 0018).

### Gaps
- No independent benchmark found that scores reading order, ligature decomposition or ToUnicode fallback across engines. The "PDF text extraction comparisons" posts were not found in this pass.

## Rendering correctness benchmarks and test suites

### Takeaway
The only rendering-correctness corpus found is hayro's own visual regression suite of 1,600+ PDFs, drawn largely from the pdf.js and PDFBox test suites. No published cross-engine pass rates were found. The author's claim to be the most correct Rust renderer is a self-assessment.

### Cited Findings
- hayro: visual regression suite of 1,600+ PDFs, many from the pdf.js and PDFBox suites. "Most PDFs that render correctly in Chrome (PDFium) or Firefox (pdf.js) should also render correctly in hayro" (vendor claim). No comparison numbers are published — [hayro](https://github.com/LaurenzV/hayro). An older description says the suite had over 1,000 files — [crate listing summary](https://crates.io/crates/hayro)
- Apryse published a "pdf.js reliability benchmark" (vendor, competitor). Only the snippet was available — [Apryse](https://apryse.com/blog/pdf-js/pdf-js-reliability-benchmark)

### Inferences
- lamina's C1a (hayro parity) spike can reuse the pdf.js and PDFBox-derived corpus and compare hayro against PDFium pixel-diffs. That would be the first published cross-engine number.

### Gaps
- PDF Association test suites and pdf.js reftest pass statistics were not fetched in this pass.
- Apryse's benchmark figures could not be read (apryse.com blocked).

## Maturity and maintenance, 2025–2026

### Takeaway
PDF.js (5.x line), PDFium (pdfium-render tracks build 7881), EmbedPDF (v3 in development) and hayro (0.7.x since July 2025) are all actively maintained. pdf_oxide is new and fast-moving but pre-1.0, with 342 open issues. krilla is stable and is the generation reference.

### Cited Findings
- EmbedPDF: about 2,853 commits, 4.6k stars, 200 open issues. v3 is "not yet recommended for production use"; v2 is the stable line — [EmbedPDF](https://github.com/embedpdf/embed-pdf-viewer). An `@embedpdf/pdfium` 2.14.3 exists on npm — [jsDelivr](https://cdn.jsdelivr.net/npm/@embedpdf/pdfium@2.14.3/package.json)
- pdfium-render 0.9.4 defaults to PDFium 7881, with versioned features back to 5961 — [pdfium-render](https://github.com/ajrcarey/pdfium-render)
- hayro: first published 2025-07-22, latest version 0.7.1 per search snippets — [rust-digger](https://crates.io/crates/hayro). About 1,992 commits, 828 stars, MSRV 1.92, and the API surface is "still being settled" — [hayro](https://github.com/LaurenzV/hayro). Downstream users include zenpdf and gpui-pdf (search snippets) — [search via docs.rs listings](https://docs.rs/zenpdf)
- pdf_oxide v0.3.69/0.3.70; 1.1k stars; copyright 2025–2026; bindings for 19 languages — [pdf_oxide](https://github.com/yfedoseev/pdf_oxide)
- PDF.js 5.0.375 was a major release with API changes in 2025 — [newreleases](https://newreleases.io/project/npm/pdfjs-dist/release/5.0.375). A comments popup was added to the editor in September 2025 — [Bugzilla 1988516](https://bugzilla.mozilla.org/show_bug.cgi?id=1988516). Later 2026 versions (6.x) are cited by Wikipedia but unverified — [Wikipedia](https://en.wikipedia.org/wiki/PDF.js)
- Apryse shipped a Summer 2026 release — [Apryse blog](https://apryse.com/blog/embed-secure-document-viewer)

### Inferences
- PDFium-WASM as the default and hayro as the alternative still looks right for 2026. hayro is the strongest pure-Rust candidate, but its API churn suggests pinning exact versions behind the `RenderBackend` trait.

### Gaps
- Exact last-commit dates for pdf-rs and lopdf were not shown on their pages. The current hayro version could not be confirmed (docs.rs blocked).
