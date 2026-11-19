# Plans

Roadmaps and design docs for upcoming work. Name files `kebab-case.md`; include a status line (`Draft`, `In progress`, `Done`, `Abandoned`) at the top.

| Plan | Status |
| --- | --- |
| [Architecture](architecture.md): shared architecture, ADR list, spikes, Rounds A–D | Draft |
| [Roadmap](roadmap.md): sequence of all rounds and PRs across the plans | Draft |
| [PDF library](pdf-library.md): `@laminapdf/sdk` and `lamina` generation library (Rounds B1–B9) | Draft |
| [Web reader](web-app.md): PDF reader on Cloudflare (Rounds C1–C7) | Draft |
| [Desktop reader](desktop-app.md): Tauri 2 reader for macOS, Windows, Linux (Rounds D1–D7) | Draft |
| [Motion and interaction states](motion-and-states.md): cursors, states and motion for the apps and both sites (Round U7) | Draft |
| [Browser extension](browser-extension.md): the reader as an extension for Chrome, Edge, Firefox and Safari, after the web launch; nice to have (Round K) | Draft |
| [Full editor](full-editor.md): all 13 feature groups on web and desktop (Rounds E1–E18) | Draft |
| [Shareable libraries](shareable-libraries.md): contracts, worker client, AI extraction, CLI and MCP, agent docs, mobile (Rounds L1–L6) | Draft |
| [UI/UX](ui-ux.md): interaction model for web, desktop and mobile; command palette, selection, mobile layout, reader mode, trust surfaces (Round U) | Draft |
| [Paper design](paper-design.md): the chosen visual design, every screen designed and documented, Round U6 design system | Draft |
| [Website](website.md): the website at `laminapdf.com` and the reader at `app.laminapdf.com` (Round W) | Draft |
| [Discoverability](discoverability.md): search engines, AI answer engines, coding agents and registries find Lamina PDF: crawl rules, the public repository, package metadata, structured data, engine registration, a Context7 listing, pages for developers' questions, an answer-engine check (Round W3) | Draft |
| [Close a document](close-document.md): a Close button beside the file name that goes back to the home screen, and one close path that always asks about unsaved changes (Round C) | Accepted |
| [CI caching](ci-caching.md): compiled Rust shared between CI runs through sccache on Cloudflare R2, tried and removed because it saved too little, the waits for runners, and browser suites run only when the dependency graph reaches them (Round X) | Done |
| [Releases](releases.md): a daily automatic release after the nightly, npm trusted publishing, docs that match the released version (Round P) | Draft |
| [Quality](quality.md): docs that tell a story in plain English, examples readers can copy, maintainable code with checks that keep it so (Round Q) | Draft |
| [Trust and proof](trust-and-proof.md): what ten years of Hacker News say users lack, turned into work: published proof, checks for what files hide, extraction users can check, flowing layout, long reading, chained CLI jobs (Round T) | Draft |
| [Docs diagrams](docs-diagrams.md): diagrams that show how Lamina PDF works, drawn in the docs' style and following light, dark and forced colours (Round Q3) | Draft |
| [Docs examples](docs-examples.md): a Result tab with pictures of each page beside every example that makes a PDF, then "Edit and run" for six TypeScript examples (B7d, B7e) | Draft |
| [Docs redesign](docs-redesign.md): docs readers can trust from the first page to the last: command-line output from real runs, a reading path, results beside the code, first runs in Rust and on the command line, a sidebar by task after watching readers (Round Q4) | Draft |
| [E1 editing foundation](editor/e1-editing-foundation.md): the editor's first round: the edit session, undo, save and the save constraint, metadata in Info and XMP (E1b–E1i) | Draft |
| [E2 page organisation](editor/e2-page-organisation.md): insert, delete, duplicate, move, rotate and reverse pages, import, merge and split, page geometry and labels; what follows a page, the save constraint per op, the viewer across page edits (E2b–E2o) | Draft |
| [E3 navigation, attachments, inspection and extraction](editor/e3-navigation-attachments-inspection.md): bookmarks, named destinations, links, the contents page, the initial view, XMP, attachments and `/AF`, the object inspector, page and image export; the save constraint per op and the shells' differences (E3b–E3l) | Draft |
| [E4 markup and drawing annotations](editor/e4-markup-and-drawing-annotations.md): highlights, notes, text corrections, ink, shapes, lines, text boxes and callouts; the ops and where their appearance streams are generated, the viewer while drawing, the save constraint and the shells' differences (E4b–E4k) | Draft |
| [E5 review, stamps and interchange](editor/e5-review-stamps-interchange.md): replies and review state, filter and search, stamps, file attachments, audio comments read, XFDF, FDF and Markdown, flattening and the comment summary; the ops, the save constraint per op and the shells' differences (E5b–E5j) | Draft |
| [E6 adding content to pages](editor/e6-adding-content.md): text, symbols, images, shapes, logos, watermarks, backgrounds, headers, footers and Bates numbers as page content; each object's Form XObject and its identity after a save, the save constraint and the shells' differences (E6b–E6h) | Draft |
| [E7 editing existing content](editor/e7-editing-existing-content.md): retyping, restyling and replacing the text a file already has, and moving, cropping, recolouring, reordering, unclipping or deleting its images and drawings, over ADR 0022's IR; the ops and their fidelity reports, the save rules, line-level text editing, the conformance-claim op, the edit corpus and the round's ten todos | Draft |
| [E8 form filling and data](editor/e8-form-filling.md): filling every AcroForm field type with appearances that look right in every viewer, tab order, required and read-only, Acrobat's form helpers run natively without JavaScript, FDF, XFDF and JSON data, reset, programmatic fill, flattening, confirmed submit and XFA named plainly; the ops, the save constraint per op, the shells' differences and the round's twelve todos (E8c–E8n) | Draft |
| [E9 form authoring](editor/e9-form-authoring.md): creating, configuring, moving and deleting fields of every type, signature fields with locks and seed values, Acrobat's format, validation and calculation helpers written as data, calculation and tab order, accessible labels and `Form` elements, and field suggestions on flat forms by rules; the ops, the planner's field-level checks of authoring, the shells' differences and the round's nine todos (E9b–E9j) | Draft |
| [Element flow](element-flow.md): measured text bottoms and an optional vertical cursor for placing subsequent content | Draft |
