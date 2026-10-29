# Shareable libraries research

How to architect and specify lamina's shareable libraries (a shared Rust core with a TypeScript package, mobile SDKs and a CLI) so they serve frontend and mobile engineers, backend and command-line users, and LLMs and AI agents. Researched October 2026.

| Document | What it is |
| --- | --- |
| [shareable-libraries.md](shareable-libraries.md) | **The report.** Gap analysis, core and bridge architecture, TypeScript DX, mobile ergonomics, LLM-first API and AI extraction, CLI and MCP; ideal-API sketches in Rust, TypeScript, CLI, Swift and Kotlin; proposed work items and ADRs 0026–0031. |
| [notes/ecosystem-gap-analysis.md](notes/ecosystem-gap-analysis.md) | TypeScript, Rust, mobile and CLI PDF libraries in 2026: licences, maintenance, flaws, why developers leave them. |
| [notes/rust-core-bridge.md](notes/rust-core-bridge.md) | UniFFI 0.32, one façade for wasm-bindgen and UniFFI, binary size, range loading, concurrency, rendering, signing, panic safety. |
| [notes/typescript-web-dx.md](notes/typescript-web-dx.md) | Loading wasm per runtime and bundler, worker offloading, tree-shaking, disposal, strict types, how other libraries' APIs look. |
| [notes/mobile-sdk.md](notes/mobile-sdk.md) | PDFKit, PdfRenderer, androidx.pdf, Nutrient; tiles, memory limits, stylus, Swift/Kotlin concurrency, packaging. |
| [notes/llm-api-and-ai-extraction.md](notes/llm-api-and-ai-extraction.md) | What makes LLMs misuse APIs, docs for agents; PDF → Markdown/JSON for RAG, reading order, tables, bounding boxes, licences. |
| [notes/cli-spec.md](notes/cli-spec.md) | qpdf, mutool, pdftk grammars; CLI conventions; batch; MCP 2026-07-28; Rust CLI crates and distribution. |

Method: six parallel research threads (one per note), then a synthesis into the report. Some hosts were blocked by the research sandbox; the notes say which claims were read from mirrors (GitHub sources, registries) and mark unverified prior knowledge. The plan that turns this into work is [`docs/plans/shareable-libraries.md`](../../plans/shareable-libraries.md).
