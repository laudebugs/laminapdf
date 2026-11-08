# T2a — Extraction flags text hidden under later shapes and images (`covered`)

| Field | Value |
| --- | --- |
| ID | `T2a` |
| Status | `done` |
| Plan | [trust and proof §T2](../../plans/trust-and-proof.md#t2-find-what-files-hide) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [L3g](./L3g-extract-structure-tree.md) |
| ADRs | Amends [0030](../../decisions/0030-extraction-output-and-model-plugins.md#4-hidden-text-is-never-in-text) §4 (a sixth flag, `covered`), §7 (new reasons within `/1`) and §9. Adds an implementation note to [0023](../../decisions/0023-content-stream-reading.md#2-api-a-tokenizer-and-a-visitor) §2 (`BM` and `SMask` join the tracked `ExtGState` subset). Follows [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (boxes in page space) and [0003](../../decisions/0003-reader-tolerance-and-limits.md) (a malformed appearance is skipped, never fatal) |
| Branch | `feat/t2a-covered-text-flag` |
| PR title | `feat(extract): flag text painted over by later shapes, images and annotations as covered` |
| Size | L (≈ 450–600 lines of code with unit tests; fixtures and the regenerated schema don't count). If it passes 800, move the annotation appearances (Tasks, step 6) into a second PR |
| PR | [#307](https://github.com/laudebugs/laminapdf/pull/307) |

## Goal

A black box drawn over live text hides it from a reader and from nobody else. Copying, search, screen readers and every extractor still get the text. Lamina PDF's extraction does the same today: ADR 0030 §4 drops invisible, off-page, tiny, clipped and low-contrast text, but text painted over by a later shape or image stays in the blocks. This todo adds a sixth flag, `covered`, for text with an opaque fill, an opaque image or an annotation's appearance painted over it. Covered runs leave the blocks and go to `dropped`, like the other flags. That closes a prompt-injection channel, since an LLM reading the extraction no longer sees text the reader can't. It also gives [T2b](./T2b-check-failed-redactions.md) its fake-redaction check.

Fake redaction is one of the trust failures Hacker News users return to after every leak. About 150 comments discuss redactions that leave the text in place, and the stricter count (black box, copy-paste, underlying text) found 54 of its 75 hits in 2023–26. "PDF redaction fails are everywhere and it's usually because people don't understand that covering text with a black box doesn't actually remove the underlying data" ([46370964](https://news.ycombinator.com/item?id=46370964)). X-ray, a library that finds such boxes, drew 709 points in December 2025 ([46369923](https://news.ycombinator.com/item?id=46369923)) ([report R2](../../research/hacker-news/pdf-users-on-hacker-news.md#r2-ship-real-redaction-and-catch-fake-redaction-in-any-file)).

## Context

- [T3a](../done/T3a-text-layer-status.md) also forwards image events through `TextExtractor::page_with` and collects image boxes in `crates/laminapdf-extract/src/marks.rs`. Whichever lands second extends the other's code rather than adding a second visitor; T3a names the pages whose text lies under an image covering at least 90% of the page, which this todo keeps in the text as an OCR layer.
- [Report R2](../../research/hacker-news/pdf-users-on-hacker-news.md#r2-ship-real-redaction-and-catch-fake-redaction-in-any-file), the [format and security note](../../research/hacker-news/notes/format-and-security.md) (complaint 6, redaction that leaves the text in place; the X-ray thread) and the [editors note](../../research/hacker-news/notes/editors.md) (complaint 7). The [extraction note](../../research/hacker-news/notes/extraction.md) counts prompt injection through extracted text.
- [ADR 0030 §4](../../decisions/0030-extraction-output-and-model-plugins.md#4-hidden-text-is-never-in-text): the five flags, `dropped` with counts and never text, `DropOptions::keep_spans` for auditors, and the rule that heuristics lean towards keeping text. §7 says what may change within `laminapdf.extract/1`.
- [L3g](../done/L3g-extract-structure-tree.md), which this builds on: it makes `laminapdf-content` track the clip's bounding box and gives `laminapdf-text`'s runs the clip box and fill colour, and its `low-contrast` test decides when a colour is near white. T2a reuses that colour test for near black and intersects every covering shape with the clip box. L3g may already have moved the drawing collector out of `tables/`; if so, extend it where it is.
- Research: [04 §10.1](../../research/pdf/04-interactive-security.md#101-drawing-a-black-box-is-not-redaction) ("a rectangle drawn over text, whether as an annotation or a `re f` in the content stream, leaves every glyph"), [§10.2](../../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do) (its hidden-content row lists "content under images") and [§2.4](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter); [02 §7.2](../../research/pdf/02-imaging-model.md#72-masking), [§10.2](../../research/pdf/02-imaging-model.md#102-blend-modes), [§10.3](../../research/pdf/02-imaging-model.md#103-constant-alpha-and-soft-masks), [§10.4](../../research/pdf/02-imaging-model.md#104-transparency-groups) and [§11](../../research/pdf/02-imaging-model.md#11-optional-content-layers). ISO 32000-2 §8.9.6 (masked images), §11.3.5 (blend modes; `Compatible` is `Normal`), §11.6 (constant alpha, soft masks, transparency groups), §8.11 (optional content), §12.5.3 (annotation flags) and §12.5.5 (appearance streams, the `/BBox`–`/Matrix`–`/Rect` mapping).
- The code this changes:
  - `crates/laminapdf-extract/src/page.rs`: `runs()` flags each run as a whole (`visibility::flags`), drops it with its first reason, and otherwise splits it at column gutters with `segments()`. `Flow`, `text_block` and the rest work on the pieces it keeps.
  - `crates/laminapdf-extract/src/model.rs`: `Flags` (five `bool`s under an `#[expect(clippy::struct_excessive_bools)]`), `Flags::reasons()`, `DropReason` (kebab-case in JSON), `Dropped` (`reason`, `page`, `bbox`, `chars`, `spans`).
  - `crates/laminapdf-extract/src/tables/edges.rs`: `Collector`, a `Visitor` that keeps the current path's straight segments and rectangles in page space and, on `paint_path`, records rules and wide fills (`Edges::fills`, used for shaded header rows). It sees no images and no paint order.
  - `crates/laminapdf-extract/src/job.rs`: the collector runs through `TextExtractor::page_with` only when `options.tables` isn't `TableMode::Off`.
  - `crates/laminapdf-text/src/extract/mod.rs`: `TextExtractor::page_with` forwards only `path_segment` and `paint_path` to the caller's visitor. `Glyph` (non-exhaustive) has the quad in page space but nothing that orders it against drawing.
  - `crates/laminapdf-content/src/visitor.rs`: `Visitor::image(&ImageDraw)` (`ImageSource::XObject { stream, .. }` or `Inline`; `state.ctm` maps the unit square to the page), `paint_shading`, `enter_xobject`/`leave_xobject` (`FormXObject { dict, state, .. }`, so `/Group` and `/OC` are readable), `begin_marked` (`MarkedContent { tag, properties_name, .. }`). `src/state.rs`: `GraphicsState` tracks `fill_alpha` but no blend mode and no soft mask; ADR 0023 §2 lists the tracked `ExtGState` keys (`LW LC LJ ML D RI FL CA ca Font`).
  - `laminapdf-build`'s `BlendMode` (`src/surface/style.rs`) is the workspace's only blend-mode type; `laminapdf-model` holds the graphics values every crate shares (`Matrix`, `FillRule`, `Dash` in `src/graphics.rs`).
  - `laminapdf-model`'s `Page::annots()` gives the annotation references; `laminapdf_content::interpret_content(doc, bytes, resources, visitor)` runs content that isn't a page's, such as an appearance stream.
- Earlier todos: [L3a](../done/L3a-lamina-extract-reading-order.md) (the first three flags), [L3b](../done/L3b-extract-tables-and-serialisers.md) (the collector, the JSON Schema), [L3d](../done/L3d-extract-structure-and-furniture.md) (furniture drops), [L3e](../done/L3e-extract-bindings-and-benchmark.md) (the TypeScript types generated from the schema), [L3f](../done/L3f-extraction-benchmark.md) (`cargo xtask extract-bench`).

## Scope

**In scope**

- **What covers text.** A glyph is covered when one opaque area painted after it holds its box's centre and at least half its area. The box is the glyph's quad bounding box; both thresholds are starting values, tuned on the fixtures and recorded in ADR 0030. Opaque areas are:
  - **Filled paths** (`f`, `F`, `f*`, `B`, `B*`, `b`, `b*`) made of axis-aligned rectangles, or of subpaths whose sides are all horizontal or vertical (the polygons `edges.rs` already boxes), cut to the clip box.
  - **Images** (`Do` of an image XObject, or an inline image) whose unit square maps to an axis-aligned rectangle, cut to the clip box. An image with `/ImageMask true`, `/Mask`, `/SMask` or `/SMaskInData` above 0 never covers. Nor does an image that covers at least 90% of the page: that is a scanned page, and the text under it is its OCR layer (FineReader's "text under the page image"). [T3a](../done/T3a-text-layer-status.md) reports such pages.
  - **Annotation appearances.** Viewers draw annotations over the page (§12.5.5), so the opaque areas of an annotation's normal appearance cover every glyph under them. Annotations flagged Hidden or NoView, popups, annotations with `/CA` below 1 and annotations without an appearance stream are left out.
- **What opaque means.** The paint uses constant alpha 1 (`ca`), no soft mask, and the `Normal` or `Compatible` blend mode. `Multiply` or `Darken` with a fill within 0.1 of black also covers, since it turns whatever is under it black: this is the dark highlighter drawn over dark text. A fill in a `Pattern` colour space never covers. Nothing inside optional content covers (a `BDC /OC` sequence, or a form or image with `/OC`), whatever the layer's state. Nothing covers inside a form with a transparency `/Group` drawn while the outer state wasn't opaque.
- **Paint order.** `laminapdf-text` numbers the drawing operations it sees (`paint_path`, `image`, `paint_shading`) and gives each glyph the count before it (`Glyph::paints_before`, new). The extract collector numbers the same calls in the same way, so a covering area applies only to glyphs drawn before it. Annotations come after every operation on the page.
- **Runs split at the cover.** A run keeps the pieces `segments()` cuts at gutters today. Inside each piece, each stretch of covered glyphs becomes a `dropped` entry with reason `covered`, its own box and its character count, and the visible glyphs on either side join with one space. A run whose text isn't its glyphs in drawing order (`ActualText`, right to left, vertical) is covered as a whole when more than half its glyphs are, and kept whole otherwise. The other flags win: an invisible run that is also covered is dropped as `invisible`.
- **A cover never splits a line.** `lines.rs` takes a gap wider than a font size between two runs on one baseline for a column gutter. When covered text on that baseline fills the gap (the covered words were their own run), the runs still join one line, so `Pay [box] now.` reads `Pay now.` and not two columns.
- **The model and output.** `Flags::covered`, `DropReason::Covered` (`"covered"` in JSON), the schema regenerated, the TypeScript types regenerated. The flag has no switch, as the other five have none; `keep_spans` shows the covered text to an auditor.
- **The collector runs on every page**, tables or not, in the same walk of the content as the text.
- **Small changes in lower crates**, each in its own commit: `BlendMode` moves to `laminapdf-model`; `GraphicsState` tracks the blend mode and whether a soft mask is set; `page_with` forwards more events and `Glyph` gets `paints_before`.

**Out of scope** (and which todo covers it)

- `laminapdf check` warnings about covered text, unapplied redactions and earlier revisions: [T2b](./T2b-check-failed-redactions.md). Counts of hidden text in a disclosure report: [T2c](../ready/T2c-reveal-command.md).
- Removing covered text from a file: [E12c](../ready/E12c-apply-content-removal.md) (content removal) and [E12d](../ready/E12d-scrub-and-rewrite.md).
- Strokes (a thick marker line over text), shadings (`sh`), curved or rotated shapes and rotated images. Each is left out to keep text, as ADR 0030 §4 asks; add them when a fixture or a corpus file shows the need.
- Text inside layers that are off: no ADR 0030 flag exists for it yet. [T2c](../ready/T2c-reveal-command.md) lists those layers.
- A rendered check (render the page, compare with the glyph boxes): ADR 0030 §4 leaves it to a later change through the render backend.

## Tasks

- [x] Claim via the work PR; a fresh worktree off `main`; load `/rust-best-practices`, and `/writing` for the ADR text and the READMEs.
- [x] **`BlendMode` in `laminapdf-model`.** Move the enum from `crates/laminapdf-build/src/surface/style.rs` to `crates/laminapdf-model/src/graphics.rs`, beside `FillRule`, and add `BlendMode::from_pdf_name(&[u8]) -> Option<Self>` (`Compatible` reads as `Normal`). `laminapdf-build` re-exports it as it does `FillRule`, so `laminapdf::BlendMode` and `laminapdf-wasm`'s `build_dto.rs` keep their paths; `cargo xtask check maintainability` stays clean with no new entry in `maintainability-duplicates.toml`.
- [x] **`laminapdf-content`.** Add `GraphicsState::blend_mode: BlendMode` and `GraphicsState::soft_mask: bool`, set by `gs` from `/BM` (a name, or an array whose first known name wins; an unknown one is `Normal`, §11.3.5) and `/SMask` (anything but `/None` sets it). `q` and `Q` save and restore them with the rest of the state. Unit tests in `src/state.rs` and the interpreter's tests. Add ADR 0023's implementation note (below).
- [x] **`laminapdf-text`.** In `TextExtractor::page_with`, also forward `image`, `paint_shading`, `begin_marked`, `end_marked`, `enter_xobject` and `leave_xobject`, after the extractor's own handling. Add `Glyph::paints_before: u32`: the number of `paint_path`, `image` and `paint_shading` calls the interpreter made before the glyph, counted whether or not a visitor is passed, so `page` and `page_with` return equal `PageText`s. Update both methods' docs and the module docs.
- [x] **The drawing collector** (`crates/laminapdf-extract`). Move `tables::edges::Collector` to `src/paint.rs` (unless L3g already did), returning `Painted { edges: Edges, covers: Vec<Cover> }`, where `Cover { rect: RectPt, after: u32 }` is an opaque area and the number of paints before it. Count `paint_path`, `image` and `paint_shading` exactly as `laminapdf-text` does. Record a cover for each filled rectangle and axis-aligned polygon whose paint is opaque, and for each opaque image, each cut to L3g's clip box. Track optional content with a depth counter (`begin_marked` with tag `OC`; forms and images with `/OC`) and transparency groups with a stack pushed in `enter_xobject`. `job.rs` always collects; `TableMode::Off` just ignores the edges.
- [x] **Annotation appearances** (`src/paint/annots.rs` or a submodule of your choice). For each `/Annots` entry that isn't a popup, isn't flagged Hidden (bit 2) or NoView (bit 6) and has `/CA` 1 or none: take `/AP /N`, a stream or the sub-dictionary entry `/AS` names. Map its `/BBox` through `/Matrix` onto `/Rect` (§12.5.5), run it with `laminapdf_content::interpret_content` through a collector whose points pass through that matrix, and keep its covers with `after: u32::MAX`. A malformed annotation or appearance is skipped (`absent_if_malformed`), with the interpreter's usual warnings.
- [x] **The covered test and the split** (`src/page.rs`, with the test itself in `src/visibility.rs`). For each glyph of a run, look for a cover with `after >= glyph.paints_before` that holds the box's centre and half its area. Index the covers (sort by `x0` and sweep, or a coarse grid) so a page with 10,000 glyphs and 10,000 fills doesn't test every pair. Split runs as Scope says: cut pieces with `segments()` first, then drop the covered stretches inside each piece. Pass the covered boxes to `lines()` (`src/lines.rs`), which skips its gutter split where one fills the gap between two runs.
- [x] **The model.** `Flags::covered` (update `NONE`, `any()`, `reasons()`: `covered` comes last, after `low_contrast`, and the `#[expect]` reason says six flags), `DropReason::Covered`, docs on both. Regenerate `crates/laminapdf-extract/schemas/extract.schema.json` (`UPDATE_GOLDEN=1 cargo test -p laminapdf-extract --test output`), then `packages/sdk/src/generated/extract.ts` (`cargo xtask gen boundary`).
- [x] **ADR 0030.** Add the flag to §4 and an implementation note under §9, worded as follows (tune the numbers first):
  > - `covered`: painted over later on the same page. A glyph is covered when one opaque area drawn after it holds its box's centre and at least half its area. Opaque areas are filled rectangles and polygons with horizontal and vertical sides, images without a mask or soft mask, and the opaque parts of an annotation's normal appearance, which viewers draw over the page. Opaque means constant alpha 1, no soft mask and the `Normal` blend mode, or `Multiply` or `Darken` with a fill near black; nothing inside optional content or a translucent transparency group covers. An image over at least 90% of the page is a scan, and the text under it is its OCR layer, so it covers nothing. A run is split where the cover starts and ends, so the visible words of a line stay in the text.

  In §7, add that new `reason` values and flag names may appear within `/1`, and that a consumer treats a value it doesn't know as hidden text. In the §9 note, record the thresholds and that strokes, shadings and rotated shapes don't cover yet.
- [x] **ADR 0023 note.** Under §2: `GraphicsState` also tracks `BM` (as `laminapdf_model::BlendMode`) and whether `SMask` is set (T2a, for the `covered` flag).
- [x] **Fixtures and tests** (see Tests). Then run `cargo xtask extract-bench` by hand and record the scores in the PR beside the README's table.
- [x] **Docs.** `crates/laminapdf-extract/README.md` ("How it reads a page", step 1, and "What it can't do yet"); the hidden-text bullet of `apps/docs/src/content/docs/recipes/extract-markdown.mdx` (plain words, no flag names: "text painted over by a later shape, image or comment"); CLAUDE.md's extraction paragraph (`Flags`: … `covered`).

## Acceptance criteria

- [x] A built page drawing "Pay John Smith now." and then a black rectangle over "John Smith" extracts as one block, `Pay now.`, with one `dropped` entry: reason `covered`, the rectangle's page, a box around the two words and `chars` 9. With `DropOptions::keep_spans`, that entry's `spans` hold `John Smith`. The same holds when "John Smith" is drawn in a font of its own, as a separate run between the other two.
- [x] These cover, each on its own built page: a white rectangle; an opaque JPEG (`fixtures/images/photo-rgb.jpg`) over one word; a near-black fill in `Multiply` over black text; a filled `Square` annotation whose appearance is opaque. Text drawn twice, with the box over the first copy only, comes out once.
- [x] These don't cover, each on its own page: the same rectangle at `ca` 0.5; a yellow `Multiply` highlight; a shaded table row drawn before its text; the next row's shading overlapping the descenders of the row above; a box inside `BDC /OC`; an RGBA image (with its `/SMask`) over text; a full-page JPEG over text; a box inside a form with a transparency `/Group` drawn at `ca` 0.5; annotations flagged Hidden or with `/CA` 0.5.
- [x] No file of `fixtures/corpus/`, `fixtures/golden/` (the examples included) or `fixtures/reading-order/` gives a `covered` entry, unless the test names it with the reason; `tests/reading_order.rs` and `tests/output.rs` pass unchanged, with `fixtures/extract/json/` and `fixtures/extract/markdown/` byte for byte, and `crates/laminapdf/tests/reading_order.rs` keeps its scores.
- [x] `extract.schema.json` lists `covered` in `DropReason`, `packages/sdk/src/generated/extract.ts` has it, and `cargo xtask gen boundary --check` passes.
- [x] `TextExtractor::page` and `page_with` return equal `PageText`s on every corpus file (a test in `crates/laminapdf-text/tests/`), and `Glyph::paints_before` counts as documented.
- [x] `GraphicsState::blend_mode` and `soft_mask` follow `gs`, `q` and `Q` (unit tests); `fixtures/corpus/pdfjs-operators.json`'s comparison in `crates/laminapdf-content/tests/corpus.rs` passes unchanged.
- [x] ADR 0030 §4, §7 and §9 and ADR 0023 §2 carry the text above, with the tuned thresholds.
- [x] `cargo xtask size` stays within `packages/wasm/size-budgets.json`; the `extract` fuzz target runs (`cargo xtask fuzz extract --time 60`) without a crash.
- [x] The PR records `cargo xtask extract-bench`'s scores; none drops by more than 0.5 points against `crates/laminapdf-extract/README.md`'s table.

## As built

- **Where the code went.** The collector moved to `src/paint/mod.rs` (`Collector`, `Cover`), with `src/paint/annots.rs` for the appearances; `Edges` stays in `tables/edges.rs`. The covered test (`Covers`, a 64 pt grid index) and the run split live in a new `src/segments.rs`, with `segments()` and `quad_box()` moved there from `page.rs`, which was 28 lines under its module budget; `visibility.rs` keeps the other flags.
- **`Paint::draws`.** `paint_path` also fires for `n` and for a path left unpainted, so `laminapdf-content` gained `Paint::draws()`, and both `laminapdf-text` and the collector count only the paths it says draw.
- **Polygons.** A subpath covers only when it fills exactly its box (every point a corner of it), so an L, a U or a frame drawn as one subpath doesn't. A box holding another box of the same path covers nothing: a frame drawn as two rectangles with `f*` would otherwise hide the text inside it. Pattern fills never cover.
- **Review round 1** added: covers cut to the page, the 90% scan rule over every image's area, annotations with `/OC` or a `/BM` other than `Normal` skipped, and an annotation that can't be read (a limit included) skipped through `laminapdf_cos::tolerant`.
- **One space.** When the covered words were their own run, the runs either side kept their own spaces (`Pay  now.`); `lines.rs`'s `line()` joins across a covered gap with exactly one space. The structure tree's lines (`tagged.rs`) don't get the covered boxes yet, so on a tagged page a covered gap between two runs may still split a line.
- **The plan's checkbox** covers T2a–T2c together, so it stays open until T2c.
- **Thresholds.** Centre and half the box, as proposed: nothing in the fixtures comes near them.

## Post-merge checks

None.

## Tests

- **Unit:** `BlendMode::from_pdf_name` (names, arrays, `Compatible`); `GraphicsState` across `gs`, `q`, `Q`; `paints_before` on content with fills between `Tj`s; the collector's covers for each opaque rule and each refusal (alpha, soft mask, blend, pattern, optional content, groups, masked images, page-size images, rotation); the covered test's centre-and-half rule at its edges; the annotation matrix (`/BBox`, `/Matrix`, `/Rect`, a rotated `/Matrix`).
- **Integration:** a new `crates/laminapdf-extract/tests/covered.rs` with the acceptance criteria's pages. Build them with `laminapdf-build` where it can draw them (`Surface::fill_color`, `opacity`, `blend_mode`, `path(…).fill(FillRule::NonZero)`, `image` with `Image::jpeg` or `Image::rgba`). Write the rest as raw objects with `laminapdf_test_fixtures::{classic, stream_body}`, as `tests/extract.rs`'s `page_with_content` does: optional content, the transparency group, the annotations. The corpus gate lives in the same file.
- **Existing:** `tests/extract.rs` (hidden text), `tests/tables.rs` (the collector's move), `tests/reading_order.rs`, `tests/output.rs`, `tests/regressions.rs` (the fuzz entry over the corpus); `crates/laminapdf/tests/extract.rs` and `reading_order.rs`; `@laminapdf/sdk`'s extract tests over the regenerated types.
- **Fuzz:** the `extract` target covers the new paths, since its seeds are every fixture; add one file with an annotation appearance under `fuzz/seeds/extract/` (and a row to `fuzz/README.md`'s seeds column).

## Docs to update

- [x] ADR 0030 §4, §7, §9 and ADR 0023 §2 (Tasks)
- [x] `crates/laminapdf-extract/README.md`; `apps/docs/src/content/docs/recipes/extract-markdown.mdx`; CLAUDE.md (the extraction paragraph and the content API's tracked state)
- [x] [Trust and proof §T2](../../plans/trust-and-proof.md#t2-find-what-files-hide): links point here; its checkbox covers T2a–T2c, so it stays open

## Risks and open questions

- **Scans with text under the image.** The 90% rule keeps the OCR text of scanned books in the output. It also misses a page-size "withheld" image drawn over a page's real text, which then stays visible to an LLM. Recommendation: keep the rule, which leans towards keeping text as ADR 0030 asks, and let [T3a](../done/T3a-text-layer-status.md)'s text-layer status name pages whose text lies under a scan. The owner may prefer a separate flag; decide in the PR.
- **Glyph boxes are taller than the ink.** Quads run from the font's descent to its ascent, so a box drawn tight around the letters may hold less of the box than the ink. The centre-and-half rule allows for that (a box from the baseline to the cap height holds about 60% of a Helvetica glyph's box); tune it on the fixtures before changing it.
- **Missed covers.** Strokes, shadings, curved and rotated shapes, and covers in optional content don't count, so some fake redactions stay in the text. Each miss leaves noise in the output, while a false cover would lose real text.
- **Whole runs.** Right-to-left, vertical and `ActualText` runs are judged as a whole, so a name hidden inside an Arabic sentence stays in the text. Splitting them needs visual-to-logical mapping per glyph; revisit with a fixture.
- **Cost.** Every page now runs the drawing collector, and every page with annotations interprets their appearances. Measure on the reading-order corpus and the benchmark subsets, and say in the PR what it cost; there are no wall-clock assertions (ADR 0018).
- **Viewer settings.** Covers from annotations assume the screen view with annotations shown, as every viewer starts. Printing can differ (the Print flag); extraction follows the screen.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
