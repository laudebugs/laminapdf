# L3a — ADR 0030 and `lamina-extract`: blocks, reading order, headings, lists, header/footer removal, hidden-text flags

| Field | Value |
| --- | --- |
| ID | `L3a` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction), principles [§4](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031) (3, 5, 7) |
| Round / milestone | Round L3 (shareable libraries) — AI-ready extraction |
| Depends on | [B8a](./B8a-structure-tree.md), [B9d](../done/B9d-text-runs-quads-and-page-text.md), [L1a](../done/L1a-contract-adrs.md), [U2b](../done/U2b-reading-order-ground-truth.md) |
| Blocks | [L3b](./L3b-extract-tables-and-serialisers.md) |
| ADRs | Writes **0030** (extraction output `lamina.extract/1` and model plug-in policy); follows 0026 (errors), 0027 (coordinates, top-left points), 0028 (jobs, cancellation) from [L1a](../done/L1a-contract-adrs.md), 0023 (from [B9a](../done/B9a-lamina-content-and-adr-0023.md)), [0001](../../decisions/0001-core-language-and-licence.md) (licences, bytes in / bytes out), [0003](../../decisions/0003-reader-tolerance-and-limits.md) (warnings, `Limits`), [0018](../../decisions/0018-test-strategy.md) (goldens, fuzz) |
| Branch | `feat/l3a-lamina-extract` |
| PR title | `feat(extract): ADR 0030 and lamina-extract with blocks, reading order and hidden-text flags` |
| Size | L (≈ 1,000–1,400 lines incl. ADR; split at the line in Risks if it grows) |
| PR | — |

## Goal

Turn B9's glyph runs into document structure an LLM or a RAG pipeline can use: blocks (paragraphs, headings, list items) in reading order, each with a bounding box in top-left points, page index and number, structure path, and visibility flags. Reading order comes from the structure tree when the file is tagged and the tree is plausible, and from XY-Cut++ otherwise. Running headers and footers are removed, hyphenated line breaks are joined, and hidden text is flagged and dropped from the text output with a record of what was dropped. ADR 0030 fixes the `lamina.extract/1` output model and the rule that models are optional plug-ins outside core. Tables, serialisers and bindings follow in [L3b](./L3b-extract-tables-and-serialisers.md); model traits in [L3c](./L3c-model-plugins.md).

## Context

- [U2b](../done/U2b-reading-order-ground-truth.md): the hand-checked reading-order corpus and metric. This crate's reading order is measured with that metric, and the reader's copy and reader mode ([U2a](./U2a-selection-surfaces-and-copy-modes.md), [U4b](./U4b-reader-mode-from-structure-tree.md)) should use this crate's ordering rather than a second implementation.
- [Report: rule-based extraction in core, models as licence-clean plug-ins](../../research/libraries/shareable-libraries.md#rule-based-extraction-in-core-models-as-licence-clean-plug-ins): rules reach about 0.90 reading-order accuracy at 15 ms a page (OpenDataLoader's vendor benchmark); fewer than 20% of PDFs are tagged; the proposed JSON shape; the hidden-text security point.
- [AI extraction note §B](../../research/libraries/notes/llm-api-and-ai-extraction.md#b-ai-consumption-primitives-the-2026-landscape-algorithms-bboxprovenance-schema-tagged-pdf-and-rule-based-vs-model-based): algorithms (XY-cut, XY-Cut++, Docstrum), header/footer and de-hyphenation heuristics, bbox conventions of Textract, Azure, Docling and OpenDataLoader, and the full proposed `lamina.extract/1` example.
- [Report: coordinates must be an explicit, typed field](../../research/libraries/shareable-libraries.md#coordinates-must-be-an-explicit-typed-field) and ADR 0027 (from [L1a](../done/L1a-contract-adrs.md)): points, top-left origin, y down; 0-based `index` plus 1-based `number`; record CropBox, `/Rotate` and `/UserUnit`.
- [Report: errors and schemas as a single contract](../../research/libraries/shareable-libraries.md#errors-and-schemas-as-a-single-contract) and ADR 0026: document text never appears in `message` or `hint`.
- [Research 03 §8.1](../../research/pdf/03-text-and-fonts.md#81-how-extractors-work), [§8.5](../../research/pdf/03-text-and-fonts.md#85-layout-analysis-and-ml-document-understanding-state-of-the-art-2026), [§9.1](../../research/pdf/03-text-and-fonts.md#91-the-model), [§9.3](../../research/pdf/03-text-and-fonts.md#93-language-alt-text-reading-order), [§2.5 text rendering modes](../../research/pdf/03-text-and-fonts.md#25-text-rendering-modes-tr).
- ISO 32000-2 §14.7 (logical structure, `/K`, `/ParentTree`, `RoleMap`), §14.8.2.2 (artifacts, `/Type /Pagination` with `/Subtype /Header|/Footer`), §14.8.4 (standard structure types), §14.9.4 (`/ActualText`), §14.9.5 (`/MarkInfo`), §9.3.6 (text rendering mode).
- XY-Cut++ ([arXiv 2504.10258](https://arxiv.org/abs/2504.10258)); Docstrum (O'Gorman 1993) for line and block grouping.
- [B9b](../done/B9b-text-extraction-core.md) and [B9d](../done/B9d-text-runs-quads-and-page-text.md) (glyph runs, quads, MCIDs, `ActualText`, `Tr 3` flag) and [B8a](./B8a-structure-tree.md) (the read-only `StructTree` view in `lamina-model`).

## Scope

**In scope**

- **ADR 0030** `docs/decisions/0030-extraction-output-and-model-plugins.md`: the `lamina.extract/1` model (top-level `schema`, `generator` with `mode` and `models`, `source` with `sha256`, `pageCount`, `tagged`, `structSource`; `coord` block; `pages`; `blocks`; `dropped`; `warnings`; `chunks` reserved for L3b); versioning (new keys allowed within `/1`, consumers ignore unknown keys; removing or changing a key needs `/2`); determinism (stable block IDs, no timestamps, `BTreeMap` ordering); rules in core and models as plug-ins in edge crates; licence rules for model weights (permissive code *and* weights, never in core crates or in the repository, pinned by hash; MinerU, Marker/Surya weights and PyMuPDF excluded per ADR 0001); visibility flags and the "hidden text never in text output" rule.
- **`crates/lamina-extract`** (layer `core`, `#![forbid(unsafe_code)]`, bytes in / bytes out), depending on `lamina-text` (B9b) and the `lamina-model` structure view (B8a). No new third-party dependency unless licence-checked per [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md).
- **Output types** (serde derives behind a `serde` feature; serialisers are L3b): `Extraction { source, coordinates, pages: Vec<PageInfo>, blocks: Vec<Block>, dropped: Vec<Dropped>, warnings }`; `PageInfo { index, number, label, width, height, rotate, user_unit, box_used }`; `Block { id, kind: BlockKind, text, page, bbox: RectPt, bbox_norm, struct_path: Vec<String>, struct_ref: Option<StructRef { obj, gen, mcids }>, spans: Vec<Span>, flags: Flags, confidence }`; `BlockKind::{Heading { level }, Paragraph, ListItem { marker, depth }, Caption, Footnote, Other}` (`Table` added in L3b); `Span { text, bbox, font, size, flags }`.
- **Lines and blocks:** group runs into lines by baseline, direction and font size; group lines into blocks by vertical gap relative to line height and by left-edge alignment (Docstrum-style, rule-based). Right-to-left runs keep logical order (from B9b); vertical writing gets a documented simple path (columns as lines) and a warning when ambiguous.
- **Reading order:**
  1. *Structure tree first* when `/MarkInfo /Marked true` and the tree passes plausibility checks (MCIDs resolve, at least a configurable share of visible characters are under structure elements, no element order that jumps backwards across columns more than a threshold). Walk `/K` in order, apply `RoleMap` to standard types, map `(page, MCID)` to runs, and emit `structSource: "tagged"`. Untagged content outside artifacts is appended by XY-Cut++ on its page with a warning.
  2. *Otherwise XY-Cut++*: pre-mask cross-column elements (wide titles, full-width figures), recursive projection-profile splits on block bboxes, then merge semantically adjacent blocks; `structSource: "xycut"`.
  3. *Content order* as an option (`ReadingOrder::ContentStream`) for debugging and comparison.
- **Headings** from font-metric clusters: body size is the mode by character count; larger sizes and bold weights cluster into up to six levels; tagged `H1`–`H6`/`H` win when the tree is used.
- **Lists** from bullet glyphs (`•`, `◦`, `▪`, `–`, `*` and similar) and numbering patterns (`1.`, `1)`, `a.`, `(a)`, `i.`) with a hanging indent; depth from indent; tagged `L`/`LI`/`Lbl`/`LBody` win when used.
- **Running header/footer removal:** `Artifact` marked content is authoritative (reason `artifact`); otherwise text in the top and bottom bands repeated at a similar y on at least N pages (digits normalised, so page numbers match) and page-number patterns are dropped (reason `running-header`, `running-footer`, `page-number`). Removal is on by default and can be turned off.
- **De-hyphenation and ligatures:** join a line ending in `-` (U+002D) or a soft hyphen (U+00AD) with the next line when it starts with a lowercase letter; keep the hyphen otherwise. Decompose ligatures U+FB00–FB06 when no `ActualText` applies. No dictionary in core (recommendation; revisit if fixtures show over-joining).
- **Visibility flags per span**, with the dropped list recording reason, page, bbox and character count: `invisible` (`Tr 3` and `Tr 7`), `clipped` (glyph quad outside the clip bbox tracked by `lamina-content`; bbox-level approximation, documented), `offPage` (outside the CropBox), `lowContrast` (fill colour within a documented distance of white when no fill is painted beneath it in the same region; heuristic, documented), `tiny` (font size below a threshold). Flagged text is never in `Block::text`; it stays available in spans only when the caller asks (`Drop` options).
- **Coordinates** per ADR 0027: `RectPt` in top-left points relative to the CropBox after `/Rotate`, scaled by `/UserUnit`; `bboxNorm` in [0, 1] of the CropBox (serde name; `bbox_norm` in Rust); the `coordinates` block (ADR 0027's shape: `origin`, `unit`, `box`, `rotate`, `userUnit`) records unit, origin, box and `rotate`.
- **API:** `lamina_extract::extract(&Document, &ExtractOptions) -> ExtractJob`, a resumable job per ADR 0028 (`step(Budget)` returns progress per page or the `Extraction`; `CancelToken` polled per page and inside XY-Cut recursion); `ExtractOptions { pages, reading_order, drop, headers_footers, dehyphenate, coords }` with `Default`. Options are plain owned data (UniFFI-compatible per ADR 0028).
- **Limits and warnings:** soft limits (max blocks per page, XY-Cut recursion depth, header/footer comparison window) drop work with a warning; new codes in `fixtures/codes.json` (e.g. `extract-struct-tree-ignored`, `extract-untagged-content`, `extract-reading-order-ambiguous`, `extract-limit-blocks`), each with an `explain` entry per [L1b](../ongoing/L1b-error-contract-codegen.md) if merged.
- **Fuzz target** `extract` (open + extract every page under default limits).

**Out of scope** (and which todo covers it)

- Tables (lattice and stream), Markdown/JSON/chunk serialisers, JSON Schema file, façade, WASM and TS exposure, benchmark scores: [L3b](./L3b-extract-tables-and-serialisers.md).
- `LayoutModel` and `OcrEngine` traits and the model edge crate: [L3c](./L3c-model-plugins.md).
- CLI `lamina extract`: [L4b](./L4b-cli-batch-render-schema.md).
- Formulas, figure descriptions, form-field values in output: later (record in ADR 0030 as future block kinds).
- Editing the structure tree or auto-tagging: full-editor E15.

## Tasks

- [ ] Load `/rust-best-practices` and follow [docs/guidelines/rust.md](../../guidelines/rust.md) (no `unwrap`/`expect`/`panic`/indexing in library code, `thiserror` errors, deterministic output); read the Context links and ISO 32000-2 §14.7–14.9
- [ ] Confirm ADR number 0030 is still free on the board; write `docs/decisions/0030-extraction-output-and-model-plugins.md` (status Proposed → Accepted in this PR); add it to `docs/decisions/README.md` and the [architecture §3 table](../../plans/architecture.md#3-decisions-to-record-adrs)
- [ ] Create `crates/lamina-extract` (`Cargo.toml` with workspace fields, `[lints] workspace = true`, `[package.metadata.lamina] layer = "core"`, feature `serde`); register it in the root `Cargo.toml`
- [ ] Modules: `model.rs` (output types), `options.rs`, `job.rs` (ADR 0028 job), `lines.rs`, `blocks.rs`, `order/{mod,tagged,xycut}.rs`, `headings.rs`, `lists.rs`, `furniture.rs` (headers, footers, page numbers), `text_fix.rs` (de-hyphenation, ligatures), `visibility.rs`, `coords.rs`
- [ ] If B9b's `TextRun` lacks fill colour, clip bbox or render mode per glyph, add them in `lamina-text` (small change, its own commit) rather than re-interpreting content here
- [ ] Extend B8a's read-only `StructTree` view only if needed (`RoleMap` resolution, `/K` with MCR/OBJR kids, `/Pg` inheritance), keeping it read-only
- [ ] Add warning codes to `fixtures/codes.json` and the Rust enums; paired tests per soft limit
- [ ] Fixtures (see Tests), with provenance in `fixtures/README.md`
- [ ] `fuzz/fuzz_targets/extract.rs` with seeds from `fixtures/corpus` and the new fixtures; run 10 min locally
- [ ] Crate README: what the rules do and do not handle, with the limits stated plainly

## Acceptance criteria

- [ ] ADR 0030 is merged with status Accepted and defines every top-level key of `lamina.extract/1`, the versioning rule and the model licence rule
- [ ] Every block on every fixture has a `bbox` in top-left points within the page's CropBox and a `bboxNorm` in [0, 1]; rotated (`/Rotate 90`) and `UserUnit` fixtures produce the hand-computed boxes
- [ ] Two-column untagged fixture: blocks come out column by column (expected order in the golden), not line across both columns
- [ ] Tagged fixture whose content order differs from its structure order: output follows the structure tree and reports `structSource: "tagged"`; the same file with a deliberately broken tree falls back to XY-Cut++ with `extract-struct-tree-ignored`
- [ ] Headings, list items (with depth) and paragraphs match the golden on the heading and list fixtures
- [ ] Running headers, footers and page numbers on a 5-page fixture are absent from `blocks` and listed in `dropped` with their reasons; artifact-marked headers are dropped as `artifact`
- [ ] Hidden-text fixture (`Tr 3`, white on white, off-page, clipped, 1 pt text): none of the hidden strings appear in any `Block::text`; each appears in `dropped` with the right reason
- [ ] De-hyphenation joins `extrac-` / `tion` and keeps `well-` / `Known`
- [ ] Extracting the same file twice gives identical output; native and `wasm32` builds produce the same output on the fixtures (via the portability check plus one wasm test run, or a recorded comparison if wasm tests are not yet wired)
- [ ] Cancelling an extraction job between steps returns an error with code `cancelled` (ADR 0026/0028) and no partial output
- [ ] `extract` fuzz target runs 10 min without panic or hang; summary in the PR
- [ ] `cargo xtask portability` passes for `lamina-extract`

## Post-merge checks

- [ ] The scheduled fuzz workflow picks up the `extract` target and runs clean

## Tests

- Unit tests per module: line grouping, block grouping, XY-Cut++ splits on synthetic boxes, heading clustering, list patterns, header/footer matching, de-hyphenation, visibility rules, coordinate transforms.
- Integration tests `crates/lamina-extract/tests/{order,headings_lists,furniture,hidden_text,coords,tagged}.rs` against goldens in `fixtures/extract/` (expected JSON reviewed by hand; tests never rewrite goldens).
- Fixtures generated by `lamina-build` (B5) and B8a's tagged builder: `two-column.pdf`, `headings-lists.pdf`, `running-headers.pdf` (5 pages), `hidden-text.pdf`, `hyphenation.pdf`, `tagged-reordered.pdf`, `tagged-broken.pdf`, `rotated-userunit.pdf`; plus a few permissive third-party files where licences allow (provenance recorded).
- Fuzz: `extract`.

## Docs to update

- [ ] [shareable-libraries §9 L3](../../plans/shareable-libraries.md#9-to-dos): tick "Write ADR 0030; create `lamina-extract` …"
- [ ] `docs/decisions/README.md` (ADR 0030), [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs) table row
- [ ] `fixtures/README.md` (extraction fixtures and goldens), `CLAUDE.md` layout tree (new crate)
- [ ] Crate README

## Risks and open questions

- **Plausibility of structure trees.** Many tagged files are tagged badly. The thresholds (share of characters covered, order jumps) are guesses; tune them on the corpus and record the values in ADR 0030.
- **XY-Cut++ on complex layouts.** Rules fail on magazines, sidebars and pages with floating figures; report confidence and say plainly in the README what rules cannot do (L3c adds models).
- **`lowContrast` and `clipped` are approximations** without rendering. Prefer false negatives over dropping real text; document the heuristic. A render-based check could come later through C2b.
- **B9b fields.** If B9b's runs miss colour or clip data, this PR changes `lamina-text`; coordinate with whoever owns B9b's follow-ups.
- **Size.** If the PR passes ~1,400 lines, split: L3a (ADR, model, lines/blocks, XY-Cut++, visibility) and L3a2 (structure-tree order, headings, lists, furniture), chained with Depends on.
- **Schema location.** JSON Schema files and the `https://lamina.dev/schema/…` URL depend on the domain decision (web plan open questions); ADR 0030 may use a placeholder.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
