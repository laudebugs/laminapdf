# LLM-friendly API design and AI-consumption primitives for lamina (as of Oct 2026)

Method note: this session's network allowed WebSearch plus fetches from github.com only. Fetches to pdfa.org, arxiv.org, learn.microsoft.com, docs.aws.amazon.com, docling-project.github.io and svelte.dev failed with DNS errors (the hosts are outside the session's allowed domains). For those hosts, the claims below rest on search-result snippets. Items marked **[bg]** come from background knowledge (pre-2026) and were not re-verified this session. Each one carries its canonical URL so the writer can check it.

## (A) What makes LLMs/agents write correct code against a library API, and what lamina should do

### Takeaway
LLMs hallucinate most on infrequent APIs, which a new library's whole API will be at first. They also break when an API changes across versions. Supplying current docs helps on rare APIs. It can hurt on common ones when retrieval is poor, so the API should be guessable without docs: conventional names, named options, explicit units/origin, stable names. Errors and machine-readable docs should then catch whatever the guessing gets wrong.

### Cited Findings
**Hallucination evidence**
- CloudAPIBench (Jain, Kwiatkowski, Ray, Ramanathan, Kumar; AWS/Columbia; arXiv 2407.09726, ICSE 2025 SEIP) has 622 synthetic Python tasks over AWS/Azure APIs. The tasks are bucketed by how often each API appears in The Stack v2. Code LLMs struggle most with low-frequency APIs. — [arXiv](https://arxiv.org/abs/2407.09726); [ICSE 2025 SEIP](https://conf.researchr.org/details/icse-2025/icse-2025-software-engineering-in-practice/33/On-Mitigating-Code-LLM-Hallucinations-with-API-Documentation); [Amazon Science](https://www.amazon.science/publications/on-mitigating-code-llm-hallucinations-with-api-documentation)
- Documentation-Augmented Generation (DAG) raised low-frequency-API performance to 47.94%. With a sub-optimal retriever it lowered high-frequency-API performance, by about 39.02% absolute according to a snippet; I could not fetch the full text to confirm that figure. DAG++ retrieves docs only when needed, for example when the model is unconfident or the API name fails an index check. It reports up to +16.40% over base models without the high-frequency degradation. — [arXiv HTML](https://arxiv.org/html/2407.09726v1); [Moonlight review](https://www.themoonlight.io/en/review/on-mitigating-code-llm-hallucinations-with-api-documentation)
- Version drift: GitChameleon (2024) has 116 version-conditioned Python completion problems with executable tests. GPT-4o reached pass@10 = 39.9%, or 43.7% with error feedback. GitChameleon 2.0 (328 problems; ACL 2026) reports enterprise-model baselines of 48–51%. — [HF papers 2411.05830](https://huggingface.co/papers/2411.05830); [arXiv 2507.12367](https://arxiv.org/abs/2507.12367v1)
- Gorilla/APIBench is the earlier benchmark showing that retrieval of API docs reduces API hallucination. — [APIBench dataset card](https://huggingface.co/datasets/gorilla-llm/APIBench/blob/ac21e1892e634dfa25f8ad75f16cbdbfb0a5736d/README.md)

**Docs formats and channels for agents**
- llms.txt was proposed by Jeremy Howard on 3 Sep 2024. The format is an H1 name, a blockquote summary, optional prose, then H2 sections of Markdown link lists pointing at `.md` pages. `llms-ctx.txt` is an expanded variant that inlines the linked content. It is still a proposal; as of early 2026 no major model provider had publicly confirmed consuming it. — [llmstxt.org](https://llmstxt.org/); [llms-ctx.txt](https://llmstxt.org/llms-ctx.txt); [Search Engine Land](https://searchengineland.com/llms-txt-proposed-standard-453676)
- Google reportedly added llms.txt to Lighthouse. This comes from a trade press report I did not verify against Google. — [PPC Land](https://ppc.land/google-adds-llms-txt-to-lighthouse-as-agentic-web-standards-heat-up/)
- Stripe publishes `/llms.txt` and a `.md` twin of every docs page; for example, docs.stripe.com/building-with-llms.md is the Markdown version of the building-with-llms page. — [Stripe: Building with LLMs](https://docs.stripe.com/building-with-llms); [.md version](https://docs.stripe.com/building-with-llms.md)
- The Vercel AI SDK publishes ai-sdk.dev/llms.txt "for Cursor, Windsurf, Copilot, Claude etc." and ships its docs as MDX inside the npm package (`ai@7.x/docs/…`). Agents can therefore read version-matched docs from `node_modules`. — [unpkg ai@7.0.90 docs](https://app.unpkg.com/ai@7.0.90/files/docs/00-introduction/index.mdx)
- Svelte (svelte.dev/docs/llms, with llms.txt / llms-full.txt / llms-medium.txt / llms-small.txt), Cloudflare and Prisma also publish llms.txt files **[bg]**. svelte.dev could not be fetched this session, so this is unverified here.
- Context7 (Upstash) is an MCP server with two tools, `resolve-library-id` and `query-docs`, that return version-specific docs and snippets to coding agents. Its own docs publish llms.txt. One commentator flags the risk of a community-submitted registry. — [Upstash blog](https://upstash.com/blog/context7-mcp); [Hashnode tag page](https://hashnode.com/tag/context7)
- AGENTS.md is plain Markdown with no schema and no required fields. OpenAI contributed it to the Agentic AI Foundation, a Linux Foundation directed fund launched 9 Dec 2025 with MCP and goose as the other founding projects. — [OpenAI](https://openai.com/index/agentic-ai-foundation/); [AAIF blog](https://aaif.io/blog/measuring-agents-md-what-five-runs-show-that-one-doesn-t)
- A third-party blog claims 60k+ repositories and 25+ tools use AGENTS.md (unverified). — [D. Vaughan](https://codex.danielvaughan.com/2026/04/07/agents-md-open-standard-cross-tool-portability)

**API traits**
- Coordinate origin is a known footgun. pdf-lib uses the PDF user-space origin at the bottom-left, so users write `y: height - 150` to place content near the top. Text `y` is the baseline. The origin depends on MediaBox/CropBox, which need not start at (0,0). — [StudyRaid on pdf-lib coords](https://app.studyraid.com/en/read/12496/404105/understanding-pdf-coordinate-systems); [Documenso libpdf drawing guide](https://libpdf.documenso.com/docs/guides/drawing); [ABCpdf coordinates](https://websupergoo.com/helppdf/source/2-getting_started/3-coordinates.htm)
- Extraction ecosystems disagree on the origin:
  - Top-left: Docling defaults `CoordOrigin.TOPLEFT` but supports both origins with `to_top_left_origin(page_height)`/`to_bottom_left_origin`. Textract, Azure and Unstructured are also top-left.
  - Bottom-left: OpenDataLoader emits `[left,bottom,right,top]` in PDF points (see section B).
  - So the origin must be an explicit, typed field. — [docling-core base.py](https://github.com/docling-project/docling-core/blob/main/docling_core/types/doc/base.py)
- Rust compiler diagnostics give each error a stable code (E0308), a primary span, labels, `help:`/`note:` lines, and machine-applicable suggestions in `--error-format=json`. Elm's error messages write the hint in plain sentences. **[bg]** — [rustc JSON diagnostics](https://doc.rust-lang.org/rustc/json.html); [Elm "Compiler Errors for Humans"](https://elm-lang.org/news/compiler-errors-for-humans)
- lamina already has stable warning/error codes in `fixtures/codes.json`, checked against the Rust enums (repo CLAUDE.md). This fits the "typed error + stable code" pattern.

### Inferences
- **A new library is a "low-frequency API" for every model by definition.** The CloudAPIBench result means lamina cannot rely on models' training data at launch. It should ship:
  1. `llms.txt` plus `llms-full.txt`, with `.md` twins of every docs page (Stripe pattern).
  2. Docs inside the npm/crate package, version-matched (AI SDK pattern).
  3. An AGENTS.md section in generated project templates.
  4. A Context7 listing and, optionally, a small MCP docs server or a `laminapdf docs <symbol>` CLI that prints the docs for the installed version.
- **Make the API guessable, because retrieval can make high-frequency patterns worse.** Use the names models already expect from pdf-lib, PDF.js and PyMuPDF where semantics match, such as `addPage`, `drawText` and `getPage`, but fix the footguns:
  - Use one coordinate convention in the public API. Recommendation (opinion): top-left origin, y down, in points, for the extraction and high-level drawing APIs. That matches every extraction vendor and the browser.
  - Keep a raw `UserSpace` type for low-level content-stream work.
  - Make the convention visible in types: `Pt(72.0)`, `Rect { origin: TopLeft, unit: Pt }`, and `Mm()`/`In()` constructors. Don't accept naked `f64` pairs.
- Use named options objects in TS and builder or `Options` structs with `Default` in Rust, never long positional argument lists. Unknown option keys should be a type error in TS (exact types) and a runtime `E_UNKNOWN_OPTION` with a "did you mean `fontSize`?" hint in the JS glue, because LLM-written JS often bypasses types.
- Avoid hidden global state (a registered default font, a global page cursor). Make output deterministic (no timestamps or random IDs unless asked; `/ID` derived from content). Agents then can verify by byte-diffing and golden tests stay stable. This aligns with lamina's golden fixtures.
- Errors should be serialisable as `{code, message, hint, didYouMean, span:{objId, offset}, docsUrl}`. The rustc JSON shape is a good model, and the same JSON should come out of the CLI's `--json` mode.
- Never reuse a name with different semantics. When renaming, keep a deprecated alias whose error or warning names the replacement; GitChameleon shows models write old-version code. Publish a machine-readable changelog of renames.
- Self-describing schemas: generate JSON Schema for every options type and output type from the Rust types. schemars or tsify both fit ADR 0002's tsify choice (this is an opinion; not verified against the ADR text). Version output schemas (`"schema": "laminapdf.extract/1"`) and ship them in the package and at a stable URL. Agents and validators can then check outputs, and the CLI can print `laminapdf schema extract`.

### Gaps
- I could not fetch Svelte, Cloudflare or Prisma llms.txt pages, or Vercel's llms-full.txt, to confirm them this session.
- I found no controlled study measuring whether llms.txt itself improves agent correctness; the evidence is for doc retrieval in general (DAG, Gorilla).
- I found no rigorous study on options-objects vs positional arguments, or on coordinate conventions, and LLM error rates. Those recommendations are reasoned, not measured.
- VersiCode and other version-drift benchmarks did not surface in search.

## (B) AI-consumption primitives: the 2026 landscape, algorithms, bbox/provenance schema, Tagged PDF, and rule-based vs model-based

### Takeaway
In 2026 the top of document parsing is dominated by VLM/model pipelines: MinerU2.5-Pro, TeleOCR, olmOCR, Chandra and PaddleOCR-VL score about 83 on olmOCR-bench and about 93–97 on OmniDocBench. Rule-based extraction from born-digital PDFs is fast and very good at reading order, about 0.90 on OpenDataLoader's benchmark at 0.015 s/page, but weak on borderless tables (about 0.49 TEDS) and scans. Several top tools are GPL/AGPL or carry restricted model licences.

That points to a clean split for lamina:
- **Rust core (rule-based, no weights):** characters with bboxes, fonts and ToUnicode; reading order (Tagged structure tree first, then XY-cut++); ruling-line tables; headings from font metrics; header/footer removal; de-hyphenation; a provenance-rich JSON schema.
- **Optional model plug-ins (licence-clean):** layout and borderless tables via Apache Docling Heron, and OCR.

### Cited Findings
**Tools and licences (Oct 2026)**
- **OpenDataLoader PDF (Hancom):**
  - Licence is Apache-2.0 from v2.0 (MPL-2.0 before). Implemented in Java, with Python/Node/Java SDKs; each `convert()` spawns a JVM.
  - Local mode is rule-based and deterministic, with no GPU. Hybrid mode sends complex pages to a local `docling-fast` backend and adds OCR (EasyOCR default), LaTeX formulas, and SmolVLM-256M picture descriptions.
  - Reading order uses XY-Cut++.
  - Tables: border analysis plus text clustering for simple bordered tables; hybrid mode for borderless ones, raising TEDS from 0.489 to 0.928.
  - Tagged PDF: `use_struct_tree=True` reads existing tags and overrides hybrid mode when both are set. `format="tagged-pdf"` auto-tags following Well-Tagged PDF, validated with veraPDF. PDF/UA export is an enterprise add-on.
  - — [GitHub](https://github.com/opendataloader-project/opendataloader-pdf)
- **OpenDataLoader's own benchmark** (200 real PDFs, 0–1 scores, Apple M4; the vendor's own numbers):

  | Engine | Overall | Reading order | Table TEDS | Heading | s/page | Licence |
  |---|---|---|---|---|---|---|
  | ODL hybrid | 0.907 | 0.934 | 0.928 | 0.821 | 0.463 | Apache-2.0 |
  | docling | 0.882 | 0.898 | 0.887 | 0.824 | 0.762 | MIT |
  | marker | 0.861 | 0.890 | 0.808 | 0.796 | 53.9 | GPL-3.0 |
  | ODL local | 0.831 | 0.902 | 0.489 | 0.739 | 0.015 | Apache-2.0 |
  | pymupdf4llm | 0.732 | 0.885 | 0.401 | 0.412 | 0.091 | AGPL-3.0 |

  — [GitHub](https://github.com/opendataloader-project/opendataloader-pdf)
- **MinerU (OpenDataLab):** now at v4.0 (`mineru>=4.0,<5`). Its licence is "based on Apache 2.0 with additional conditions" (the MinerU Open Source License), so it is not plain Apache and needs legal review. The question's suggestion of AGPL refers to older versions; in background knowledge, MinerU 1.x/2.x was AGPL-3.0 **[bg]**. MinerU uses ONNX/Torch small models plus a VLM via llama.cpp, vLLM or LMDeploy, and renders Markdown, HTML, LaTeX, DOCX, EPUB and Content List V1/V2. — [GitHub](https://github.com/opendatalab/MinerU)
- **MinerU2.5-Pro (NeurIPS 2026)** raises OmniDocBench v1.6 Full from 92.98 to 95.69. — [NeurIPS 2026 poster](https://neurips.cc/virtual/2026/poster/139545)
- **Marker (Datalab):** code was GPL-3.0 through 1.10.2 (Jan 2026) and reportedly moved to Apache-2.0 at v2.0.0 (2026-07-20). The Surya weights are under a modified OpenRAIL-M with revenue/funding thresholds ("free for … startups under $5M") and a no-competing-with-Datalab clause. These claims come from secondary sources and should be verified against the repo's LICENSE/MODEL_LICENSE. — [Particula blog](https://particula.tech/blog/docling-vs-mineru-vs-marker-pdf-parser); [Fossies MODEL_LICENSE](https://github.com/datalab-to/marker/blob/master/MODEL_LICENSE)
- **olmOCR (AI2):** Apache-2.0. olmOCR-Bench has more than 7,000 tests over about 1,400 documents in these categories: arXiv math, old-scan math, tables, old scans, headers/footers, multi-column, long tiny text and base. Leaderboard (overall):

  | Tool | Score |
  |---|---|
  | Chandra 0.1.0 | 83.1 |
  | Infinity-Parser 7B | 82.5 |
  | olmOCR v0.4.0 (Oct 2025) | 82.4 |
  | PaddleOCR-VL | 80.0 |
  | Marker 1.10.1 | 76.1 |
  | DeepSeek-OCR | 75.7 |
  | MinerU 2.5.4 | 75.2 |
  | Mistral OCR API | 72.0 |
  | Nanonets-OCR2-3B | 69.5 |

  The leaderboard dates from late 2025. — [GitHub allenai/olmocr](https://github.com/allenai/olmocr)
- **OmniDocBench (CVPR 2025):** v1.6 (2026-04-10) added 296 harder pages. On v1.6_full: Marker pipeline 78.44 and Mistral OCR 85.66. TeleOCR (1.2B, MinerU-based) is reported at about 96.9. These are secondary readings of the leaderboard; I found no reliable Docling number. — [OmniDocBench paper](https://arxiv.org/html/2412.07626v2); [CVPR PDF](https://openaccess.thecvf.com/content/CVPR2025/papers/Ouyang_OmniDocBench_Benchmarking_Diverse_PDF_Document_Parsing_with_Comprehensive_Annotations_CVPR_2025_paper.pdf); [FourWeekMBA on TeleOCR](https://fourweekmba.com/ai-teleocr-1b-document-parsing-benchmark/); [ocr-benchmarks blog](https://invoicedataextraction.com/blog/ocr-benchmarks)
- **Docling model licences:** the docling-layout-heron model (RT-DETRv2, ResNet-50/101 variants) is Apache-2.0 on Hugging Face, with an Apache ONNX export. The tech report says all checkpoints are permissively licensed. — [HF docling-layout-heron](https://huggingface.co/docling-project/docling-layout-heron/tree/main); [arXiv 2509.11720](https://arxiv.org/html/2509.11720v1)
- **Kreuzberg** redistributes layout models in one place, which shows there is demand for packaged ONNX layout weights. — [HF xberg-io/layout-models](https://huggingface.co/xberg-io/layout-models)
- **Commercial services** (LlamaParse, Reducto, Mistral OCR, Azure DI, AWS Textract, Google Document AI): Mistral OCR scores are given above. I found no new 2026 primary data on LlamaParse, Reducto or Google Document AI this session.

**Bounding box conventions**
- **Textract:** `BoundingBox{Left,Top,Width,Height}` are ratios of page width and height, from a top-left origin. Polygon points are also normalized to [0,1]. — [AWS Textract text location](https://docs.aws.amazon.com/textract/latest/dg/text-location.html); [API_BoundingBox](https://docs.aws.amazon.com/textract/latest/dg/API_BoundingBox.html)
- **Azure Document Intelligence:** `boundingRegions[{pageNumber, polygon}]`. The polygon is a flat `[x1,y1,…]` list running clockwise from top-left. Units are inches for PDF and pixels for images (`page.unit`). `spans{offset,length}` index into the concatenated reading-order `content`. — [Python BoundingRegion](https://learn.microsoft.com/en-us/python/api/azure-ai-formrecognizer/azure.ai.formrecognizer.boundingregion?view=azure-python); [DocumentPageLengthUnit](https://learn.microsoft.com/dotnet/api/azure.ai.formrecognizer.documentanalysis.documentpagelengthunit?view=azure-dotnet); [JS DocumentOutput](https://learn.microsoft.com/en-us/javascript/api/@azure-rest/ai-document-intelligence/documentoutput); [Q&A](https://learn.microsoft.com/en-us/answers/questions/2179958/azure-document-intelligence-behavior-and-interface)
- **Unstructured:** `metadata.coordinates.points` lists the box corners starting top-left and running counter-clockwise, in pixels from a top-left origin, plus a `system` (e.g. PixelSpace) with orientation, layout width and layout height. — [Unstructured docs](https://docs.unstructured.io/open-source/concepts/document-elements)
- **Docling:** `BoundingBox{l,t,r,b,coord_origin}` (TOPLEFT default), with a `normalized()` helper. Items carry `prov[{page_no, bbox, charspan}]` **[bg for prov/charspan]**. — [docling-core base.py](https://github.com/docling-project/docling-core/blob/main/docling_core/types/doc/base.py)
- **OpenDataLoader:** `bounding box: [left,bottom,right,top]` in PDF points, `page number` 1-indexed. Other fields: `type`, `id`, `heading level`, `font`, `font size`, `text color`, `content`, and `alt`/`alt_source`/`pdfua_tag` for images. — [GitHub](https://github.com/opendataloader-project/opendataloader-pdf)

**Tagged PDF and the PDF Association**
- Adobe (Matthew Hardy, PDF Association event, Sept 2025): fewer than 20% of PDFs are tagged and fewer than 5% are tagged at high quality. — [pdfa.org slides PDF](https://pdfa.org/wp-content/uploads/2025/10/1-1-11_45-MatthewHardy-Empowering_the_Future_of_PDF_with_AI.pdf) (snippet only; the host was unreachable)
- The Well-Tagged PDF spec was produced by the PDF/UA and PDF Reuse TWGs. Its requirements feed ISO 14289-2 (PDF/UA-2). Conference reporting calls UA-2 plus reuse features "the gold standard" starting point for AI. — [pdfa.org accessibility](https://pdfa.org/accessibility); [Metaarchivist dispatch](https://metaarchivist.substack.com/p/dispatch-from-berlin-15d)
- The PDF Association published an AI-and-PDF FAQ in March/April 2026 ("sets the AI record straight"). — [ANSI member update 4-1-26](https://www.ansi.org/standards-news/member-updates/4-1-26-the-pdf-association-sets-the-ai-record-straight-with-faq-resource); [pdfa.org "Enabling AI to use PDF"](https://pdfa.org/enabling-ai-to-use-pdf/) (not fetchable)
- May 2026: the PDF Association reported Hancom open-sourcing AI auto-tagging in OpenDataLoader. — [pdfa.org](https://pdfa.org/hancom-open-sources-ai-auto-tagging-in-opendataloader-pdf/); [OpenDataLoader tagged-PDF doc](https://scour.ing/p/https://opendataloader.org/docs/tagged-pdf)
- ISO 32000-2 §14.8 (Tagged PDF) defines the logical structure tree, `/MarkInfo`, the standard structure types (H1–H6, P, L/LI/Lbl/LBody, Table/TR/TH/TD, Figure with `/Alt`, Artifact) and `/ActualText`. ISO 32000-2 Annex L / PDF 2.0 adds namespaces. **[bg]** — [PDF Association ISO 32000-2 page](https://pdfa.org/resource/iso-32000-2/)

**Algorithms (canonical sources, [bg])**
- **Reading order:** XY-cut is recursive projection-profile splitting (Nagy & Seth 1984). XY-Cut++ (2025) adds pre-masking of cross-column elements and semantic merging, and is used by OpenDataLoader. Docstrum (O'Gorman 1993) uses nearest-neighbour clustering of components to find lines and blocks. — [XY-Cut++ arXiv 2504.10258](https://arxiv.org/abs/2504.10258); [Docstrum, IEEE TPAMI 1993](https://doi.org/10.1109/34.244677)
- **Tables:** Camelot "lattice" detects ruling lines and intersections (originally via OpenCV morphology on a rendered page) and assigns text to cells. Camelot "stream" infers columns from whitespace and text alignment (Nurminen's algorithm). pdfplumber uses `explicit/lines/text` strategies with snap, join and intersection tolerances over vector edges, and is MIT. Tabula provides lattice/stream in Java and is MIT. — [Camelot docs: how it works](https://camelot-py.readthedocs.io/en/latest/user/how-it-works.html); [pdfplumber table extraction](https://github.com/jsvine/pdfplumber#extracting-tables); [tabula-java](https://github.com/tabulapdf/tabula-java)
- **Text-level issues:** ToUnicode CMaps are often missing or wrong, which garbles text (ISO 32000-2 §9.10). Ligatures (ﬁ/ﬂ) should be decomposed by NFKC or `/ActualText`. Invisible text uses render mode 3 (`Tr 3`), which is OCR layers in scanned PDFs, and also white-on-white or clipped text, a prompt-injection vector. Soft hyphens and end-of-line hyphens are joined when a line ends in "-" and the next starts lowercase, checked against a dictionary or frequency list. Headers and footers are removed by detecting text repeated at similar y positions across pages; page numbers by regex. `Artifact` marked content is authoritative where tagged. **[bg]** — [ISO 32000-2](https://pdfa.org/resource/iso-32000-2/)

### Inferences
**Pure rule-based in Rust core (no weights)**
- Content-stream text extraction with full graphics state: glyph to Unicode via ToUnicode, then the encoding, then the AGL, then `/ActualText`. Report per-glyph confidence and a warning code when the mapping is guessed.
- Glyph-level and span-level bounding boxes.
- Render mode and visibility flags (`invisible`, `clipped`, `offPage`, `lowContrast`) so RAG pipelines can drop hidden text, a security win.
- Struct-tree walker: when `/MarkInfo /Marked true` and the tree passes sanity checks, it becomes the source of reading order and semantics. Mark it `source: "tagged"`.
- Fallback reading order with XY-cut++ and column detection. This is competitive: about 0.90 reading-order on ODL's benchmark with a rule-based method.
- Font-metric heading classification (size and weight clusters relative to body mode).
- List detection from bullet glyphs and numbering regexes with hanging indent.
- Ruling-line (lattice) tables from vector paths. pdfplumber-style edge snapping is easier than Camelot's raster approach because lamina has the vector paths directly.
- Header/footer/page-number artifact removal.
- De-hyphenation and ligature normalisation.
- Markdown and JSON serialisers.

All of this is bytes-in/bytes-out and wasm-friendly, consistent with ADR 0001's core rules.

**Optional, model-based, edge crates/packages**
- Layout detection for untagged complex pages via Docling Heron (Apache-2.0, ONNX), run through `ort` or the web via onnxruntime-web.
- Borderless or complex tables: TableFormer (licence unverified) or TATR (MIT, per a third-party mirror).
- OCR, through a pluggable trait. Options: PaddleOCR (Apache-2.0 **[bg]**), Tesseract (Apache-2.0 **[bg]**), or platform OCR such as Apple Vision and Android ML Kit at the Swift/Kotlin edges.
- VLM "describe figure" and full-page VLM fallbacks (olmOCR Apache-2.0) as a remote or plug-in backend.

Avoid MinerU (custom conditions), marker/Surya weights (restricted OpenRAIL-M), and pymupdf4llm/PyMuPDF (AGPL), in line with ADR 0001's licence rules.

**Coordinate recommendation (opinion)**
- Emit points, top-left origin, page index 0-based in the API, and `pageNumber` 1-based for humans. Provide `bboxNorm` in [0,1] relative to the CropBox for renderer-independent citation highlighting.
- Always record the page box used (CropBox), the `/Rotate` value applied, and the `/UserUnit`.

**Ideal chunk/provenance JSON (proposed `laminapdf.extract/1`)**

```json
{
  "schema": "https://lamina.dev/schema/extract/1.json",
  "generator": { "name": "lamina", "version": "0.4.0", "mode": "rules", "models": [] },
  "source": { "sha256": "…", "pageCount": 12, "tagged": true, "structSource": "tagged" },
  "coord": { "unit": "pt", "origin": "top-left", "box": "CropBox", "rotationApplied": true },
  "pages": [
    { "index": 0, "number": 1, "label": "i", "width": 612, "height": 792, "rotate": 0 }
  ],
  "blocks": [
    {
      "id": "b12",
      "type": "heading", "level": 2,
      "text": "3.1 Reading order",
      "page": 0,
      "bbox": [72.0, 96.4, 310.2, 112.0],
      "bboxNorm": [0.1176, 0.1217, 0.5069, 0.1414],
      "path": ["Document", "Sect[2]", "H2[0]"],
      "structRef": { "obj": 145, "gen": 0, "mcids": [3, 4] },
      "textRange": { "start": 10432, "end": 10449 },
      "spans": [
        { "text": "3.1 Reading order", "bbox": [72.0, 96.4, 310.2, 112.0],
          "font": "Inter-Bold", "size": 14, "flags": [] }
      ],
      "confidence": 1.0,
      "warnings": []
    },
    {
      "id": "t3", "type": "table", "page": 1, "bbox": [72, 200, 540, 380],
      "detector": "lattice",
      "cells": [ { "row": 0, "col": 0, "rowSpan": 1, "colSpan": 1, "header": true,
                   "text": "Tool", "bbox": [72, 200, 180, 220] } ],
      "markdown": "| Tool | Licence |\n|---|---|\n| docling | MIT |"
    }
  ],
  "chunks": [
    { "id": "c5", "text": "## 3.1 Reading order\n\nXY-cut splits…", "tokensApprox": 412,
      "blockIds": ["b12", "b13", "b14"],
      "headingPath": ["3 Algorithms", "3.1 Reading order"],
      "pages": [0, 1],
      "regions": [ { "page": 0, "bbox": [72, 96, 540, 700] }, { "page": 1, "bbox": [72, 72, 540, 180] } ],
      "textRange": { "start": 10432, "end": 12210 } }
  ],
  "dropped": [ { "reason": "invisible-text", "page": 3, "bbox": [0, 0, 612, 792], "chars": 2210 },
               { "reason": "running-header", "page": 1, "text": "ACME Annual Report" } ],
  "warnings": [ { "code": "W_TOUNICODE_GUESSED", "page": 4, "font": "F7", "hint": "…" } ]
}
```

The schema combines several sources:
- `textRange` offsets into a concatenated `markdown` or `text` output, as in Azure spans.
- A structure path, as in Docling's body tree and the Tagged PDF struct tree.
- Per-span boxes, needed for citation highlighting.
- An explicit `coord` block, which none of the vendors share.

**Ideal APIs (sketch)**

Rust:
```rust
let doc = Document::open(bytes, OpenOptions::default())?;
let out = doc.extract(ExtractOptions {
    reading_order: ReadingOrder::TaggedThenXyCut, // Tagged | XyCut | ContentStream
    coords: Coords { origin: Origin::TopLeft, unit: Unit::Pt, normalized: true },
    drop: Drop::INVISIBLE | Drop::RUNNING_HEADERS | Drop::ARTIFACTS,
    tables: TableMode::Lattice,                   // Off | Lattice | LatticeAndStream
    layout: None,                                 // Some(&dyn LayoutModel) from laminapdf-layout-onnx
    ocr: None,                                    // Some(&dyn OcrEngine)
    ..Default::default()
})?;
for chunk in out.chunks(ChunkOptions { max_tokens: 512, overlap: 64, split_on: SplitOn::Headings }) {
    index(chunk.text(), chunk.provenance()); // pages, regions, headingPath, textRange
}
let md: String = out.to_markdown(MarkdownOptions::default());
```

TypeScript (wasm):
```ts
import { openDocument } from "@laminapdf/core";
const doc = await openDocument(bytes);
const res = doc.extract({ readingOrder: "tagged-then-xycut", coords: { origin: "top-left", unit: "pt", normalized: true },
                          drop: ["invisible", "running-headers"], tables: "lattice" });
for (const c of res.chunks({ maxTokens: 512, splitOn: "headings" })) {
  await store.add({ text: c.text, meta: { pages: c.pages, regions: c.regions, path: c.headingPath } });
}
// errors: LaminaPdfError { code: "E_LIMIT_EXCEEDED", hint, didYouMean?, docsUrl }
```

CLI:
```sh
laminapdf extract report.pdf --format json --schema extract/1 --reading-order tagged-then-xycut > out.json
laminapdf extract report.pdf --format markdown --drop invisible,running-headers
laminapdf extract report.pdf --chunks --max-tokens 512 --jsonl | your-embedder
laminapdf schema extract/1          # print the JSON Schema
laminapdf explain W_TOUNICODE_GUESSED  # rustc --explain style
```

### Gaps
- I could not fetch pdfa.org, so I have no primary detail on the March 2026 AI FAQ's contents. I found no evidence of a formal ISO 32000 extension or a "PDF/R"-style AI-ready-PDF standard project in 2025–2026; searches surfaced none.
- I found no primary 2026 data on LlamaParse, Reducto, Google Document AI or unstructured.io benchmark standing, or on their bbox schemas beyond Unstructured's.
- TableFormer's licence was not confirmed.
- I could not confirm MinerU's exact "additional conditions", or its content_list bbox format; background knowledge says it was 0–1000-normalized in MinerU 2.x **[bg, unverified]**.
- Docling's OmniDocBench score was not found. OmniDocBench numbers come from secondary blogs, and the versions mix v1.5 and v1.6.
- Marker's 2.0 Apache relicensing rests on one secondary blog.
