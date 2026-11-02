# Differentiation opportunities and MVP scoping for lamina (as of Oct 2026)

Scope: which 2–3 differentiators could give users a reason to switch, and what a zero-bloat MVP should contain, for lamina: a MIT/Apache Rust-core PDF platform with a privacy-first web reader on Cloudflare (no uploads), a Tauri 2 desktop app, a later full editor, and a JS/Rust generation library. Items marked **(opinion)** are recommendations, not sourced facts. Licences flagged ⚠ are copyleft (GPL/AGPL) or restrictive.

## Q1. On-device OCR compatible with a permissive licence and WASM/mobile

### Takeaway
The cleanest pure-Rust, permissive option is **ocrs** (MIT/Apache-2.0, ONNX models run on the RTen engine), but it is an early preview and handles **Latin script only**. **PP-OCRv5 mobile** ONNX models (Apache-2.0, about 20 MiB for detection plus recognition) cover many more scripts. **Tesseract** (Apache-2.0) is the mature WASM fallback. On mobile, the platform APIs (Apple Vision, ML Kit) are free, on-device and better quality, so the native shells should use them. OCR is a strong later-phase feature, not a v0.1 one.

### Cited Findings
- ocrs is dual-licensed MIT and Apache-2.0 (LICENSE-APACHE.txt and LICENSE-MIT.txt in the repo). Its README says it "is currently in an early preview. Expect more errors than commercial OCR engines." It recognizes only the Latin alphabet; other languages are planned. Its models are trained in PyTorch, exported to ONNX and run with RTen, and are trained on "open and liberally licensed datasets" — [robertknight/ocrs](https://github.com/robertknight/ocrs)
- ocrs documents an alpha WebAssembly build (`make wasm`) — [docs.rs ocrs](https://docs.rs/crate/ocrs/latest); package listings confirm MIT + Apache-2.0 — [AUR ocrs-git](https://aur.archlinux.org/packages/ocrs-git)
- A third-party WASM wrapper of ocrs (locr) is MIT, but its bundled models are **CC-BY-SA 4.0** (share-alike). Check model licences separately from code licences — [KevRojo/locr](https://github.com/KevRojo/locr)
- tesseract-wasm is a WebAssembly build of Tesseract for the browser and Node. It strips code the browser doesn't need (such as image-format parsers) to cut download size and startup time. The npm unpacked size is about 3.88 MB (v0.10.0), not counting language data. Weekly downloads are low (about 198) — [npm tesseract-wasm](https://www.npmjs.com/package/tesseract-wasm), [README](https://unpkg.com/tesseract-wasm@0.11.0/README.md). I found no published speed benchmark for tesseract-wasm.
- PP-OCRv5 ONNX conversions, as packaged by a Rust project (oar-ocr): mobile detection is about 4.6 MiB, mobile recognition about 15.8 MiB, and per-language mobile recognition models (English, Latin, Cyrillic) about 7.5–7.7 MiB — [oar-ocr models.md](https://docs.rs/crate/oar-ocr/0.8.1/source/docs/models.md). The Apache-2.0 licence follows upstream PaddleOCR — [HF monkt/paddleocr-onnx](https://huggingface.co/monkt/paddleocr-onnx/commit/7b02d0a30a07ba2b92ad1ff5a8941ae2c633de65). PP-OCRv5 has mobile (smaller, faster) and server (more accurate, larger) variants — [ffocr README](https://cdn.jsdelivr.net/npm/ffocr@0.1.13/README.md). These are third-party size figures, not an official PaddleOCR table.
- Apple Vision: `RecognizeTextRequest` (iOS 18+) is the modern OCR API. `RecognizeDocumentsRequest` (iOS 26+) adds structured document reading with layout understanding — [Apple docs](https://developer.apple.com/documentation/vision/recognizedocumentsrequest.md), [Kodeco](https://www.kodeco.com/ios/programs/apple-intelligence/apple-ai-models/49307523-vision-framework/04-unveiling-new-vision-framework-features-in-ios-18-ios-26/05). Developers report weak results for Khmer/Thai — [Apple dev forums](https://www.developer.apple.com/forums/tags/visionkit)
- Google ML Kit text recognition v2 runs on-device and offline. It supports Latin (default) plus Chinese, Devanagari, Japanese and Korean as add-on libraries — [ML Kit text recognition](https://developers.google.com/ml-kit/vision/text-recognition/v2/android?hl=fr), [ML Kit overview](https://developers.google.com/ml-kit/nl)

### Inferences
- **(opinion)** Use a tiered OCR backend behind one trait. Native shells call platform OCR (Vision, ML Kit, Windows.Media.Ocr). Web/WASM uses ocrs for Latin, with PP-OCRv5 mobile ONNX for other scripts. Load models lazily (about 5–25 MB) so they never count against reader cold start.
- **(opinion)** For the core, prefer ONNX + a pure-Rust runtime (RTen, which ocrs already uses) over a C++ Tesseract dependency, to keep the core portable.
- Verify each model-weights licence on its own terms. Code licences (Apache/MIT) do not carry over to weights (see the CC-BY-SA example above).

### Gaps
- No head-to-head accuracy or speed benchmarks of ocrs vs Tesseract vs PP-OCRv5 in WASM were found. lamina should measure these itself.
- Windows.Media.Ocr language and quality details were not researched (no source gathered).
- The ocrs-models weights licence was not confirmed from a primary source.

## Q2. On-device semantic search, local embeddings, local "chat with PDF", and user sentiment toward AI in PDF apps

### Takeaway
In-browser embeddings are technically practical: about 15 ms per embedding on WebGPU vs about 530 ms on WASM for all-MiniLM-L6-v2 in a Hugging Face benchmark. User sentiment toward paid AI add-ons in PDF apps is negative and price-focused. That leaves room for *optional, local, free* AI that is clearly off by default, rather than an AI-first pitch.

### Cited Findings
- Transformers.js v3 (Oct 2024, `@huggingface/transformers`) added WebGPU via `device: 'webgpu'` — [HF blog](https://huggingface.co/blog/transformersjs-v3). The GitHub releases page lists v4.x with a native WebGPU execution provider — [releases](https://github.com/huggingface/transformers.js/releases)
- WebGPU embedding benchmark (Xenova/all-MiniLM-L6-v2): batch 1, WASM 531.6 ms vs WebGPU 15.2 ms; batch 32, 17,338.8 ms vs 284.8 ms. Hardware and browser are not stated — [HF Space discussion](https://huggingface.co/spaces/Xenova/webgpu-embedding-benchmark/discussions/3)
- Under v3, WebGPU still needed flags in Firefox and Safari — [SitePoint](https://www.sitepoint.com/webgpu-vs-webasm-transformers-js/) (current 2026 browser status not verified here)
- Acrobat AI Assistant is a paid add-on, around $4.99/month on top of Acrobat — [PCWorld](https://www.pcworld.com/article/3215096/adobe-acrobat-pro-dc-review.html). Adobe staff confirmed it is a separate add-on, not included in Creative Cloud — [Adobe community](https://community.adobe.com/t5/acrobat-discussions/is-ai-assistant-for-acrobat-in-my-plan/m-p/15438687)
- On Adobe's forums, a Creative Cloud Pro subscriber called paying extra a "huge rip-off". A September 2025 thread on the Acrobat Studio rollout said it "critically damaged Acrobat's credibility amongst daily professional users" — [Adobe community: Meet Acrobat Studio](https://community.adobe.com/t5/acrobat-discussions/meet-the-new-adobe-acrobat-studio/td-p/15465617) (anecdotal)
- A licensing consultancy estimates enterprise AI Assistant add-ons at $7.50–12/seat/month and uptake of 25–35% in its late-2025 engagements. This is a single-firm observation — [Redress Compliance](https://redresscompliance.com/adobe-acrobat-subscription-pricing-2026)

### Inferences
- **(opinion)** "AI that never leaves your device, costs nothing extra and is off by default" is a credible counter-position to Adobe's paid cloud add-on. It is a *supporting* differentiator, not a lead one: forum complaints focus on price and bundling, not on wanting AI.
- **(opinion)** Ship local semantic search (embeddings + vector index in IndexedDB/OPFS) before any LLM chat. Search is cheap (about 20–90 MB models), deterministic and useful. Local LLM chat adds GBs of weights and quality risk; defer it to a post-editor phase.

### Gaps
- No quantitative survey of user attitudes toward AI in PDF apps was found; the evidence is forum anecdotes.
- No sourced data on small local LLMs (WebLLM etc.) running in-browser on PDFs, or on download sizes acceptable to users.

## Q3. Instant reflow / reader mode: tagged PDFs vs layout analysis; open-source layout models and licences

### Takeaway
About half of web PDFs are now tagged, so a structure-tree reflow path covers roughly half the corpus, but tag quality varies. The rest needs heuristic or ML layout analysis. Licences are a minefield: **DocLayout-YOLO is ⚠ AGPL-3.0**. Docling's code is MIT, but its model licences are separate. Marker's code is now Apache-2.0, but its **weights carry a restrictive modified OpenRAIL-M licence** (free only for research, personal use and startups under $5M).

### Cited Findings
- Dual Lab's analysis of about 15M PDFs from a 2026 Common Crawl found the tagged share passed **50% in mid-2025**, growing about 1.5 points a year. About 2% have an empty structure tree — [Common Crawl group post](https://groups.google.com/g/common-crawl/c/_ttmwLV7wuk). Fewer than 1% of PDFs claim PDF/UA conformance, so many producers tag by default without aiming for accessibility conformance — [PDF Association: Tagged PDF in the Wild](https://pdfa.org/wp-content/uploads/2025/10/0-3-11_30-BorisDoubrov-Tagged_PDF_in_the_Wild.pdf)
- A 2023 Luxembourg study of 25,398 PDFs from public sites found 52% untagged — [accessibilite.public.lu](https://accessibilite.public.lu/en/news/2023-04-28-des-pdf-majoritairement-inaccessibles)
- **⚠ DocLayout-YOLO: AGPL-3.0.** It is built with Ultralytics/YOLO-v10; the weights licence is not stated on the repo page — [opendatalab/DocLayout-YOLO](https://github.com/opendatalab/DocLayout-YOLO)
- Docling's codebase is MIT. Model licences are "found in the original packages" (layout model, TableFormer). It is a Python library (3.10+) with no WASM path mentioned — [docling-project/docling](https://github.com/docling-project/docling)
- Marker's code LICENSE file is Apache-2.0 — [marker LICENSE](https://raw.githubusercontent.com/datalab-to/marker/master/LICENSE). Its **model weights use a modified AI Pubs Open RAIL-M licence**: free for research, personal use and startups under $5M funding/revenue; commercial use beyond that is paid — [datalab-to/marker](https://github.com/datalab-to/marker). Note: the brief describes Marker as GPL. Older releases were GPL-3.0 (from prior knowledge, not re-verified in this session); the current repo shows Apache-2.0 code plus restricted weights. Either way, the weights are not usable in an MIT/Apache product without a commercial agreement.

### Inferences
- **(opinion)** Reflow v1: use the structure tree when present and non-empty. Otherwise use a heuristic path (text-run clustering, column detection, reading order from content order and geometry) in pure Rust. Treat ML layout models as an optional, later plugin.
- **(opinion)** Never depend on DocLayout-YOLO, Ultralytics-derived weights or Marker weights in the permissive core. If an ML layout model is needed, check the licences of Docling's model packages first, or train or obtain a permissively licensed model.
- Reflow quality on untagged PDFs is a hard, demo-able feature. It is a good *later* differentiator, but too risky for v0.1.

### Gaps
- Licences of Docling's layout and TableFormer weights were not checked directly.
- pdfplumber (believed MIT) was not verified in this session.
- No source found on which PDF readers ship true reflow, or on user demand for it.

## Q4. Non-destructive batch operations, signature-preserving incremental save, true redaction: how competitors message these

### Takeaway
Redaction failures (text left under black boxes) recur in high-profile legal cases into 2025–2026, so "true redaction, verified" is a concrete, demonstrable promise. Stirling-PDF already markets local "edit, sign, redact … without sending documents to external services". Privacy plus breadth is contested ground, so lamina has to win on correctness and verifiability, not on tool count.

### Cited Findings
- Failed redactions are a recurring legal embarrassment. The ABA Judges' Journal covers a 2025 big-firm filing where "material so carefully marked secret proved not to be secret at all" — [ABA](https://www.americanbar.org/groups/judicial/resources/judges-journal/archive/embarrassing-redaction-failures/). A study suggested tens of thousands of PACER documents may have failed redactions — [ABA Journal](https://www.abajournal.com/news/article/tens_of_thousands_of_pacer_documents_could_have_failed_redactions_study_sug)
- In the DOJ Epstein file releases (Dec 2025–Feb 2026), text under black boxes could be copied and pasted — [Daily Dot](https://dailydot.com/viral-politics/epstein-files-copy-paste/), [HN thread](https://hn.nuxt.dev/item/46368946). An independent site claims about 12,000 affected PDFs (unverified, non-authoritative) — [epstein-data.com technical report](https://epstein-data.com/reports/evidence/DEFECTIVE_REDACTIONS_TECHNICAL_REPORT.html)
- Vendors describe the cause: the black box is a drawn shape, and the text underneath stays in the content stream — [Syncfusion](https://www.syncfusion.com/resource-center/why-black-boxes-dont-protect-sensitive-data/), [dev.to](https://dev.to/haruodev/the-black-box-in-your-pdf-is-a-shape-not-a-delete-key-1nc3)
- Stirling-PDF (about 95k GitHub stars) markets "Edit, sign, redact, convert, and automate PDFs without sending documents to external services", with 50+ tools, automation workflows and REST APIs — [Stirling-PDF repo](https://github.com/Stirling-Tools/Stirling-PDF)

### Inferences
- **(opinion)** Market redaction with *proof*: after redacting, lamina re-parses the output and shows that no text, image or metadata from the redacted region remains, i.e. a "redaction verification report". Few tools surface this.
- **(opinion)** Signature-preserving incremental save (ADR 0007's save planner) is a technical moat but hard to market on its own. Frame it as "edits never break signatures or the original bytes; every change is reversible" (op log, ADR 0006).
- **(opinion)** Non-destructive batch operations (preview → apply, with undo) belong in the editor phase, not the reader MVP.

### Gaps
- No competitor messaging was found that explicitly promotes "incremental save preserves signatures". This could be a messaging white space, or simply not researched deeply enough here.

## Q5. Privacy-first positioning: evidence that users care

### Takeaway
The strongest primary signal is the **FBI Denver Field Office alert (March 2025)**: free online file converters can deliver malware while still doing the conversion. Combined with routine redaction leaks, this backs a "your file never leaves your device" message that can be checked (no network requests). Stirling-PDF and many small "no upload" WASM tools show the claim is common, so it must be *verifiable* to stand out.

### Cited Findings
- FBI Denver warned in March 2025 that free online document converters "will do the task advertised, but the resulting file can contain hidden malware giving criminals access to the victim's computer". It cites ransomware risk and theft of personal information — [Malwarebytes](https://www.malwarebytes.com/blog/news/2025/03/warning-over-free-online-file-converters-that-actually-install-malware), [TechRadar](https://www.techradar.com/pro/security/free-online-file-converters-could-infect-your-pc-with-malware-fbi-warns), [CBS Colorado](https://www.cbsnews.com/amp/colorado/news/fbi-online-file-converter-malware-steal-private-information), [Security Affairs](https://securityaffairs.com/?p=175764). The alert reportedly names no specific sites; researchers flagged examples such as convertisseurs-pdf[.]com — [Gridinsoft](https://blog.gridinsoft.com/online-file-converter-scam-fbi-warning/)
- Many 2025–2026 indie tools pitch "100% in the browser, no uploads" WASM PDF tools — [dev.to example](https://dev.to/pirate_haizei/i-built-a-free-pdf-tool-that-runs-100-in-your-browser-no-uploads-no-servers-2lle), [DocShift](https://awesome.ecosyste.ms/projects/github.com%2Ftonycletus%2Fdocshift). One write-up warns that "client-side" doesn't mean the page makes no network requests at all — [dev.to](https://dev.to/rodfreitas/why-your-pdf-tool-shouldnt-need-a-server-building-privacy-first-browser-apps-with-webassembly-1bgh)

### Inferences
- **(opinion)** Make privacy verifiable, not just claimed:
  - a strict CSP with `connect-src 'none'` (or self only) on the reader route
  - an in-app "network: 0 requests" indicator
  - open source plus reproducible builds
  - offline/PWA use after first load
  - no analytics on document content

  This sets lamina apart from the many "no upload" claims.
- **(opinion)** Being "the safe place to open PDFs" ties in with the FBI warning: a hardened parser with Limits and fuzzing (ADR 0003) is a security story as well as a privacy one.

### Gaps
- No UK NCSC or German BSI guidance against online converters was found (searches returned only vendor blogs).
- No quantitative user survey of privacy concern with Smallpdf/iLovePDF uploads was found, and no primary report of a leak at those services.
- A claimed Mandiant finding about converters harvesting data appeared only on low-quality pages and was not verified.

## Q6. Pricing and business models of successful open-source/freemium document apps; what "no hostile monetisation" can look like

### Takeaway
Three working templates:
1. **Donations/sponsors** (SumatraPDF, ⚠ GPL)
2. **A free local app with paid optional cloud services** (Obsidian: commercial licence made optional in Feb 2025; Sync and Publish stay paid)
3. **Open core with seat-based enterprise tiers** (Stirling-PDF: free for up to 5 users, paid per-seat Enterprise)

For a permissive (MIT/Apache) project, the Obsidian model fits best: never charge for local features; charge for things that genuinely cost money to run.

### Cited Findings
- SumatraPDF is free, open source and **⚠ GPL-licensed** — [Wikipedia](https://en.wikipedia.org/wiki/Sumatra_PDF). It is funded through Patreon backers and corporate and individual sponsors — [SumatraPDF backers](https://web.archive.org/web/20260312195412/https://www.sumatrapdfreader.org/backers). The author publicly calls out clones that repackage it with malware or ask for donations — [ripoffs: RocketPDF](https://sumatrapdfreader.org/ripoffs/rocketpdf)
- Obsidian: from Feb 20, 2025 the commercial licence is optional (before that, it was required for work use at companies with 2+ employees). The core app keeps "No account required, no ads, no tracking, no strings attached". Sync and Publish remain paid add-ons — [Obsidian blog: Free for work](https://obsidian.md/blog/free-for-work/). Third-party sources list Sync at about $4–5/month and Publish at about $8–10/site/month — [eesel.ai](https://www.eesel.ai/de/blog/obsidian-pricing)
- Stirling-PDF calls itself open-core — [repo](https://github.com/Stirling-Tools/Stirling-PDF). Its pricing page lists free use for up to 5 people and Enterprise "From $12 user/month", for 250+ users with LDAP/SCIM — [stirlingpdf.com/pricing](https://stirlingpdf.com/pricing). The official docs describe Enterprise as custom-quoted, and the hosted web version runs on credits — [Stirling docs: Paid Offerings](https://docs.stirlingpdf.com/Paid-Offerings/), [Modes and Licensing](https://docs.stirlingpdf.com/Modes%20and%20Licensing). Press reports a $99/month or $1,000/year server licence with v2.0 (Dec 2025), which conflicts with the official docs — [OpenSourceForU](https://www.opensourceforu.com/2025/12/stirling-pdf-2-0-brings-text-editing-and-enterprise-tools-to-open-source/)
- Adobe's model, by contrast, layers paid AI add-ons on top of a subscription — [PCWorld](https://www.pcworld.com/article/3215096/adobe-acrobat-pro-dc-review.html)

### Inferences
- **(opinion)** "No hostile monetisation" for lamina could mean:
  - every on-device feature is free forever (reader, editor, OCR, local search, redaction)
  - no account to read or edit
  - no watermark or page caps
  - paid tiers only for costly services: optional E2E-encrypted sync, hosted generation API, support/SLA, enterprise deployment (SSO, policy, MDM)
  - GitHub Sponsors / Open Collective for the core

  Write this down as a public pledge.
- Stirling's open-core move shows the risk: moving features behind a licence later causes friction. Decide the free/paid line up front. I found no sourced evidence on community reaction to Stirling's licensing change.
- Affinity and Okular were not researched (see Gaps).

### Gaps
- No sourced data on Okular (KDE, donation-funded) or Affinity's 2025 business-model changes; no revenue figures for SumatraPDF, Obsidian or Stirling.
- No verified figure for Stirling's server licence price (sources conflict).

## Q7. Recommended phased MVP and success metrics

### Takeaway
**(opinion)** Lead with 2–3 differentiators:
1. **Verifiably private and fast**: zero-upload web reader with a provable "0 network requests", plus a small, instant desktop app.
2. **Correct-by-construction editing**: incremental, signature-preserving saves with undo, and true, verified redaction.
3. **Free local intelligence** (later): on-device OCR and semantic search with no paid add-on.

v0.1 should be a reader only, judged on cold start, time to first page, crash-free rate and fidelity.

### Cited Findings
- Crash-free benchmarks: median crash-free sessions are 99.95% across apps (iOS 99.91%, Android 99.80%), with the 75th percentile about 99.99%. Apps rated under 3 stars have a median of 99.82%. These figures come from a vendor SDK dataset — [Instabug/Luciq 2025 outlook](https://www.instabug.com/mobile-app-stability-outlook-2025), [Luciq benchmark blog](https://www.luciq.ai/blog/benchmarking-crash-free-sessions-for-mobile-apps-whats-a-good-crash-free-rate)
- Existing local-first web PDF readers already exist (e.g. OnePDFs, which handles files up to 100 MB on the device and caches copies in IndexedDB). Most rely on PDF.js, pdf-lib or WASM builds of qpdf/MuPDF — [dev.to survey](https://dev.to/rickkm/why-local-first-web-apps-are-the-future-building-a-zero-server-pdf-engine-with-webassembly-43b), [OnePDFs-style reader](https://pdf.selfboot.cn/en/tools/pdf-reader). Note that **MuPDF is ⚠ AGPL** (lamina CLAUDE.md licence note), so lamina's Rust core is a licensing differentiator over MuPDF-WASM tools.

### Inferences (all opinion)
- **v0.1 reader (web + desktop)**:
  - open from file, drag-drop or OS "open with"; render pages; zoom and fit
  - thumbnails, outline/bookmarks, links
  - text selection and copy, find in document
  - dark mode, keyboard navigation, remember last position
  - open password-protected files
  - a "0 uploads" network indicator plus a strict CSP, offline PWA
  - graceful handling of malformed files (repair warnings)
  - *Out*: annotations, forms, editing, OCR, AI, accounts, sync.
- **v0.2**: annotations (highlight, note, ink) saved with an incremental save; form filling; print; signature *validation display*.
- **v0.3**: page operations (reorder, rotate, delete, merge, split) as non-destructive op-log batches; true redaction with a verification report.
- **v0.4+**: OCR (platform APIs on native, ocrs/PP-OCR in WASM), local semantic search, tagged-PDF reflow then heuristic reflow, generation library 1.0. Local LLM chat comes last, if ever.
- **Success metrics (targets are opinions)**:
  - cold start to interactive: < 300 ms desktop, < 1.5 s web on mid-range hardware
  - time to first page for a 10 MB PDF: < 500 ms
  - crash-free sessions ≥ 99.95% (the median above)
  - render fidelity on the fixtures corpus vs a reference renderer (pixel-diff pass rate)
  - parse success on the malformed corpus with no hangs (ADR 0003/0018)
  - installer/bundle size (desktop < 15 MB; web initial JS+WASM < 1.5 MB gzip)
  - D7/D30 retention, plus "set as default PDF app" rate on desktop
  - share of sessions with zero network requests after load: 100%

### Gaps
- No product-management sources on MVP scoping for document apps were gathered.
- No public time-to-first-page benchmarks for PDF.js, Acrobat or Preview were found, so targets above are not benchmarked against competitors.
- No retention benchmarks for productivity/utility apps were found.
