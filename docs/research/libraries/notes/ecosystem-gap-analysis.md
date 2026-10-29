# PDF library ecosystem gap analysis (TypeScript, Rust, mobile, CLI): state as of 29 Oct 2026

Method note: `gh api` was blocked in this session, and so were documenso.com, libpdf.documenso.com, pistack.xyz and api.npmjs.org (weekly downloads). Version, date and licence figures come straight from the npm registry (`registry.npmjs.org/<pkg>`) and the crates.io API (`crates.io/api/v1/crates/<crate>`), queried on 2026-10-29. GitHub figures come from WebFetch of the repo pages on the same date. Everything else comes from web search snippets, which are weaker sources and are labelled as such.

## Q1: Licences: which are permissive, and which are AGPL/GPL/commercial

### Takeaway
Most TS and Rust libraries are permissive (MIT, Apache, or dual). The copyleft ones are the MuPDF family (mupdf-rs, mutool, PyMuPDF/pymupdf4llm), poppler-utils and probably Ghostscript, pdftk and cpdf. Some AI extractors also carry copyleft or custom terms on their code or model weights. qpdf is the only full-featured manipulation CLI under a permissive licence.

### Cited Findings
**TypeScript (npm registry `license` field, latest version):**
- pdf-lib 1.17.1: MIT — [npm](https://registry.npmjs.org/pdf-lib)
- @cantoo/pdf-lib 2.11.1: MIT — [npm](https://registry.npmjs.org/@cantoo/pdf-lib)
- @pdfme/pdf-lib 6.2.8 (pdfme's own fork of pdf-lib): MIT — [npm](https://registry.npmjs.org/@pdfme/pdf-lib)
- @libpdf/core 0.5.3: MIT. `src/fontbox/` is Apache-2.0 because it is derived from Apache PDFBox — [npm](https://registry.npmjs.org/@libpdf/core), [GitHub](https://github.com/LibPDF-js/core)
- pdfjs-dist 6.4.299: Apache-2.0 — [npm](https://registry.npmjs.org/pdfjs-dist)
- unpdf 2.0.0: MIT — [npm](https://registry.npmjs.org/unpdf)
- pdfkit 0.20.2: MIT — [npm](https://registry.npmjs.org/pdfkit)
- jspdf 4.2.1: MIT — [npm](https://registry.npmjs.org/jspdf)
- @pdfme/generator 6.2.8: MIT — [npm](https://registry.npmjs.org/@pdfme/generator)
- react-native-pdf 7.0.5: MIT — [npm](https://registry.npmjs.org/react-native-pdf)
- "pdf-lib-plus" does not exist on npm; the registry returned no package — [npm](https://registry.npmjs.org/pdf-lib-plus)

**Rust (crates.io `license`, latest version):**
- lopdf 0.46.0: MIT — [crates.io](https://crates.io/api/v1/crates/lopdf)
- hayro / hayro-syntax / hayro-write 0.8.0: Apache-2.0 OR MIT — [crates.io](https://crates.io/api/v1/crates/hayro)
- pdfium-render 0.9.4: MIT OR Apache-2.0. The PDFium binary it loads is Google's, under BSD-style terms; this is training knowledge and was not verified here — [crates.io](https://crates.io/api/v1/crates/pdfium-render)
- **mupdf (mupdf-rs) 0.8.0: AGPL-3.0** — [crates.io](https://crates.io/api/v1/crates/mupdf)
- pdf (pdf-rs) 0.10.0: MIT — [crates.io](https://crates.io/api/v1/crates/pdf)
- printpdf 0.12.8: MIT — [crates.io](https://crates.io/api/v1/crates/printpdf)
- krilla 0.8.2: MIT OR Apache-2.0 — [crates.io](https://crates.io/api/v1/crates/krilla)
- typst-pdf 0.15.1: Apache-2.0 — [crates.io](https://crates.io/api/v1/crates/typst-pdf)
- oxidize-pdf 5.5.4: MIT — [crates.io](https://crates.io/api/v1/crates/oxidize-pdf)

**Mobile:**
- AndroidPdfViewer (now at DImuthuUpe/AndroidPdfViewer): Apache-2.0, copyright 2017 Bartosz Schiller — [GitHub](https://github.com/barteksc/AndroidPdfViewer)
- Nutrient (PSPDFKit) and Apryse are commercial with quote-based pricing. The mobile SDK is "Contact Sales" — [G2 Nutrient](https://www.g2.com/products/pdf-sdk/pricing), [Verdocs on Apryse](https://verdocs.com/?p=14046), [Airframe](https://www.airframe.ai/product/apryse-com-products-ios/compare)

**CLI:**
- qpdf: Apache-2.0 — [qpdf.sourceforge.io](https://qpdf.sourceforge.io)
- mutool: AGPL, the same licence as MuPDF. This is a 2014 source, labelled old — [Linux Pro Magazine 2014](https://www.linuxpromagazine.com/Issues/2014/160/PDF-Tools/(offset)/6/(tagID)/155)
- poppler-utils: GPL (GPLv2 or GPLv3), per distro package metadata — [CERN SL7 repoview](https://linuxsoft.cern.ch/scientific/7x/x86_64/updates/fastbugs/repoview/poppler-utils.html)

**AI extractors (secondary sources only, which conflict):**
- Docling: MIT according to most sources; one page says Apache-2.0 — [osrepos](https://osrepos.com/compare/docling-project-docling-vs-opendataloader-project-opendataloader-pdf), [Codesota](https://codesota.com/ocr/docling-vs-mineru)
- opendataloader-pdf: Apache-2.0. PDF/UA export and accessibility editing are "enterprise" features — [osrepos](https://osrepos.com/compare/datalab-to-marker-vs-opendataloader-project-opendataloader-pdf)
- MinerU: originally AGPL-3.0. One report says v3.1.0 in 2026 moved to a custom "MinerU Open Source License" based on Apache 2.0, with commercial thresholds (more than 100M MAU or more than $20M monthly revenue) — [Codesota](https://codesota.com/ocr/docling-vs-mineru), [ToolNavs](https://toolnavs.com/article/2457-is-mineru-worth-using-the-50k-star-pdf-to-markdown-tool-has-deployment-and-licen), [wikidocs (checked 2026-10-03)](https://wikidocs.net/441164)
- Marker: GPL-3.0 code with cc-by-nc-sa model weights in 1.x. One blog says v2.0.0 relicensed the code to Apache 2.0, while the Surya weights stay under a modified OpenRAIL-M with revenue triggers. That claim has a single source — [themenonlab](https://themenonlab.blog/blog/best-open-source-pdf-to-markdown-tools-2026), [file2markdown](https://www.file2markdown.ai/blog/marker-vs-mineru)
- pymupdf4llm and PyMuPDF are AGPL or commercial, being MuPDF-based — [file2markdown](https://www.file2markdown.ai/blog/is-pymupdf-free-for-commercial-use)

### Inferences
- lamina can depend on, port from or study these without licence risk: pdf-lib and its forks, LibPDF, PDF.js, unpdf, pdfkit, jsPDF, pdfme, lopdf, hayro, krilla, printpdf, pdf-rs, oxidize-pdf, typst-pdf, qpdf (Apache, as a test oracle or for reading its code), AndroidPdfViewer, and the PDFium-based crates.
- lamina must not depend on or copy code from mupdf-rs, MuPDF/mutool, PyMuPDF/pymupdf4llm or poppler. Using them as external test oracles in CI is a separate question that ADR 0019 should decide.
- LibPDF's Apache-2.0 fontbox subdirectory shows the pattern of porting PDFBox (Apache) code. That route is open to lamina as well.

### Gaps
- No sources were retrieved this session for the licences of Ghostscript, pdftk/pdftk-java and cpdf. From training knowledge, unverified: Ghostscript is AGPL-3.0 or commercial (Artifex), pdftk-java is GPL-2.0+, and cpdf is AGPL-3.0 or commercial (Coherent Graphics). Check each project's LICENSE file before relying on this.
- Foxit mobile SDK licensing terms were not retrieved.
- The exact current licence of oxidize-pdf over its history was not checked. crates.io shows MIT for 5.5.4 only.

## Q2: Maintenance status (last release, cadence, bus factor, backlog)

### Takeaway
pdf-lib is effectively unmaintained: its last release was 1.17.1 in November 2021 and its backlog is around 280 issues and 39 PRs. Work has moved to forks (@cantoo, @pdfme) and to the newer LibPDF, which is backed by Documenso but still in beta at 0.x. The Rust side is very active (lopdf, hayro, oxidize-pdf all released in October 2026), but several projects depend on a single person, notably the LaurenzV crates: hayro, krilla and the typst PDF stack. On mobile, Android's own androidx.pdf is still in alpha.

### Cited Findings
**TypeScript**
- **pdf-lib:** latest release is 1.17.1, published 2021-11-06, with 1 npm maintainer — [npm](https://registry.npmjs.org/pdf-lib). The repo has 8.7k stars, 279 open issues and 39 open PRs, and its README has no maintenance notice — [GitHub](https://github.com/Hopding/pdf-lib). One directory reports a last push of 2024-07-17 and 293 open issues — [awesome.ecosyste.ms](https://awesome.ecosyste.ms/projects/github.com%2Fhopding%2Fpdf-lib). Snyk rates its maintenance "Inactive" — [Snyk](https://snyk.io/advisor/npm-package/pdf-lib). In April 2024 the community opened issue #1616, "Is @Hopding ok?", out of concern for the maintainer. This is a secondary source, Spanish-language — [ecosistemastartup](https://ecosistemastartup.com/?p=59905). The repo's MAINTAINERSHIP.md describes a single principal maintainer since September 2017 — [MAINTAINERSHIP.md](https://raw.githubusercontent.com/Hopding/pdf-lib/HEAD/docs/MAINTAINERSHIP.md)
- **@cantoo/pdf-lib:** 2.11.1, released 2026-09-15, with 6 npm maintainers — [npm](https://registry.npmjs.org/@cantoo/pdf-lib). Its README says: "Until pdf-lib project gets a better maintainance, we will maintain this project as long as we need it but cannot guarantee the support for issues too far from our own roadmap." The fork adds SVG support — [README](https://unpkg.com/@cantoo/pdf-lib@2.4.1/README.md)
- **@pdfme/pdf-lib:** 6.2.8, released 2026-10-21. pdfme ships its own pdf-lib fork inside the pdfme monorepo, with 1 npm maintainer — [npm](https://registry.npmjs.org/@pdfme/pdf-lib)
- **@libpdf/core:** 0.5.3, released 2026-10-16, with 1 npm maintainer — [npm](https://registry.npmjs.org/@libpdf/core). The repo has 1.8k stars, 68 forks, 31 open issues and 11 open PRs. The README labels it "Beta Software", warns that APIs "may change between minor versions", and says it is used in production at its sponsor Documenso — [GitHub](https://github.com/LibPDF-js/core)
- **pdfjs-dist:** 6.4.299, released 2026-10-03, with 5 npm maintainers (Mozilla) — [npm](https://registry.npmjs.org/pdfjs-dist)
- **unpdf:** 2.0.0, released 2026-10-28, with 1 npm maintainer (UnJS). The npm metadata has no `types` field; types may still ship through `exports` — [npm](https://registry.npmjs.org/unpdf)
- **pdfkit:** 0.20.2, released 2026-08-30, with 4 npm maintainers. Still pre-1.0, and no `types` field (types come from @types/pdfkit) — [npm](https://registry.npmjs.org/pdfkit)
- **jspdf:** 4.2.1, released 2026-03-17, with 1 npm maintainer — [npm](https://registry.npmjs.org/jspdf)
- **pdfme:** @pdfme/generator 6.2.8, released 2026-10-21, with 1 npm maintainer — [npm](https://registry.npmjs.org/@pdfme/generator)

**Rust** (crates.io: `updated_at`, total downloads / recent 90-day downloads)
- lopdf 0.46.0, 2026-10-21; 26.9M total / 13.2M recent — [crates.io](https://crates.io/api/v1/crates/lopdf)
- hayro 0.8.0, 2026-10-04; 4.3M / 3.0M. hayro-syntax 5.4M / 4.0M; hayro-write 2.6M / 1.6M — [crates.io](https://crates.io/api/v1/crates/hayro-syntax). The repo has 828 stars, 34 open issues, 46 open PRs and 1,992 commits — [GitHub](https://github.com/LaurenzV/hayro)
- pdfium-render 0.9.4, 2026-09-06; 3.5M / 1.7M — [crates.io](https://crates.io/api/v1/crates/pdfium-render)
- mupdf 0.8.0, 2026-06-22; 1.8M / 0.24M — [crates.io](https://crates.io/api/v1/crates/mupdf)
- pdf (pdf-rs) 0.10.0, 2026-03-02; 0.64M / 0.07M, which is the slowest cadence of the Rust set — [crates.io](https://crates.io/api/v1/crates/pdf)
- printpdf 0.12.8, 2026-09-05; 3.7M / 1.7M — [crates.io](https://crates.io/api/v1/crates/printpdf)
- krilla 0.8.2, 2026-06-04; 3.3M / 2.1M — [crates.io](https://crates.io/api/v1/crates/krilla)
- typst-pdf 0.15.1, 2026-07-17; 3.2M / 1.6M — [crates.io](https://crates.io/api/v1/crates/typst-pdf)
- oxidize-pdf 5.5.4, 2026-10-28; 0.18M / 0.13M. That is a very high major-version number for a young crate — [crates.io](https://crates.io/api/v1/crates/oxidize-pdf)
- typst uses hayro for PDF images — [rust-digger](https://rust-digger.code-maven.com/crates/hayro)

**Mobile**
- **androidx.pdf:** the latest release is 1.0.0-alpha19 (2026-07-01), and no stable version has shipped. The first alpha came out in August 2024 with PdfViewerFragment. alpha12 (2025-12-03) was a full Kotlin rewrite using coroutines and ViewModel. EditablePdfViewerFragment, which adds annotations and form filling, arrived in alpha13 or alpha18; the locale pages conflict — [release notes](https://developer.android.com/jetpack/androidx/releases/pdf), [hi](https://developer.android.com/jetpack/androidx/releases/pdf?hl=hi), [de](https://developer.android.com/jetpack/androidx/releases/pdf?hl=de), [he](https://developer.android.com/jetpack/androidx/releases/pdf?hl=he)
- **AndroidPdfViewer:** the project changed owner (barteksc to DImuthuUpe). Its README says the owners are "actively looking for contributors" (issue #1186). It has 8.5k stars, 2.1k forks, 718 open issues and 25 open PRs. The stable version is still 2.8.2; the 16 KB page-size fix exists only in 3.2.0-beta.1 — [GitHub](https://github.com/barteksc/AndroidPdfViewer). Forks on Maven Central exist mainly to supply 16 KB-aligned PDFium builds — [mhiew fork](https://central.sonatype.com/artifact/com.github.mhiew/android-pdf-viewer), [oothp pdfium-android](https://central.sonatype.com/artifact/io.github.oothp/pdfium-android)
- **react-native-pdf:** 7.0.5, released 2026-08-13, with 1 npm maintainer — [npm](https://registry.npmjs.org/react-native-pdf)

### Inferences
- The JS ecosystem has fragmented into pdf-lib forks: @cantoo, @pdfme, and LibPDF as a rewrite. That fragmentation shows demand for an actively maintained pdf-lib-class library. Each fork is scoped to its sponsor's roadmap.
- Bus factor: pdf-lib, LibPDF (one npm publisher, one company), jsPDF, unpdf, pdfme and react-native-pdf each have a single npm publisher. Laurenz Stampfl is central to hayro, krilla and typst-pdf. Only PDF.js (Mozilla) and qpdf have a long-established institutional or community maintainer base.
- Recent download counts show lopdf and hayro-syntax are where Rust usage concentrates.

### Gaps
- Weekly npm download counts are missing because api.npmjs.org was unreachable.
- Commit cadence was not measured for GitHub repos other than the ones fetched above (`gh` was blocked). Stars and issue counts for pdf.js, pdfkit, jsPDF, pdfme, lopdf, pdf-rs, printpdf, krilla, oxidize-pdf, qpdf and react-native-pdf were not retrieved.
- Release cadence for the CLI tools (qpdf, MuPDF, poppler, Ghostscript, cpdf, pdftk-java) was not retrieved.

## Q3: Architecture, strengths, documented flaws and reasons for abandonment

### Takeaway
Pure-language libraries tend to be strong in one area: pdf-lib's API, PDF.js's rendering, hayro's Rust rendering correctness, krilla's generation. They are weak on malformed input, encryption, signatures and incremental saves. LibPDF is the first TypeScript library to claim parsing, encryption, PAdES signing and incremental saves together, but it is beta and has gaps (no signature verification, no TTC, no JBIG2/JPX decoding). Libraries that bind C/C++ (pdfium-render, mupdf-rs, AndroidPdfViewer) render well, but they add native-binary weight (~16 MB on Android), ABI/page-alignment work and, for MuPDF, AGPL.

### Cited Findings
- **pdf-lib (pure TS):** LibPDF's README says pdf-lib has a good API but "chokes on slightly malformed documents", and that PDF.js renders well but needs a browser — [LibPDF GitHub](https://github.com/LibPDF-js/core). Community consensus: treat the original as unmaintained and look at the Cantoo fork — [Snyk](https://dev.snyk.io/advisor/npm-package/pdf-lib), [gittrend](https://gittrend.io/repo/cantoo-scribe/pdf-lib). Also widely known but not re-verified this session: no encryption or decryption, and no incremental save.
- **LibPDF (pure TS):** features are parsing with fallback for malformed files, create/modify, RC4/AES-128/AES-256 (R2–R6) encryption, PAdES B-B/B-T/B-LT/B-LTA signatures, form fill and flatten, merge/split, attachments, text extraction with positions, TTF/OTF embedding with subsetting, JPEG/PNG, and incremental saves that preserve signatures. Runtimes are Node 20+, Bun and browsers with Web Crypto. Stated limitations: no signature verification, no TTC, JBIG2 and JPX passed through but not decoded, no certificate encryption, and JS actions ignored — [GitHub](https://github.com/LibPDF-js/core). Its docs say a full save invalidates signatures and incremental saves must be requested explicitly with `save({ incremental: true })` — [docs: incremental saves](https://libpdf.documenso.com/docs/concepts/incremental-saves), [signatures guide](https://libpdf.documenso.com/docs/guides/signatures) (search snippets; the host was unreachable for fetching).
- **hayro (pure Rust, CPU renderer):** split into crates: hayro-syntax (parse, xref repair, stream decoding), hayro-interpret (to an abstract Device), hayro (bitmaps), hayro-svg, hayro-jpeg2000, hayro-jbig2, hayro-ccitt, hayro-postscript, hayro-cmap, plus hayro-write. Its visual regression suite has 1,600+ PDFs, many from the pdf.js and PDFBox test suites. The author claims the "highest correctness of the Rust renderers". Documented gaps: no knockout groups, no CMYK blending, no 3D; untrusted input can still cause timeouts and OOM; no glyph caching; the API is unsettled; editing is out of scope; MSRV 1.92 — [GitHub](https://github.com/LaurenzV/hayro). The author calls hayro-write internal and not meant for external use — [rust-digger](https://rust-digger.code-maven.com/crates/hayro-write)
- **hayro-syntax:** has best-effort repair of broken xref tables, but its docs do not mention encryption — [docs.rs](https://docs.rs/crate/hayro-syntax/latest)
- **lopdf (pure Rust):** a low-level object-manipulation crate. A competing project's comparison describes it as the more mature option for editing existing files (a vendor source, not neutral) — [pdf_oxide comparison](https://pdf.oxide.fyi/docs/comparison/vs-lopdf). Firecrawl's pdf-inspector uses lopdf for parsing, object resolution and decryption — [instagit](https://instagit.com/firecrawl/pdf-inspector/what-is-the-underlying-technology-used-by-pdf-inspector-for-pdf-parsing.md)
- **pdf-rs:** described as having a battle-tested reader and an experimental write path (blog, could not be fetched) — [pistack 2026-08-26](https://www.pistack.xyz/posts/2026-08-26-rust-pdf-libraries-printpdf-lopdf-pdf-rs-comparison/). The name collides with other crates and packages — [docs.rs pdf-rs](https://docs.rs/crate/pdf-rs/latest), [PyPI pdf-rs](https://pypi.org/project/pdf-rs/)
- **AndroidPdfViewer (Java over PdfiumAndroid, C++):** the README says PdfiumAndroid is "almost 16 MB" of native libraries across ABIs and recommends per-ABI APK splits. pdfium-android 1.9.0's prebuilt .so files were not 16 KB-page aligned, which Google Play now requires; the fix exists only in a beta — [GitHub](https://github.com/barteksc/AndroidPdfViewer)
- **androidx.pdf:** Google's first-party viewer. It is pre-stable and its APIs keep breaking; alpha19 removed the no-argument `onLoadDocumentSuccess()` — [release notes (tr)](https://developer.android.com/jetpack/androidx/releases/pdf?hl=tr). A secondary article positions it as a replacement for third-party viewers — [ProAndroidDev](https://proandroiddev.com/say-goodbye-to-third-party-pdf-libraries-androidx-pdf-is-here-c49872728784)
- **Apple PDFKit (system framework):** regressions reported on Apple's developer forums in iOS 26: `removeAnnotation` stopped working; removed annotations kept rendering until a page reload; annotations outside page bounds stopped rendering; page contents appeared swapped; slowdowns and memory spikes when zooming with many annotations; a new PDFPageAnalyzerV2 (Vision table analysis) runs with no API to disable it, and crashes mention it. On iOS 27, widget annotations render pixelated. No Apple fix has been announced — [804551](https://developer.apple.com/forums/thread/804551), [804619](https://developer.apple.com/forums/thread/804619), [802744](https://developer.apple.com/forums/thread/802744), [825803](https://developer.apple.com/forums/thread/825803), [847155](https://developer.apple.com/forums/thread/847155), [tag list](https://developer.apple.com/forums/tags/pdfkit)
- **Commercial mobile SDKs:** Apryse's median buyer pays $25,140 a year (39 purchases, range $6,913–$92,641) — [Vendr](https://www.vendr.com/marketplace/apryse). Apryse packages start "as low as $1,500" — [Verdocs](https://verdocs.com/?p=14046). The full Apryse SDK can noticeably increase mobile bundle size — [FitGap](https://us.fitgap.com/products/apryse-pdf-sdk). Nutrient pricing is quote-only, and reviewers call it "Expensive" — [G2](https://www.g2.com/products/pdf-sdk/pricing). One unverified claim puts PSPDFKit at about 4–8 MB of binary size and about $10k–$50k+ a year; the source is a SWOT-template site, so it is low quality — [businessmodelcanvastemplate](https://businessmodelcanvastemplate.com/products/pspdfkit-swot-analysis)

### Inferences
- Common reasons developers abandon a library, from the evidence above:
  - pdf-lib: maintenance stall, malformed-file failures, and no encryption or incremental save.
  - PDF.js: rendering-centric, and its write and edit support is limited.
  - AndroidPdfViewer: binary size, 16 KB alignment, and maintainer churn.
  - PDFKit: OS-version regressions the app developer cannot fix.
  - Commercial SDKs: price and opacity.
- What commercial SDKs sell that free stacks lack is breadth in one supported package. Free stacks need assembling, and the platform viewers (PDFKit, androidx.pdf) are viewer-first and controlled by the OS vendor. The breadth is: viewer UI, annotations, forms, signatures and redaction across iOS, Android and web, with a vendor on the hook for regressions. This is partly inference; vendor feature pages were not fetched.
- hayro is the strongest permissive Rust renderer and is reusable for lamina's rendering layer, but it explicitly does not edit documents and is not hardened against untrusted input.

### Gaps
- Reasons for abandoning pdfkit, jsPDF, pdfme, unpdf, krilla, printpdf, oxidize-pdf, pdfium-render and mupdf-rs were not researched with primary sources this session; their issue trackers were not reachable via `gh`.
- No HN or Reddit threads were retrieved. No direct comparison of encryption handling or malformed-file tolerance across Rust crates was found ("I found no reliable sources").
- No vendor feature lists or binary sizes were retrieved for Nutrient, Apryse or Foxit.
- CLI tool architecture and flaws (qpdf, mutool, pdftk, poppler, cpdf, Ghostscript) were not covered by fetched sources beyond licences.
- AI extractors (marker, docling, MinerU, opendataloader-pdf, pymupdf4llm): only licences are covered. A 2026 benchmark exists but was not fetched — [MarkTechPost 2026-07-24](https://www.marktechpost.com/2026/07/24/datalab-marker-v2-vs-mineru-docling-and-liteparse-benchmark-breakdown/)

## Q4: Gap table: what no permissively licensed library does well today

### Takeaway
No permissive library covers parse → edit → incremental save → sign → render in one engine across web, Node, iOS and Android. LibPDF comes closest in TypeScript (no rendering, beta, TS-only). hayro comes closest in Rust rendering (no editing). Signature verification, hardening against untrusted input and a stable first-party mobile viewer are open gaps.

### Cited Findings
| Capability | Best permissive option today | Gap | Source |
|---|---|---|---|
| Malformed-file tolerance in an editing library | LibPDF (claims "graceful fallback"); hayro-syntax (xref repair, reader only) | pdf-lib chokes; hayro is a reader, not an editor | [LibPDF](https://github.com/LibPDF-js/core), [hayro-syntax](https://docs.rs/crate/hayro-syntax/latest) |
| Encryption (read and write) | LibPDF (RC4/AES R2–R6, password only); lopdf (decryption) | No certificate (public-key) encryption in LibPDF; hayro-syntax does not document encryption | [LibPDF](https://github.com/LibPDF-js/core), [instagit](https://instagit.com/firecrawl/pdf-inspector/what-is-the-underlying-technology-used-by-pdf-inspector-for-pdf-parsing.md) |
| Incremental save and signature preservation | LibPDF (explicit `incremental: true`) | Not automatic; a full save invalidates signatures; nothing equivalent in Rust found | [LibPDF docs](https://libpdf.documenso.com/docs/concepts/incremental-saves) |
| Signature verification | none found | LibPDF signs but does not verify | [LibPDF](https://github.com/LibPDF-js/core) |
| Rendering fidelity, pure code (wasm-friendly) | hayro (1,600+ PDF regression suite) | No knockout, CMYK blending or glyph cache; DoS/OOM on untrusted input | [hayro](https://github.com/LaurenzV/hayro) |
| JBIG2/JPX in pure TS | none | LibPDF passes them through only | [LibPDF](https://github.com/LibPDF-js/core) |
| Font handling | LibPDF (TTF/OTF subsetting); krilla (generation) | No TTC support in LibPDF | [LibPDF](https://github.com/LibPDF-js/core) |
| Android viewer without a large native binary | androidx.pdf (alpha) | Pre-stable APIs; AndroidPdfViewer ships ~16 MB native and its 16 KB fix is beta only | [androidx.pdf](https://developer.android.com/jetpack/androidx/releases/pdf), [AndroidPdfViewer](https://github.com/barteksc/AndroidPdfViewer) |
| iOS viewer and editor independent of OS regressions | PDFKit (system) | iOS 26/27 annotation regressions developers cannot fix | [Apple forums](https://developer.apple.com/forums/tags/pdfkit) |
| Full manipulation CLI | qpdf (Apache-2.0) | Rendering and text extraction CLIs are copyleft (mutool AGPL, poppler GPL) | [qpdf](https://qpdf.sourceforge.io), [poppler](https://linuxsoft.cern.ch/scientific/7x/x86_64/updates/fastbugs/repoview/poppler-utils.html) |
| LLM-ready extraction without copyleft | Docling (MIT), opendataloader-pdf (Apache) | PyMuPDF4LLM is AGPL; MinerU and Marker have custom or model-weight restrictions | see Q1 |

### Inferences
- lamina's clearest niche: one permissive Rust core with TS, Swift, Kotlin and CLI front ends that combines LibPDF-level editing (encryption, incremental save, PAdES) with hayro-level reading and rendering. lamina could reuse hayro under MIT/Apache, or interoperate with it.
- Signature verification and untrusted-input hardening (a `Limits`-style API, as in ADR 0003) are differentiators no permissive library documents today.
- A lamina CLI could be the first permissive tool covering both qpdf-style structural operations and text extraction or rendering. Today the second half requires AGPL or GPL tools.

### Gaps
- Bundle size and Node-only API problems, plus memory blowups on large files, were not documented with sources for pdf-lib, PDF.js, jsPDF or pdfkit this session. Training knowledge says pdfkit is Node-stream-oriented and needs bundler shims in browsers, and PDF.js loads its worker as a separate bundle; both are unverified here.
- No measured numbers on TS bundle sizes or wasm sizes.
