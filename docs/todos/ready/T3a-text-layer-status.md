# T3a — A per-page text-layer status that says when a page needs OCR

| Field | Value |
| --- | --- |
| ID | `T3a` |
| Status | `ready` |
| Plan | [trust and proof §T3](../../plans/trust-and-proof.md#t3-extraction-users-can-check) |
| Round / milestone | Round T (trust and proof) |
| Depends on | None |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md) §1, [§4](../../decisions/0030-extraction-output-and-model-plugins.md#4-hidden-text-is-never-in-text) and [§7](../../decisions/0030-extraction-output-and-model-plugins.md#7-versioning-and-determinism) (a new key within `/1`; records an implementation note), [0026](../../decisions/0026-error-and-diagnostic-contract.md) (the new warning), [0023](../../decisions/0023-content-stream-reading.md) (image events), [0003](../../decisions/0003-reader-tolerance-and-limits.md) (warnings, strict mode) |
| Branch | `feat/t3a-text-layer-status` |
| PR title | `feat(extract): a per-page text-layer status that says which pages need OCR` |
| Size | L (≈ 500–700 lines: the measures and rules, the warning, tests on built pages, the bench counts; the schema, goldens and generated files don't count) |
| PR | — |

## Goal

Tell a pipeline, page by page, whether the extracted text is the page's text, so it can send only the pages that need it to OCR or a vision model. Scanned pages come up in 617 extraction comments, users praise pipelines that send only the hard pages to a model ([41206121](https://news.ycombinator.com/item?id=41206121), [49290831](https://news.ycombinator.com/item?id=49290831)), and one user puts text that is really images or paths "closer to the 40% case than the 1% case" ([44784425](https://news.ycombinator.com/item?id=44784425)). Today a scanned page gives an empty Markdown page and no warning: on olmOCR-bench, 89 of 685 pages gave no text, and only the scores showed it (`crates/laminapdf-extract/README.md`).

## Context

- [Report R4: make extraction checkable](../../research/hacker-news/pdf-users-on-hacker-news.md#r4-make-extraction-checkable) and the [extraction note](../../research/hacker-news/notes/extraction.md): want 3 (scanned pages), the hybrid pipelines in §3, and §6 priority 2 ("Tell users which pages need OCR or a VLM"). A user with "thousands" of legacy PDFs says "Some have Adobe's OCR embedded, but most have none at all" ([43975217](https://news.ycombinator.com/item?id=43975217)): a status has to tell those two apart.
- [ADR 0030](../../decisions/0030-extraction-output-and-model-plugins.md): §1 lists a page's keys; §4 keeps invisible text (render mode 3, which OCR layers use) out of every block, so an OCR'd scan extracts as nothing today; §7 lets new keys appear within `/1`, so this is an additive change and needs no `/2` and no amendment, only an implementation note like L3b's and L3d's.
- The code:
  - `crates/laminapdf-extract/src/page.rs`: `runs()` flags each run with `visibility::flags()`; `extract_page()` builds the `ExtractedPage`.
  - `crates/laminapdf-extract/src/job.rs`: `ExtractJob::run` calls `TextExtractor::page` when tables are off and `page_with` (with `tables::Collector`) otherwise.
  - `crates/laminapdf-text/src/extract/mod.rs`: `TextExtractor::page_with` forwards only path events (`path_segment`, `paint_path`) to its visitor. `Glyph::mapped` and `PageText::unmapped` count glyphs no rule maps to Unicode (Type 3 and other fonts without ToUnicode, usable names or a `cmap`).
  - `crates/laminapdf-content/src/visitor.rs`: `Visitor::image` reports every image XObject and inline image, stencil masks included, with `ImageDraw::state.ctm` mapping the unit square to user space, inside Form XObjects too.
- [L3f](../done/L3f-extraction-benchmark.md) and `cargo xtask extract-bench`: the three scored olmOCR-bench subsets hold the scanned pages that gave no text, so they check the status on real files.
- [L3c](./L3c-model-plugins.md) plans to OCR "pages without a usable text layer"; this todo defines that test, so L3c can pick pages by the status. [E13c](./E13c-ocr-text-layer.md) writes invisible OCR layers; pages it makes should read as `ocr-layer`.
- [T2a](./T2a-covered-text-flag.md) needs the page's image boxes too (text covered by later images). Put the image collection in a module both can use (`marks.rs` below).

## Scope

**In scope**

- **A `textLayer` object on every page** (`ExtractedPage::text_layer: TextLayer`): the status and the measures it was decided from, so a pipeline can apply its own cut-offs to the JSON:
  - `chars`: visible characters (not white space, as `text_fix::visible_chars` counts them) in runs not flagged invisible, off-page or tiny. Artifact-marked runs count, since they are drawn.
  - `invisibleChars`: characters in runs drawn in render mode 3 or 7.
  - `unmappedShare`: the share of the visible runs' glyphs whose `Glyph::mapped` is false; 0 with no visible glyphs.
  - `images`: how many images the page draws, in its forms too.
  - `imageShare`: the share of the page that image boxes cover, measured on a 32 × 32 grid of the page (a cell counts when its centre lies in an image's box, clipped to the page). The grid makes the value deterministic and counts overlapping images once.
  - `paths`: how many paths the page paints (`Paint::Stroke`, `Fill` and `FillStroke`).
  - Shares are rounded to 1/1000, as ADR 0030 §7 rounds lengths.
- **`TextLayerStatus`**, the first rule that matches, with the default cut-offs:

  | `status` | When | What a pipeline does with the page |
  | --- | --- | --- |
  | `empty` | no text, visible or invisible; no image; fewer than 16 painted paths | nothing; it is blank |
  | `ocr-layer` | images cover at least 50% of the page and its invisible characters outnumber its visible ones | it is an image with an OCR layer, which extraction leaves out (ADR 0030 §4): trust the layer (`DropOptions::keep_spans` returns it in `dropped`), or OCR it again |
  | `needs-ocr` | no visible text; or images cover at least 50% and the page has at most 200 visible characters (a stamp or page number on a scan); or at least 50% of the visible glyphs are unmapped | OCR or a vision model |
  | `mixed` | visible text, but at least 10% of its glyphs are unmapped, or images cover at least 25% of the page | the text is usable; parts of the page may be pixels or glyphs without Unicode |
  | `native` | anything else | use the text |

  Paths-only text (outlined glyphs) reads as `needs-ocr`, since it has no visible text and paints paths. Type 3 text reads by its glyphs: a Type 3 font whose glyphs map to Unicode is text like any other, and one whose glyphs don't is counted in `unmappedShare`.
- **`TextLayerThresholds`** (new, Rust only): `image_page` (0.5), `image_mixed` (0.25), `few_chars` (200), `unmapped_needs_ocr` (0.5), `unmapped_mixed` (0.1) and `drawn_paths` (16), with a `Default`. `ExtractOptions` gains `text_layer: TextLayerThresholds` and `with_text_layer()`. The bindings and the CLI use the defaults in this todo.
- **A warning, `extract-needs-ocr`** (kind `unsupported`), once for each page whose status is `needs-ocr` or `ocr-layer`, with the data field `status` and the page in its location. Without it, a Markdown or text user gets an empty page and no sign of why. `mixed`, `native` and `empty` pages raise nothing.
- **Image events in `laminapdf-text`**: `TextExtractor::page_with` forwards `Visitor::image` as it forwards path events, and extraction always reads a page with `page_with`, so the status costs no second pass over the content.
- **The surfaces:** the façade re-exports the new types (`laminapdf::extract`); the committed schema, the JSON goldens and the generated TypeScript types gain `textLayer`; `@laminapdf/sdk` exports `TextLayer` and `TextLayerStatus`; `laminapdf extract --format json|md|chunks` carries it through `laminapdf.extract/1`, and the warning reaches standard error and exit code 3 (ADR 0031's warnings-only code).
- **The bench:** `extract-bench` records each page's status, and its report counts them.

**Out of scope** (and which todo covers it)

- Running OCR: [L3c](./L3c-model-plugins.md) (`OcrEngine`) and [E13d](./E13d-local-ocr.md) (local engines); writing an OCR layer into a PDF: [E13c](./E13c-ocr-text-layer.md).
- Putting an OCR layer's text in the blocks of an `ocr-layer` page: that changes ADR 0030 §4, so it waits for the owner's decision (Risks).
- Text whose glyphs map to the wrong characters, text that differs from what the page shows, and text under later images: a rendered check through the render backend later (ADR 0030 §4, [ADR 0004](../../decisions/0004-render-backend.md)), and [T2a](./T2a-covered-text-flag.md) for covered text.
- The clip: image boxes are taken whole until [L3g](./L3g-extract-structure-tree.md) tracks the clip's box.
- `laminapdf extract`'s plain-text format (`laminapdf.text/1`) and the `laminapdf_extract` MCP tool, which answers with that format only: neither runs the extraction rules. Giving the MCP tool the extracted document is a later CLI and MCP change.
- The cut-offs as `doc.extract()` options in TypeScript (Risks).

## Tasks

- [ ] Claim via the work PR (see Definition of done); a fresh worktree off `main`. Load `/rust-best-practices` and follow [the Rust guidelines](../../guidelines/rust.md); `laminapdf-extract` stays a core crate with `#![forbid(unsafe_code)]`, and no todo ID goes in a code comment (`cargo xtask check maintainability`'s `todo-id` rule).
- [ ] `laminapdf-text`, in its own commit: in `crates/laminapdf-text/src/extract/mod.rs`, the private `Collector`'s `Visitor` impl forwards `image` to `paths`, as `path_segment` and `paint_path` do; `page_with`'s doc names image events. A unit test: a visitor passed to `page_with` sees a built page's image.
- [ ] `crates/laminapdf-extract/src/marks.rs` (new): `Marks`, a `Visitor` that holds an `Option<tables::Collector>` and forwards path events to it, records each image's box in page space (the unit square's four corners through `ImageDraw::state.ctm` and `PageFrame::to_page`, their bounding box clipped to the page) and counts painted paths. `finish()` returns the table rules (`Option<Edges>`) and the image boxes and path count.
- [ ] `crates/laminapdf-extract/src/text_layer.rs` (new): `measure()` (the measures from the page's runs and glyphs, with the same `visibility::flags()` that `page::runs()` uses, before the `max_blocks_per_page` cut) and `classify()` (the table's rules, in order), with unit tests for each rule and each cut-off's edge.
- [ ] `job.rs`: `ExtractJob::run` always calls `page_with` with a `Marks` (holding a `Collector` only when tables are on) and passes the marks to `extract_page`.
- [ ] `page.rs`: `extract_page` sets `info.text_layer` and reports `Warning::ExtractNeedsOcr` for `needs-ocr` and `ocr-layer` pages.
- [ ] `model.rs`: `TextLayer` and `TextLayerStatus` (`#[non_exhaustive]`, serde camel-case keys and kebab-case values, schemars), and `text_layer` as the last field of `ExtractedPage`. `options.rs`: `TextLayerThresholds`, `ExtractOptions::text_layer` and `with_text_layer`. Export them from `lib.rs`, and re-export them from `crates/laminapdf/src/extract.rs`.
- [ ] The warning: an entry in `fixtures/codes.json`'s `warnings` (`extract-needs-ocr`, kind `unsupported`, summary, hint, `since` as the other extract codes have it); `docs/errors/extract-needs-ocr.md` (the data field `status`); `Warning::ExtractNeedsOcr { page: u64, status: &'static str }` in `crates/laminapdf-cos/src/reader/warning.rs`, in `code`, `object`, `kind`, `location` and `key` and in its tests' list of every variant; the `status` data in `crates/laminapdf/src/data.rs` and a sample in `crates/laminapdf/tests/data_fields.rs`; then `cargo xtask gen codes`.
- [ ] Regenerate: `UPDATE_GOLDEN=1 cargo test -p laminapdf-extract --all-features --test output` (the schema; the Markdown goldens don't change), `UPDATE_GOLDEN=1 cargo test -p laminapdf --test extract` (`fixtures/extract/json/`), `cargo xtask gen boundary` (`packages/sdk/src/generated/extract.ts`). Add `TextLayer` and `TextLayerStatus` to the type exports in `packages/sdk/src/extract.ts`.
- [ ] The bench: `crates/laminapdf/examples/differential/extract-bench.rs` writes each file's page statuses into its `facts`, and `render_report` in `tools/xtask/src/extract_bench.rs` adds a line counting pages per status. Run `cargo xtask extract-bench` once and record the counts.
- [ ] Docs (below), and `cargo xtask size` within the module budgets.

## Acceptance criteria

- [ ] On the pages built in `tests/text_layer.rs` (Tests), each status is the one named there, with the measures asserted; `extract-needs-ocr` is raised once for each `needs-ocr` and `ocr-layer` page, with its `status`, and for no other page.
- [ ] Every page of `fixtures/reading-order/` and of `fixtures/golden/text-*.pdf` is `native`, and nothing in those files raises `extract-needs-ocr`.
- [ ] Changing a cut-off through `ExtractOptions::with_text_layer` changes the status it governs (a `mixed` page becomes `native`).
- [ ] `crates/laminapdf-extract/schemas/extract.schema.json`, `fixtures/extract/json/` and `packages/sdk/src/generated/extract.ts` are regenerated; their only change is the new `textLayer` object. Every JSON output still validates against the schema, and the same input gives the same bytes.
- [ ] `laminapdf extract <image-only page> --format md` prints nothing to standard output, names `extract-needs-ocr` on standard error and exits 3 (0 with `--warnings-ok`); `--format json` holds `"status":"needs-ocr"` and validates against the committed schema.
- [ ] `cargo xtask extract-bench` reports pages per status. On the three scored subsets, every page that gave no text, other than the pages whose filters can't be decoded ([#108](https://github.com/laudebugs/laminapdf/issues/108)), is `needs-ocr` or `ocr-layer`; the counts, and the number of pages that passed the benchmark's baseline test while marked `needs-ocr`, are in the crate README.
- [ ] `cargo xtask size` passes without a raised budget.

## Post-merge checks

None.

## Tests

- `crates/laminapdf-extract/tests/text_layer.rs` (new), on pages built with `laminapdf-build` (`Builder`, `Image::rgba`, `surface.image`, `surface.path`, `TextRun::render_mode(TextRenderMode::Invisible)`), saved and reopened as `tests/extract.rs`'s `build()` does:
  - a page of Helvetica text: `native`;
  - one image covering the page: `needs-ocr`;
  - the same image under an invisible line of 300 characters: `ocr-layer`, with no blocks and the line in `dropped` as `invisible`;
  - the image with a visible 70-character stamp across its foot: `needs-ocr`;
  - a paragraph beside an image covering a third of the page: `mixed`;
  - a blank page: `empty`, with no warning;
  - forty filled shapes made of curves, and no text: `needs-ocr`;
  - a 3-page file mixing these, so the warnings carry the right page indices;
  - `fixtures/golden/images.pdf` and `vectors.pdf`, whose statuses the test records with the reason for each.
- Unmapped glyphs need a font without Unicode, which `laminapdf-build` never writes. Build it by hand, as `tests/tables.rs`'s `pages()` does with `laminapdf_test_fixtures::classic`: a Type 3 font whose glyph names aren't in the AGL and that has no ToUnicode. One page drawn only in it is `needs-ocr` (`unmappedShare` 1); a page with one run in five in it is `mixed`.
- Unit tests in `marks.rs` (an image's box under a rotation and a scale, a box clipped to the page, two overlapping images counted once) and `text_layer.rs` (each rule and each cut-off's edge).
- `crates/laminapdf-text`: the forwarding test above.
- `crates/laminapdf-cli/tests/extract.rs`: the CLI criterion above, with the image-only page built as `hidden_text()` builds its page.
- `crates/laminapdf/tests/data_fields.rs`: the warning's `status`.
- The `extract` fuzz target keeps running over the corpus (`tests/regressions.rs`).

## Docs to update

- [ ] ADR 0030: an implementation note (as L3b's and L3d's), recording `textLayer`, its measures, the five statuses with their default cut-offs, the warning, and that §4 still keeps an OCR layer out of the blocks.
- [ ] `crates/laminapdf-extract/README.md`: a section on pages that need OCR, the bench counts, and the new test file in its Tests list.
- [ ] `apps/docs/src/content/docs/recipes/extract-markdown.mdx`: under "What to change", how to find the pages that need OCR in TypeScript (`pages.filter(p => p.textLayer.status === "needs-ocr")`) and Rust, in plain words and without internal IDs ([docs writing](../../guidelines/docs-writing.md)).
- [ ] `crates/laminapdf-cli/README.md`: `extract`'s JSON says which pages need OCR, and the warning sets exit code 3.
- [ ] `CLAUDE.md`'s "Extraction for retrieval" paragraph: `marks.rs`, `text_layer.rs`, `TextLayer`, `extract-needs-ocr`.

## Risks and open questions

- **The cut-offs are starting values.** They come from the built pages and the bench counts and are recorded in ADR 0030's note. A born-digital page drawn over a full-page background image reads as `mixed`, and a cover page with a large photo and a short title reads as `needs-ocr`. Both lean towards sending a page to OCR, which costs a call; the opposite mistake loses a page's text.
- **`imageShare` is an estimate.** It ignores the clip until L3g and ignores transparency, so a large image clipped to a small area, or a faint watermark, counts whole.
- **Warnings on long scans.** A scanned book raises one warning per page and can reach `Limits::max_warnings` (1,000), after which `warnings-truncated` hides later warnings. The status in `pages` stays complete.
- **Strict mode** turns every warning into a failure, so a strict extraction of a scan fails with `strict` instead of returning empty pages. That matches how the other extract warnings behave; the owner may prefer to exempt this one.
- **For the owner: use an existing OCR layer?** On an `ocr-layer` page, extraction drops the layer as invisible text (ADR 0030 §4), so a file made searchable by OCRmyPDF, Acrobat or Lamina PDF's own E13c extracts as nothing. An opt-in that takes the layer's text into the blocks on `ocr-layer` pages only would fix that without opening the prompt-injection channel §4 closes elsewhere. It needs an ADR 0030 amendment and its own todo.
- **For the owner: thresholds in TypeScript?** The JSON carries every measure, so a TypeScript caller can apply other cut-offs today; a `textLayer` option in `doc.extract()` can follow if users ask.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
