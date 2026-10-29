# Product and UX research

What a fast, intuitive cross-platform PDF app needs, and how lamina's architecture turns that into an advantage: competitors and their friction, personas, UI/UX patterns, rendering engines and SDKs, differentiation and a phased MVP. Researched October 2026.

| Document | What it is |
| --- | --- |
| [pdf-app-product-and-ux.md](pdf-app-product-and-ux.md) | **The report.** Friction audit of incumbents, three personas, docked chrome + modes + command palette, mobile layout, reflow, engines under lamina's licence rule, three differentiators, a four-phase roadmap and 19 recommendations (R1–R19) mapped to todos. |
| [notes/competitive-teardown.md](notes/competitive-teardown.md) | Acrobat, Foxit, PDF Expert, GoodNotes, LiquidText, Preview, SumatraPDF, Okular, browser viewers, web tools, Stirling-PDF: pricing, complaints, praised UX, gaps. |
| [notes/personas-workflows.md](notes/personas-workflows.md) | Task-frequency evidence (the 70/20/8/2 split is unverified), academic, business/legal and casual mobile personas, redaction incidents, eIDAS, accessibility. |
| [notes/ui-ux-patterns.md](notes/ui-ux-patterns.md) | Progressive disclosure, selection menus, command palettes (cmdk), thumb-zone and sheets, reflow, page organiser, perceived performance, dark mode. |
| [notes/engines-sdks.md](notes/engines-sdks.md) | PDFium, MuPDF, PDF.js, Poppler, Apryse, Nutrient, Foxit SDK, hayro, pdf_oxide, lopdf, pdf-rs, krilla: licence, size, memory, incremental save, extraction. |
| [notes/differentiation-mvp.md](notes/differentiation-mvp.md) | On-device OCR and model licences, local embeddings, reflow feasibility, privacy evidence, business models, MVP phases and metrics. |

Method: five parallel research threads (one per note), each citing sources and tagging claims as fact, report, estimate or opinion, then synthesised into the report. Several primary sites were unreachable from the research session (listed in the report); claims resting on search snippets should be re-checked before they go into an ADR. The plan that turns this into work is [`docs/plans/ui-ux.md`](../../plans/ui-ux.md).
