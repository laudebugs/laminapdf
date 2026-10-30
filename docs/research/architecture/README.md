# Architecture research

How to architect lamina's apps and packages: a web PDF reader→editor on Cloudflare, a desktop reader→editor, and importable TypeScript/Rust libraries. Researched October 2026.

| Document | What it is |
| --- | --- |
| [app-and-package-architecture.md](app-and-package-architecture.md) | **The report.** Recommended reference architecture (one Rust core, three shells), alternatives and trade-offs per decision, risks, open questions and 19 proposed ADRs. |
| [notes/rendering.md](notes/rendering.md) | pdf.js vs PDFium (WASM/native) vs hayro vs MuPDF; how real viewers structure rendering. |
| [notes/desktop-shell.md](notes/desktop-shell.md) | Tauri 2 vs Electron vs native/Rust UIs; IPC for page bitmaps; updates, signing, distribution. |
| [notes/cloudflare.md](notes/cloudflare.md) | Workers static assets, R2, D1, Durable Objects, Workflows, Containers, Browser Run; limits and costs. |
| [notes/web-frontend.md](notes/web-frontend.md) | Framework choice, virtualisation, overlays, undo state, workers/COOP-COEP, OPFS/File System Access. |
| [notes/editing-architecture.md](notes/editing-architecture.md) | Document model, op log/undo, incremental vs full save, signatures/redaction ordering, collaboration, WASM memory. |
| [notes/repo-and-packaging.md](notes/repo-and-packaging.md) | Monorepo layout, Rust→WASM tooling, package design, versioning/publishing, CI tiers, licence hygiene. |
| [notes/hosted-ci-runners.md](notes/hosted-ci-runners.md) | Blacksmith vs CircleCI vs GitHub-hosted runners for Linux and Windows CI: prices, measured run times, monthly estimates (2026-10-30). Re-check before moving CI to hosted machines. |

Method: six parallel research threads (one per note), each citing primary sources where possible and marking unverified claims, then synthesised into the report. The plan that turns this into work is [`docs/plans/architecture.md`](../../plans/architecture.md).
