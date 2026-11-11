# T4a — ADR 0032: a layout engine for flowing pages (own engine over `Surface`, or Typst), with a spike

| Field | Value |
| --- | --- |
| ID | `T4a` |
| Status | `done` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | None |
| ADRs | Writes **0032** (flowing layout and the documents it takes in). Answers [pdf-library §9](../../plans/pdf-library.md#9-open-questions) questions 4 and 3. Amends [0031](../../decisions/0031-cli-and-mcp-contract.md) §2, §6 and §9 (a `build` command, its report and its MCP tool) and [0016](../../decisions/0016-package-design.md) §4 and §6 (the WASM module that carries the flow layer, and its budget) if the decision needs them. Follows [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (the builder writes into the overlay), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (licences), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space) and [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) (jobs) |
| Branch | `docs/t4a-adr-0032-layout-engine` |
| PR title | `docs(adr): 0032 flowing layout and document inputs, with the flow-layout spike` |
| Size | M (≈ 350–500 lines of ADR and docs; the spike is throwaway code outside the workspaces) |
| PR | [laudebugs/laminapdf#306](https://github.com/laudebugs/laminapdf/pull/306) |

## Goal

Decide whether flowing layout becomes a library goal and how Lamina PDF gets it: an engine of its own over `laminapdf-build`'s `Surface`, Typst embedded, or a mix of the two. ADR 0032 records the decision, and a short spike measures each route first. On Hacker News, control over pages is the generation request with the clearest gap: about 345 comments ask for page breaks, running headers, page numbers, contents pages, footnotes and repeated table headers. The two largest generation requests are writing Markdown (about 1,140 comments) and filling a template from JSON (about 560). Today the builder breaks lines inside one box and reports what overflows, and the library plan lists a flowing layout engine as a non-goal, so T4b–T4e wait on this decision.

## Context

- Evidence: the [generation note](../../research/hacker-news/notes/generation.md) §1.1, §1.2, §1.6, §2.4, §5 and §6 (points 1 and 8), and the report's [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json). Typst went from no mentions to 1,640 in 2023–26, and 94 comments describe it running in production from data. Talk of AI near generation rose from 6 to 11 to 263 comments, so a format an LLM can write matters too. "I make invoices by having a Typst template read line items from a JSON file." ([39672973](https://news.ycombinator.com/item?id=39672973))
- [pdf-library §2](../../plans/pdf-library.md#2-scope-and-non-goals) lists "a flowing layout engine" as a non-goal and names "Typst as layout backend" as one option; [§9](../../plans/pdf-library.md#9-open-questions) asks question 3 (a JSX or declarative layer) and question 4 (a layout engine, and whether Typst is the route).
- [Research 05 §8.3](../../research/pdf/05-creating-pdfs-from-code.md#83-build-versus-wrap-which-existing-work-to-stand-on) lists three layout options: an engine of our own, Typst as an embedded backend, or Taffy with our own pagination.
- The [`krilla-fit` spike](../../../spikes/krilla-fit/README.md) and [ADR 0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) found that krilla only returns finished bytes and renumbers every object at `finish`, and that its pages lose their tags when imported into a document. Typst's PDF export writes through krilla. The spike's method is the model here: one standalone Cargo project, the same document in every experiment, a WASM size probe, and `cargo deny` against the repository's `deny.toml`.
- The builder today: `Surface::text_box` (`crates/laminapdf-build/src/surface/mod.rs`, `surface/textbox.rs`) lays one paragraph in one font and size into a rectangle, and `TextLayout::overflow` says where the rest starts (`Overflow::Error` fails with `text-overflow`). `laminapdf-text`'s `break_lines` (`src/layout.rs`) breaks lines greedily at one size and reserves `LayoutOptions::justify`. `src/tag/` writes structure elements per page ("one doesn't continue on the next page"), and `ArtifactKind::Pagination` has no `/Subtype`.
- Size: [ADR 0016 §6](../../decisions/0016-package-design.md#6-budgets-per-module) records the text module at 1,074,722 bytes gzip against a 1,130,000 budget, 5.1 % headroom.
- Speed: [pdf-library §4](../../plans/pdf-library.md#4-requirements) asks for a 100-page text report in at most 500 ms after `init()`, and [B5d](../done/B5d-text-report-generation-performance.md) measured 39 ms natively. A Typst user reports a two-page invoice in under 100 ms ([45697377](https://news.ycombinator.com/item?id=45697377)). [T1c](../done/T1c-publish-performance-numbers.md) publishes generation timings.
- [ADR 0031](../../decisions/0031-cli-and-mcp-contract.md): verbs come first ([§2](../../decisions/0031-cli-and-mcp-contract.md#2-grammar)), and `render` already means rasterising pages ([L4e](../done/L4e-cli-render.md)); reports are versioned JSON ([§6](../../decisions/0031-cli-and-mcp-contract.md#6-json-output)); MCP is read-only unless started with `--allow-write` ([§9](../../decisions/0031-cli-and-mcp-contract.md#9-mcp-defaults-for-l4c)). The [trust and proof plan §9](../../plans/trust-and-proof.md#9-open-questions) asks whether output should be tagged by default (question 1) and whether generation becomes an MCP tool (question 2).
- [E6d](../ready/E6d-watermarks-headers-footers.md) stamps headers, footers and page numbers onto existing documents. One field syntax should serve E6d and the flow layer.
- [E13f](../ready/E13f-office-and-html-to-pdf.md) converts HTML and Office files as an opt-in server job (Browser Run, LibreOffice); the library doesn't take HTML.

## Scope

**In scope**

- ADR 0032 at `docs/decisions/0032-flowing-layout-and-document-inputs.md` (Context, Decision, Consequences, Alternatives considered). It decides:
  1. **The goal:** whether flowing layout is a library goal (pdf-library §9 question 4).
  2. **The route,** with the spike's numbers: (1) an engine of our own over `Surface`, with blocks, keep rules, page templates with running headers and footers, and "page X of Y" from laying out every page before drawing any (or from deferred content); (2) Typst embedded (`typst` and `typst-pdf`, Apache-2.0), whose finished bytes from krilla open as a `Document` (option A of ADR 0005 §6); (3) a hybrid: Typst's layout drawn through `Surface`, or our engine with Typst as an optional front end in an edge crate.
  3. **The layout model,** if the route is ours: block kinds, inline spans, page templates and their fields (shared with E6d), the pagination rule (greedy with keep rules, or optimal), widow and orphan defaults, and how "page X of Y" and a contents page get their numbers (a bounded number of layout passes). It also lists what stays out (floats, columns, hyphenation, justification, maths, HTML and CSS), each with a later home or "not planned".
  4. **Tagging:** whether flowing output is tagged by default (plan §9 question 1, narrowed to flowing output); which block becomes which structure type; running headers and footers as `Pagination` artifacts with a `/Subtype`; elements that continue across pages.
  5. **Placement:** a `flow` module in `laminapdf-build` or a crate of its own ([code organisation §11](../../guidelines/code-organization.md#11-when-to-split-a-crate-or-package-and-when-not-to)); where the Markdown and template front ends live; which WASM module carries them, with a budget, as an ADR 0016 §6 amendment for the owner to approve.
  6. **Inputs:** the Markdown dialect (CommonMark plus which extensions: tables, footnotes, strikethrough, task lists, front matter), what is dropped (raw HTML, maths) and how a drop is reported. The JSON document format (proposal: `laminapdf.document/1`, with a JSON Schema committed and versioned under ADR 0031 §6's rule) and the template language for data: placeholders inside the document JSON with JSON Pointer paths, a text template engine such as MiniJinja, or Typst's. Which format the docs and the MCP tool offer coding agents, and whether that document tree is the declarative layer of pdf-library §9 question 3 (a JSX layer would map onto it later).
  7. **Surfaces:** the CLI verb (proposal: `laminapdf build`), its `laminapdf.build/1` report, and a generation tool in `laminapdf mcp` under `--allow-write` (an ADR 0031 amendment; plan §9 question 2). Also the TypeScript shape (typed objects that match the JSON form) and whether long layouts run as a job ([ADR 0028 §3](../../decisions/0028-facade-rules-jobs-and-cancellation.md#3-jobs)).
  8. **Fonts:** what the front ends use when the caller gives none: the standard 14, or a bundled face. pdf-library §9 question 2 stays open unless the ADR answers it.
  9. **Budgets:** time for a two-page invoice and a 100-page report, natively and in Node, and the WASM bytes the flow layer and the front ends may add.
- The spike in `spikes/flow-layout/`: throwaway, outside both workspaces ([code organisation §12](../../guidelines/code-organization.md#12-spikes-vs-product-code)), described under Tasks.
- The index and plan updates listed under Docs to update.

**Out of scope** (and which todo covers it)

- The flow layer, as this todo reshaped it: [T4f](../done/T4f-flow-builder-groundwork.md) (builder groundwork), [T4g](../done/T4g-flow-styled-spans.md) (styled spans), [T4b](../done/T4b-flow-layout-core.md) (paragraphs, keep rules, page templates, page X of Y), [T4c](../done/T4c-flow-tables-notes-contents.md) (tables) and [T4h](../done/T4h-flow-notes-contents-outline.md) (footnotes, contents pages, the outline).
- The front ends: [T4d](../done/T4d-markdown-documents.md) (Markdown in Rust), [T4i](../done/T4i-build-command-and-bindings.md) (`laminapdf build`, the MCP tool, the TypeScript `markdown` call), [T4e](../done/T4e-json-documents-and-templates.md) (JSON documents and templates) and [T4j](../ready/T4j-documents-in-typescript.md) (documents in TypeScript).
- HTML and CSS to PDF: [E13f](../ready/E13f-office-and-html-to-pdf.md).
- Publishing the timings: [T1c](../done/T1c-publish-performance-numbers.md).

## Tasks

- [x] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices` for the spike and `/writing` for the ADR
- [x] Read the Context items, ADRs 0005 §6, 0016 §4–§7, 0027, 0028 §3 and 0031, and research 03 [§7.5](../../research/pdf/03-text-and-fonts.md#75-hyphenation-knuthliang)–[§7.6](../../research/pdf/03-text-and-fonts.md#76-justification) (hyphenation and justification) and [§9.5](../../research/pdf/03-text-and-fonts.md#95-generating-tagged-pdf-from-code) (generating tagged PDF)
- [x] Check that ADR 0032 is still free; [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs) reserves it for this todo
- [x] Set up `spikes/flow-layout/`: a standalone Cargo project with its own `[workspace]` table and `Cargo.lock`, the root `Cargo.toml`'s `wasm-release` profile copied in, path dependencies on `crates/laminapdf`, `crates/laminapdf-build` and `crates/laminapdf-text`, and `typst` and `typst-pdf` pinned with `=` at the current release (record the version and the date)
- [x] Inputs in `spikes/flow-layout/inputs/`: an invoice as JSON with 40 line items (so it runs to two pages); a seeded 100-page report with headings, paragraphs, a table that crosses pages, a running header and a "Page X of Y" footer; one Arabic and one Devanagari paragraph; the Noto fonts from `fixtures/fonts/` (OFL 1.1); and the same documents as Typst sources (the invoice reads its JSON with Typst's `json()`)
- [x] Experiment A, our own engine (`src/own.rs`): a minimal paginator over `laminapdf-build`. It flows paragraphs with `Surface::text_box` and its overflow (the builder can't measure without drawing, which the README records), keeps headings with the next block, repeats a table's header row, draws headers and footers in a second pass once the page count is known, and tags with `Surface::tag` and `Surface::artifact`. Record what the builder lacks (elements across pages, several fonts and sizes in one paragraph, artifact subtypes)
- [x] Experiment B, Typst embedded (`src/typst_pdf.rs`): a `World` with in-memory sources, the Noto fonts and a fixed `today()`; `typst::compile`, then `typst-pdf` with a fixed identifier and timestamp, and tagged output if the release offers it. Open the bytes with `laminapdf::Document::open_bytes`, save an edit incrementally, and try to append the pages to an existing document
- [x] Experiment C, hybrid (`src/typst_frames.rs`): Typst's laid-out frames drawn through `Surface` (glyph runs through the builder's font registry, shapes, images, links) without `typst-pdf`. Stop at the first page that renders, and record what doesn't map (Typst's tags, gradients, fonts Typst loaded itself)
- [x] Measure, and write one table per question in the spike README:
  - [x] WASM bytes (gzip): each experiment as a `wasm32-unknown-unknown` cdylib built as `cargo xtask wasm` builds the modules (the `wasm-release` profile, then wasm-opt), less the text module's measurement; and `pulldown-cmark` and one template engine on their own, for T4d and T4e
  - [x] Time for the two-page invoice and the 100-page report: native release, best of 30, and Node 24 through the WASM build
  - [x] Tagged output: `cargo xtask verapdf --dir <out>` under PDF/UA-1 for each output; whether a paragraph across a page break stays one `P`; whether running headers are artifacts
  - [x] Shaping: the Arabic and Devanagari paragraphs extracted by pdf.js and pdftotext (`tools/differential/text.mjs`) equal the input, and the glyphs match `hb-shape` as the goldens in `fixtures/shaping/` do (pdf.js and pdftotext done; `hb-shape` wasn't installed, so the glyph comparison wasn't run, as the spike's README says)
  - [x] Determinism: byte-identical output across 30 runs, and between native and wasm32
  - [x] The object graph: whether generated objects live in a `Document`'s overlay, so that "generate then edit" saves incrementally and flowing pages can be appended to an open document
  - [x] Licences and upkeep: `cargo deny --manifest-path spikes/flow-layout/Cargo.toml --config deny.toml check licenses bans sources`; any licence outside the allow-list (OFL font data, if a Typst crate bundles fonts); the dependency count; Typst's release cadence and the breaking changes between its last two minor releases
- [x] Write the spike README: date, question, method, measurements, findings and a recommendation
- [x] Write ADR 0032 with every decision in Scope, each backed by the spike's numbers, and the alternatives with the reasons they lost: krilla used directly, Taffy with our own pagination, HTML and CSS through a browser, and optimal (Knuth–Plass style) page breaking
- [x] Reshape T4b–T4e to the decision before review: their scope, tasks, codes, placement and sizes, splitting each one estimated over 800 lines into todos of at most L (their sizes today are XL). If Typst wins, rewrite them as the integration it needs (fonts, tagging checks, the edit path) and say so in the PR description
- [x] Review with the owner, and record the sign-off in the ADR's status line
- [x] The index and plan updates (Docs to update)

## Acceptance criteria

- [x] ADR 0032 is Accepted with the owner's sign-off, and each decision listed in Scope cites a number or finding from the spike
- [x] The spike README answers each measured question with numbers, names the machine, toolchain and Typst version, and ends with its recommendation
- [x] The library plan marks §9 questions 3 and 4 as answered, with a link to ADR 0032, and §2's non-goal matches the decision
- [x] T4b, T4c, T4d and T4e match the ADR (scope, codes, placement, sizes), and the board is regenerated (`cargo xtask todos`)
- [x] `cargo xtask check docs` passes

## Post-merge checks

None.

## Tests

Spike only; no production code. `cargo xtask check docs` checks the links and the board.

## Docs to update

- [x] `docs/decisions/README.md`: the ADR 0032 row
- [x] [Architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs): the reserved 0032 row gets its link and status; [§4](../../plans/architecture.md#4-spikes-to-close-open-questions): a `flow-layout` row with the spike's result
- [x] [pdf-library](../../plans/pdf-library.md): §2's non-goals and §9 questions 3 and 4 (and 2, if the ADR settles default fonts)
- [x] ADR 0031 (an amendment for `build`, `laminapdf.build/1` and the MCP tool) and ADR 0016 §4 and §6 (the module and its budget), if the ADR decides them
- [x] [Trust and proof plan](../../plans/trust-and-proof.md) §T4 and §9 (questions 1 and 2, if the ADR answers them for flowing output)
- [x] `CLAUDE.md`: Round T names ADR 0032 as proposed, and the spike is listed; it joins the accepted ADRs on sign-off
- [x] `spikes/flow-layout/README.md`

## Risks and open questions

- **Typst moves fast.** Its crates are 0.x and change their API between minor releases. A dependency would tie Lamina PDF to that cadence, as krilla would have (ADR 0005 §6). The spike records the churn between two releases.
- **Typst's output is finished bytes.** Option B gives up the overlay: edits happen after reopening, and appending to an open document needs page import with a structure-tree merge, which the krilla-fit spike didn't build. Option C keeps the overlay but has to map every frame item and tag, and may cost as much as option A.
- **Size.** A Typst build may be larger than any budget the text module can take. The spike measures before anyone argues.
- **Our own engine is real work.** Keep rules, tables across pages, footnotes and contents pages are where layout engines spend years. T4b and T4c limit the scope to what the evidence asks for, and the ADR names what stays out.
- **One machine.** The times come from one machine: a Linux x86_64 cloud container, not the Mac the other spikes used. The ADR and the README state it and compare ratios rather than absolutes.
- **The time box.** The spike answers the listed questions and stops; experiment C stops at the first page that renders.
- **The owner decides** the route, whether flowing output is tagged by default, whether `laminapdf mcp` gets a generation tool, the default fonts, and any budget increase.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
