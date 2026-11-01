# Roadmap: lamina

- **Status:** Draft
- **Date:** 2026-10-28
- **Plans:** [architecture](architecture.md) · [PDF library](pdf-library.md) · [shareable libraries](shareable-libraries.md) · [web reader](web-app.md) · [desktop reader](desktop-app.md) · [full editor](full-editor.md) · [UI/UX](ui-ux.md)
- **Prior PRs (all merged):** #1 (PDF research and spike), #2 ([guidelines](../guidelines/README.md)), #3 ([architecture](architecture.md) and its [research](../research/architecture/app-and-package-architecture.md)). This roadmap and the product plans build on all three.

## What we are building

1. **Packages:** `laminapdf` (Rust) and `@laminapdf/sdk` (TypeScript) that generate PDFs, then read and edit them with the same `Document` API.
2. **A web reader** on Cloudflare, privacy-first, growing into a full editor.
3. **A desktop reader** (Tauri 2, macOS/Windows/Linux) with the same UI, updated from Cloudflare.
4. **A full editor** on web and desktop covering all 13 feature groups of the [feature inventory](../research/pdf/06-feature-inventory.md).

All four share one Rust core, so every operation is built once ([architecture §2](architecture.md#2-the-architecture-in-one-picture)).

## Sequence and dependencies

```
 #2 guidelines ─┐
 #3 architecture┴─► A foundations ─► B1 ─► B2 ─► B3 ─► B4 ─► B5 ─► B6 ─┬─► B7 (library 0.1 alpha) ─► B8 (0.2: UA/A)
                                                                       │
                                                                       ├─► B9 (text extraction) ──┐
                                                                       │                          ▼
                                                                       └─► C1 ─► C2 ─► C3 ─► C4 ─► C5 ─► C6 ─► C7 (web reader v0.1)
                                                                                        │     │
                                                                                        └► D1 └► D2 ─► D3 ─► D4 ─► D5 ─► D6 ─► D7 (desktop beta)

 C2a ─► U1a ─► C4a ─► U1b ─► U1c ─► C6a, C7a, U3a, U4a        (Round U: UI/UX; see the todo board for exact edges)
 B9b ─► U2b;  C5a + U2b + U3a ─► U2a;  C2b + C7a ─► U2c;  U2a + U2c + U3a + U4a ─► C7c
 B8a + C5a + U2b ─► U4b ─► U4c;  U3a ─► U3b;  C7c ─► U5a, U5c;  C7c + D7c ─► U5b

 C7 + D7 ─► E1 ─► E2 ─► E3 ─► E4 ─► E5 ─► E6 ─► E7 ─► E8 ─► E9 ─► E10 ─► E11 … E18 (editor phases 2–6)
```

- **A before everything.** Workspace, CI fast lane and `laminapdf-syntax`/`laminapdf-cos` ([architecture §5](architecture.md#5-rounds)).
- **Library first** (recommended in [architecture §8](architecture.md#8-open-questions-for-the-team), question 1): B1–B3 produce the model (with decryption, so readers can open password-protected files), ops and save path everyone needs.
- **Web reader after B6:** C1's PDFium spike loads `laminapdf-wasm` next to PDFium-WASM, so it starts once B6 has merged (its `hayro-parity` half can start earlier). C5's text layer needs B9 (text extraction), which runs alongside C1–C4. B7, B8 and B9 run in parallel with the reader rounds. Technically B9a (the `laminapdf-content` interpreter) only needs B2c, so it may start before B6 if people are free; B9c (bindings) needs B6b.
- **Desktop after the web packages:** D1 can start after C3; D2 needs `@laminapdf/ui` from C4.
- **UI/UX alongside the readers** ([UI/UX plan](ui-ux.md)): ADR 0024 (U1a) comes before `@laminapdf/ui` (C4a), the command registry and shell (U1b, U1c) right after it, and Round U adds selection, page modes, the phone layout and the network indicator before the web launch (C7a, C7c wait for them). Reader mode, the mobile spike and the product promises (pricing page, benchmark, opt-in telemetry) follow the launch; the pricing page and the benchmark also wait for the desktop beta (D7c).
- **Editor after both readers** ship v0.1, so phase 1 (core viewer) is done once and E-rounds add features to both shells and the packages at the same time.
- **Shareable libraries (L1–L6)** run alongside B and C: L1 (contracts) lands before the B5 builder so the first public API follows it; L2 follows B6; L3 (AI-ready extraction) follows B9; L4 (CLI and MCP) follows B5c and B9c; L6 (mobile spike, then a mobile plan) follows B5c and C2b ([shareable libraries §7](shareable-libraries.md#7-rounds)).
- **B8 and later E-rounds** can overlap with reader work as people allow; they don't block each other except where listed.

## Milestones

| Milestone | Rounds | What ships |
| --- | --- | --- |
| M0 Foundations | A | Monorepo, CI fast lane, `laminapdf-syntax` + `laminapdf-cos`, ADRs 0001/0002/0015/0019 |
| M1 Library alpha | B1–B7 | `lamina` and `@laminapdf/sdk` `0.1.0-alpha` (local and tagged, not published to registries yet): generate PDFs with fonts, shaping, images, vectors, links, outlines, metadata; open (including encrypted files)/edit/save basics; docs site |
| M2 Web reader v0.1 | B9, C1–C7, U1, U2, U3a, U4a | Text extraction in the library; hosted reader on Cloudflare: render, search, select, navigate, print, offline, accessible, no upload; command palette, column-aware copy, page appearance modes, phone layout, network indicator |
| M3 Desktop reader beta | D1–D7 | Signed installers for three OSes, file associations, tabs, auto-update channels |
| M4 Library 0.2 | B8 | Tagged PDF, PDF/UA-1, PDF/A-2/3 profiles, veraPDF-clean |
| M5 Organiser | E1–E3 | Save/undo, page organisation, bookmarks, attachments, metadata, extraction |
| M6 Annotator | E4–E5 | Markup, drawing, review threads, stamps, XFDF |
| M7 Content editor | E6–E7 | Add and edit text, images and vectors |
| M8 Forms and signing | E8–E10 | Fill and author forms, PAdES signing and validation |
| M9 Advanced | E11–E17 | Encryption on write, sanitisation, redaction, OCR/conversion, compare, accessibility, print production, advanced interactions |
| M10 Collaboration (optional) | E18 | Shared review on Durable Objects |

## All rounds and PRs

One row per round. Each round is split into one or more todos, one PR each, on the [todo board](../todos/README.md); editor rounds start with a feature-plan (or ADR) todo followed by implementation todos ([full-editor §7](full-editor.md#7-phases-and-rounds)).

| Round | Plan | Deliverable | Key ADRs and spikes |
| --- | --- | --- | --- |
| #2 | [guidelines](../guidelines/README.md) | Code organisation, Rust, TypeScript, git workflow | — |
| #3 | [architecture](architecture.md) | Architecture research and plan | — |
| This PR | roadmap + product plans | Plans with to-dos | — |
| A | [architecture §5](architecture.md#5-rounds) | Monorepo foundations | 0001, 0002, 0015, 0019 |
| B1 | [library](pdf-library.md#b1-decisions-and-spike) | Decisions and spike | 0003, 0005, 0006, 0007, 0018; `krilla-fit` |
| B2 | [library](pdf-library.md#b2-document-model) | Document model, decryption (read), corpus harness, fuzzing | — |
| B3 | [library](pdf-library.md#b3-ops-and-save) | Ops, undo, save planner | — |
| B4 | [library](pdf-library.md#b4-fonts-and-text) | Fonts, shaping, subsetting | — |
| B5 | [library](pdf-library.md#b5-builder-and-the-laminapdf-crate) | Builder and `laminapdf` crate | — |
| B6 | [library](pdf-library.md#b6-wasm-and-laminapdfsdk) | `laminapdf-wasm`, `@laminapdf/sdk`, runtime matrix | 0016 |
| B7 | [library](pdf-library.md#b7-docs-examples-and-first-release) | Docs site, examples, `0.1.0-alpha` (local) | 0017 |
| B8 | [library](pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa) | Tagged PDF, PDF/UA, PDF/A, `0.2.0-alpha` (local) | — |
| B9 | [library](pdf-library.md#b9-text-extraction) | `laminapdf-content` read-only interpreter, text extraction (after B6, before C5) | 0023 (proposed) |
| L1–L2 | [shareable libraries](shareable-libraries.md#l1-contracts) | Error, coordinate and façade contracts; worker client, disposal, range loading | 0026, 0027, 0028 |
| L3 | [shareable libraries](shareable-libraries.md#l3-ai-ready-extraction) | `laminapdf-extract`: reading order, tables, Markdown/JSON with bounding boxes | 0030 |
| L4 | [shareable libraries](shareable-libraries.md#l4-cli-and-mcp) | `laminapdf` CLI, MCP mode, distribution | 0031 |
| L5 | [shareable libraries](shareable-libraries.md#l5-docs-for-agents) | `llms.txt`, docs in packages, agent eval | — |
| L6 | [shareable libraries](shareable-libraries.md#l6-mobile) | `lamina-ffi` spike, mobile SDK plan | 0029 |
| C1 | [web](web-app.md#c1-rendering-spikes) | Rendering spikes | `pdfium-wasm`, `hayro-parity` |
| C2 | [web](web-app.md#c2-render-backend-and-adrs) | `laminapdf-render`, render packages | 0004, 0009, 0010, 0011 |
| C3 | [web](web-app.md#c3-viewer-core) | `@laminapdf/viewer` | — |
| C4 | [web](web-app.md#c4-app-shell-on-cloudflare) | `@laminapdf/ui`, `apps/web`, previews | — |
| C5 | [web](web-app.md#c5-text-selection-and-search) | Text layer, selection, search | — |
| C6 | [web](web-app.md#c6-navigation-print-and-properties) | Navigation, print, properties | — |
| C7 | [web](web-app.md#c7-accessibility-offline-and-launch) | Accessibility, PWA, launch | — |
| D1 | [desktop](desktop-app.md#d1-ipc-and-print-spikes) | IPC and print spikes | `desktop-ipc`, `desktop-print` |
| D2 | [desktop](desktop-app.md#d2-shell-and-native-core) | Tauri shell, engine process | 0008 |
| D3 | [desktop](desktop-app.md#d3-files) | File associations, recent files, watching | — |
| D4 | [desktop](desktop-app.md#d4-windows-tabs-menus-and-printing) | Windows, tabs, menus, printing | — |
| D5 | [desktop](desktop-app.md#d5-update-service) | `apps/updates` Worker + R2 | 0014 |
| D6 | [desktop](desktop-app.md#d6-signing-packaging-and-release-pipeline) | Signing and packaging | — |
| D7 | [desktop](desktop-app.md#d7-crash-reporting-and-beta) | Crash reporting, beta | — |
| U1 | [UI/UX](ui-ux.md#u1-shell-foundations) | ADR 0024, command registry and palette, inverted-L shell and modes | 0024 |
| U2 | [UI/UX](ui-ux.md#u2-reading-quality) | Selection surfaces and copy modes, reading-order corpus, page appearance modes | — |
| U3 | [UI/UX](ui-ux.md#u3-mobile-and-touch) | Phone and tablet layout; `mobile-shell` spike | 0025 (proposed); `mobile-shell` |
| U4 | [UI/UX](ui-ux.md#u4-trust-and-reflow) | Network indicator, reader mode (tagged, then approximate) | — |
| U5 | [UI/UX](ui-ux.md#u5-product-promises) | Pricing and promises page, reader benchmark, opt-in telemetry | — |
| E1 | [editor](full-editor.md#e1-editing-foundation) | Save, undo, metadata edits (Info + XMP in sync) | — |
| E2 | [editor](full-editor.md#e2-page-organisation) | Page organisation, content append API | — |
| E3 | [editor](full-editor.md#e3-navigation-attachments-inspection-and-extraction) | Navigation, attachments, extraction | — |
| E4 | [editor](full-editor.md#e4-markup-and-drawing-annotations) | Markup and drawing annotations | — |
| E5 | [editor](full-editor.md#e5-review-stamps-and-interchange) | Review, stamps, XFDF | — |
| E6 | [editor](full-editor.md#e6-adding-content-to-pages) | Add content | — |
| E7 | [editor](full-editor.md#e7-editing-existing-content) | Edit existing content | 0022 (proposed) |
| E8 | [editor](full-editor.md#e8-form-filling-and-data) | Form filling | 0021 (proposed) |
| E9 | [editor](full-editor.md#e9-form-authoring) | Form authoring | — |
| E10 | [editor](full-editor.md#e10-signatures) | Signatures | 0020 (proposed) |
| E11 | [editor](full-editor.md#e11-security-and-sanitisation) | Encryption on write, sanitisation | — |
| E12 | [editor](full-editor.md#e12-redaction) | Redaction | — |
| E13 | [editor](full-editor.md#e13-ocr-and-conversion) | OCR and conversion | 0013 |
| E14 | [editor](full-editor.md#e14-compare-revisions-and-batch) | Compare, revisions, batch | — |
| E15 | [editor](full-editor.md#e15-accessibility) | Accessibility | — |
| E16 | [editor](full-editor.md#e16-optimisation-and-print-production) | Optimisation, print production | — |
| E17 | [editor](full-editor.md#e17-advanced-interactions) | Advanced interactions | — |
| E18 | [editor](full-editor.md#e18-collaboration-hosted-service) | Collaboration | 0012; `auth` |
| Q | [quality](quality.md) | Docs as a story, readable examples, maintainability checks, reuse | — |

## How we track progress

- **The [todo board](../todos/README.md)** is the work queue: one file per PR in `ready/`, `ongoing/` or `done/`, each with explicit **Depends on** / **Blocks**. Every round below is split into todos there (for example B2 → B2a, B2b, B2c). Pick only todos whose dependencies are all done ([git workflow §2](../guidelines/git-workflow.md#pick-work-from-the-todo-board)).
- **Plans** hold the to-do checklists and link each round to its todos; tick them in the PR that completes them.
- **The capability matrix** ([full-editor §3](full-editor.md#3-tracking-the-capability-matrix)) is the source of truth for which of the ~180 operations work on which surface.
- **ADRs** in `docs/decisions/` record each decision in the round listed above.
- A plan's status moves Draft → In progress (first round merged) → Done (last round merged).

## Decisions made

- **Hosted service** (2026-10-28, project owner): lamina will be a hosted service with user login and cloud features (accounts, cloud storage and sync), not just a reference deployment. Local, on-device processing stays the default and needs no account; the reader v0.1 (M2) still uploads nothing. Accounts arrive with the first round that needs them, after the `auth` spike ([architecture §8](architecture.md#8-open-questions-for-the-team), Q2; ADR 0011).
- **Code signing** (2026-10-28, project owner): the owner, as the publisher, signs the desktop app for Windows, macOS and Linux (M3). Windows Artifact Signing's individual eligibility is US/Canada only, so the owner must confirm eligibility or use an OV certificate ([architecture §8](architecture.md#8-open-questions-for-the-team), Q5; ADR 0014).

- **Pricing** (2026-10-29, project owner): lamina will be priced; the code stays MIT OR Apache-2.0. The model and prices are still to be set; the UI/UX plan's fair-pricing rules apply ([UI/UX §5.9](ui-ux.md#59-product-promises-r18-r19), [U5a](../todos/ready/U5a-pricing-and-promises-page.md)).
- **Telemetry** (2026-10-29, project owner): optional, opt-in and off by default; none in web v0.1 ([U5c](../todos/ready/U5c-opt-in-usage-telemetry.md), ADR 0011).
- **Desktop speed and size gates** (2026-10-29, project owner): the research's numbers are the release gates: cold start < 300 ms to an interactive window and installer < 15 MB per platform ([desktop-app §4](desktop-app.md#non-functional-targets-and-release-gates)).
- **Publishing later** (2026-10-28, project owner): crates and npm packages stay local for now; registry names and publishing are a later to-do ([architecture: Publishing (later)](architecture.md#publishing-later)).
- **Shareable libraries** (2026-10-29, project owner): the `lamina-ffi` UniFFI spike ([L6a](../todos/done/L6a-lamina-ffi-spike.md)) runs right after B5c, in parallel with B6; UniFFI is pinned to `=0.32.x`; ADR 0016 ([B6a](../todos/done/B6a-lamina-wasm-and-adr-0016.md)) sets a size budget per wasm module, with 350 KB gz for the base module ([shareable libraries §11](shareable-libraries.md#11-open-questions)).
- **Domain** (2026-10-31, project owner): `laminapdf.com`. The reader goes to the apex (staging at `staging.laminapdf.com`), the docs site is at `docs.laminapdf.com`, desktop updates and downloads at `updates.` and `downloads.laminapdf.com` ([ADR 0011 §1](../decisions/0011-cloudflare-topology-and-privacy.md#1-topology), [web-app §8](web-app.md#8-open-questions) Q1).
- **Visual design** (2026-10-29, project owner): the **Paper** direction (calm, editorial: warm off-white chrome, ink text, deep teal accent, Newsreader and Source Sans 3) for every surface; screens and documentation in the [Paper design plan](paper-design.md), built in Round U6.
- **npm scope** (2026-10-31, project owner): the npm packages are published under `@laminapdf` (`@laminapdf/sdk`, `@laminapdf/wasm`, …); the TypeScript API package is `@laminapdf/sdk` (was `@lamina/pdf`, ADR 0016); crate names and the product name stay lamina ([architecture §8](architecture.md#8-open-questions-for-the-team), Q3; [P0](../todos/done/P0-npm-scope-laminapdf.md); ADR 0015).

## Decisions this roadmap assumes (recommendations, not yet decided)

- Packages ship before the reader ([architecture §8](architecture.md#8-open-questions-for-the-team), question 1).
- Readers ship before editing, so phase 1 of [06 §9](../research/pdf/06-feature-inventory.md#9-suggested-phasing) is built once for both shells.
- Architecture rounds B, C and D are each delivered as a stack of smaller PRs (B1–B9, C1–C7, D1–D7) to stay reviewable ([git workflow §5](../guidelines/git-workflow.md#5-pull-requests)).
