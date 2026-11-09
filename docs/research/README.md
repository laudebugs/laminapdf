# Research

Deep dives and research notes. Each topic gets its own subfolder.

| Topic | Summary |
| --- | --- |
| [PDF](pdf/) | How PDF works, end to end: file structure, imaging model, text & fonts, interactivity & security, creating PDFs from code, and a full feature inventory. |
| [Architecture](architecture/) | How to architect the web app (Cloudflare), the desktop app and the TS/Rust packages: one Rust core, three shells. Report, six research notes, a hosted CI runner pricing note and 19 proposed ADRs. |
| [Libraries](libraries/) | Shareable libraries across TypeScript, Rust, mobile and the CLI for humans and AI agents: gap analysis, bridges, DX, extraction, CLI/MCP. Report, six notes and ADRs 0026–0031 proposed. |
| [Product and UX](product/) | What a fast, intuitive PDF app needs on desktop, web and mobile: competitor friction, personas, UI/UX patterns, engines and SDKs, differentiation and MVP phases. Report and five notes; feeds the [UI/UX plan](../plans/ui-ux.md). |
| [Docs site](docs-site/) | How the docs should show what an example makes, and let readers change it: pictures of the output, live samples, hosted sandboxes, a hosted bundler, a playground of our own. Recommends page pictures now and a small runner of our own later. |
| [Hacker News](hacker-news/) | What ten years of Hacker News (Nov 2016 – Nov 2026, 3,755 stories, 56,907 comments) say PDF users want from readers, editors, extraction and generation, and what lets them down. Report, five area notes and a method note, with the scripts in `tools/hn-research/`; feeds the [trust and proof plan](../plans/trust-and-proof.md). |
| [Browser extension](browser-extension/) | Whether and how to ship the reader as an extension for Chrome, Edge, Firefox, Safari and the Chromium forks: taking PDFs over in each engine, permissions and privacy, stores and tooling, competitors. Recommends bundling the reader and shipping Chromium first; feeds the [browser extension plan](../plans/browser-extension.md). |
