# 0030 — Extraction output (`laminapdf.extract/1`) and model plug-ins

- Status: Accepted (2026-11-04, when L3a's PR merges)
- Date: 2026-11-04

## Context

Text extraction ([B9b](../todos/done/B9b-text-extraction-core.md), [B9d](../todos/done/B9d-text-runs-quads-and-page-text.md)) gives runs of glyphs with boxes, in the order the content stream draws them. An LLM or a retrieval pipeline needs more: paragraphs, headings and list items in reading order, each with a box it can cite, and no text a reader can't see. Fewer than 20% of PDFs are tagged, so extraction usually has to infer reading order from the layout. Rules do this well enough to be the default: OpenDataLoader's vendor benchmark puts rule-based reading order at about 0.90 at 15 ms a page ([report: rule-based extraction in core, models as licence-clean plug-ins](../research/libraries/shareable-libraries.md#rule-based-extraction-in-core-models-as-licence-clean-plug-ins)). Layout models do better on magazines and complex tables, but most of them carry weights under licences ADR 0001 rules out: MinerU's custom conditions, Marker and Surya's OpenRAIL-M, PyMuPDF's AGPL ([AI extraction note §B](../research/libraries/notes/llm-api-and-ai-extraction.md#b-ai-consumption-primitives-the-2026-landscape-algorithms-bboxprovenance-schema-tagged-pdf-and-rule-based-vs-model-based)).

Hidden text is a security problem as much as a quality one. A person reading the page never sees text drawn in render mode 3, white on white, clipped away or off the page. An LLM reading the extracted text sees all of it, which makes hidden text a prompt-injection channel. Extraction is where to stop it.

The output is a contract. Pipelines will store it, index it and compare it across versions, so its keys, its coordinates and its determinism have to be fixed before the first serialiser ships ([L3b](../todos/done/L3b-extract-tables-and-serialisers.md)). [ADR 0026](0026-error-and-diagnostic-contract.md) (errors), [ADR 0027](0027-coordinates-units-and-page-indexing.md) (coordinates) and [ADR 0028](0028-facade-rules-jobs-and-cancellation.md) (jobs) already constrain it. This ADR fixes the `laminapdf.extract/1` model, the rules that build it, and the policy that keeps models out of the core.

## Decision

### 1. The `laminapdf.extract/1` document

One extraction is one JSON object with these top-level keys, in this order:

| Key | Holds |
| --- | --- |
| `schema` | `"laminapdf.extract/1"` |
| `generator` | `{ "name": "laminapdf", "version", "mode": "rules", "models": [] }`: `mode` is `"rules"` or `"rules+models"`; `models` names each plug-in used, with its version and weight hash (§6) |
| `source` | `{ "sha256", "pageCount", "tagged", "structSource" }`: the input's SHA-256 (hex; filled in by the layer that holds the bytes, the façade, and absent when unknown), its page count, whether `/MarkInfo /Marked` is true, and where reading order came from: `"tagged"`, `"xycut"` or `"content"` |
| `pages` | one object per extracted page: `index` (0-based), `number` (1-based), `label` (the page label, when there is one), `width` and `height` (displayed, in points), and the `coordinates` block of ADR 0027 §4 (`origin`, `unit`, `box`, `rotate`, `userUnit`) |
| `blocks` | the blocks in reading order, across pages (§2) |
| `dropped` | what extraction removed from the text, and why (§4) |
| `warnings` | ADR 0026 warning objects raised while extracting |
| `chunks` | reserved for L3b's chunker; absent until then |

A **block** has:
- `id`: `"p<index>-b<n>"`, its page index and its position in that page's reading order. Equal input and options give equal IDs.
- `type`: `"heading"` (with `level`, 1–6), `"paragraph"`, `"list-item"` (with `marker` and `depth`), `"caption"`, `"footnote"` or `"other"`. L3b adds `"table"`; formulas, figure descriptions and form-field values are future types.
- `text`: the block's visible text, lines joined by a space after §5's fixes.
- `page` (index), `bbox` (`[x, y, width, height]` in ADR 0027's page space: top-left points of the displayed CropBox) and `bboxNorm` (the same divided by the page's width and height, clamped to [0, 1], absent when the block lies wholly outside the box).
- `structPath` (the structure types from the root to the block, `["Document", "Sect[2]", "H2[0]"]`, empty when untagged) and `structRef` (`{ "obj", "gen", "mcids" }`, when the block comes from one structure element).
- `spans`: `{ "text", "bbox", "baseline", "font", "size", "flags" }` per run, in reading order. ADR 0027 §5 asks for `baseline` beside `bbox`. `flags` is a list of §4's names, empty for visible text.
- `confidence`: in [0, 1], 1 for structure from the tree and lower for inferred structure. Callers may threshold it; its values are not otherwise specified.

Keys are camel case and values kebab case (`"list-item"`, `"off-page"`). Rust has the same model with snake-case names (`bbox_norm`, `struct_path`), with serde behind a `serde` feature; `ExtractedSource` and `ExtractedPage` are the Rust names of a `source` and a `pages` entry.

### 2. Reading order

Three sources, tried in this order unless the caller picks one:

1. **The structure tree**, when `/MarkInfo /Marked` is true and the tree is plausible. Plausible means its MCIDs resolve to content, structure elements cover at least 80% of the visible characters outside artifacts, and its order jumps backwards across columns on at most 10% of a page's block boundaries. The tree's `/K` order is the reading order; `RoleMap` maps custom types to standard ones; untagged content outside artifacts goes after it on its page by XY-Cut++, with `extract-untagged-content`. An implausible tree is ignored with `extract-struct-tree-ignored`. The thresholds are starting values to tune on the corpus; a change is recorded here.
2. **XY-Cut++** otherwise. Blocks at least 60% as wide as the page's text that span two side-by-side narrower blocks and share no height with a narrower block (titles, abstracts, full-width figures) are masked first. They split the page into bands, so a full-width title doesn't stop the column split and a figure between two sections keeps each section's columns together. Each band is then cut recursively at the widest gap in the projection of its block boxes, a column gap before a row gap, so paragraphs whose breaks line up across columns still read column by column. A group whose blocks are all one line (a grid such as a table) cuts rows first instead, reading row by row. The masked blocks go back between the bands.
3. **Content order**, the order the content stream draws, on request (`ReadingOrder::ContentStream`), for debugging and comparison.

Right-to-left text keeps B9b's logical order within a line. A page written right to left as a whole (most of its characters right to left) takes its columns from right to left.

### 3. Blocks from runs

Rules, no models:
- **Lines:** runs on one baseline (within half the font size), in one direction, joined left to right in visual order (B9b already puts each run's text in logical order).
- **Blocks:** consecutive lines in one column whose vertical gap is under 1.5 times the line height and whose left edges or centres align. A change of font size by more than 20% starts a new block. This is Docstrum's idea (O'Gorman 1993) without its nearest-neighbour search.
- **Headings** from font-metric clusters: the body size is the size that covers the most characters; larger sizes, and bold at the body size, rank into up to six levels. Tagged `H1`–`H6` and `H` win when the tree is used.
- **List items** from a leading bullet (`•`, `◦`, `▪`, `–`, `*`) or number (`1.`, `1)`, `a.`, `(a)`, `i.`) followed by a hanging indent; depth from the indent. Tagged `L`/`LI`/`Lbl`/`LBody` win.
- **Vertical writing** is read top to bottom as columns of lines, right to left, with `extract-reading-order-ambiguous` when a page mixes it with horizontal text.

### 4. Hidden text is never in `text`

Each span carries flags, and a flagged span is never part of any block's `text`:
- `invisible`: drawn in render mode 3 or 7 (ISO 32000-2 §9.3.6).
- `off-page`: its box lies wholly outside the CropBox.
- `tiny`: drawn below 1 pt.
- `clipped`: its box lies outside the clip's bounding box. The clip is tracked as a box, not a path, so text inside a curved clip's box counts as visible.
- `low-contrast`: filled within a short distance of white with no fill painted under it. This is a heuristic without rendering.

The last two are approximations, and both lean towards keeping text: a false negative leaves visible noise, a false positive loses real text. A rendered check can come later through the render backend ([ADR 0004](0004-render-backend.md)).

`dropped` lists each removal with its `reason`, `page`, `bbox` and `chars` (a character count, never the text itself, so a log of the output can't leak what was hidden). Reasons are the five flags above, `artifact` (marked content tagged `Artifact`, authoritative when present), `running-header`, `running-footer` and `page-number`. Running headers and footers are lines in the top or bottom 10% of the page that repeat at a similar position on at least three pages once digits are ignored, and page numbers are lines there that hold only a number pattern. A caller auditing a file can ask for the flagged runs themselves (`DropOptions::keep_spans`); they then appear as `spans` in their `dropped` entry, never in a block.

### 5. Text fixes

- **De-hyphenation:** a line ending in `-` (U+002D) or a soft hyphen (U+00AD) joins the next line without the hyphen when that line starts with a lowercase letter; otherwise the hyphen stays (`well-` / `Known`). No dictionary in the core; revisit if fixtures show over-joining.
- **Ligatures** U+FB00–FB06 are decomposed when no `/ActualText` applies (ISO 32000-2 §14.9.4).

### 6. Rules in the core, models as plug-ins

- **The core is rules only.** `laminapdf-extract` is a core crate: bytes in, bytes out, no weights, no network, no clocks, no threads ([ADR 0001](0001-core-language-and-licence.md)).
- **Models plug in through traits** that [L3c](../todos/ready/L3c-model-plugins.md) defines (`LayoutModel`, `OcrEngine`), implemented in edge crates or packages, never in a core crate.
- **Weights are never in the repository or in a published crate or package.** A plug-in downloads them on first use or takes them from the caller, and pins them by SHA-256.
- **Licences:** a model is allowed only when its code *and* its weights are permissive (Apache-2.0, MIT, BSD, or a licence the [supply-chain policy](0019-supply-chain-and-licence-hygiene.md) admits). MinerU, Marker and Surya weights, and PyMuPDF are excluded.
- **Every model used is named** in `generator.models` with its version and weight hash, and `generator.mode` becomes `"rules+models"`, so an output always says how it was made.

### 7. Versioning and determinism

- **Within `/1`**, new keys may appear in any object, and consumers ignore keys they don't know (as qpdf's JSON and ADR 0031 §7 ask). Removing a key, renaming one or changing a key's meaning or type needs `laminapdf.extract/2`.
- **Determinism:** the same bytes and options give byte-identical JSON on every platform. Block IDs are positional, maps are ordered (`BTreeMap`), there are no timestamps, and floats are rounded to 1/1000 of a point before serialising.
- **The JSON Schema file** comes with L3b, generated from the Rust types; its URL waits on the documentation domain decision.

### 8. The API shape

`laminapdf_extract::extract_job(&Document, &ExtractOptions) -> Result<ExtractJob>` starts a resumable job per ADR 0028 §3, and `extract(&Document, &ExtractOptions, &CancelToken)` runs one to completion (ADR 0028 F10). `ExtractJob::step(&doc, Budget)` extracts one page per work unit and returns `Step::Progress` (pages done of the pages asked for) or `Step::Done(Extraction)`; `Budget`, `Step` and `Phase` live in `laminapdf-syntax` beside `CancelToken`, so core jobs and the façade share them. The token is polled per page and inside the XY-Cut recursion; a cancelled job fails with `cancelled` and returns no partial output. `ExtractOptions` is plain owned data (`pages`, `reading_order`, `drop`, `dehyphenate`, `limits`, and `drop.furniture`, the switch for page furniture) with a `Default`, so it crosses UniFFI and WASM as a record. Soft limits (blocks per page, XY-Cut depth) cut work short with a warning, as [ADR 0003](0003-reader-tolerance-and-limits.md) asks of every limit: past the blocks limit, `extract-limit-blocks`; past the depth limit, a group reads top to bottom. The document's warnings stay on the document, and the façade's serialiser writes them as `warnings`.

### 9. What lands when

[L3a](../todos/done/L3a-lamina-extract-reading-order.md) builds §1's model, the layout and content orders of §2, §3's lines and blocks, the `invisible`, `off-page` and `tiny` flags of §4, and §5–§8. [L3d](../todos/done/L3d-extract-structure-and-furniture.md) adds §3's headings and lists from the layout, and §4's artifacts, running headers, footers and page numbers. [L3g](../todos/done/L3g-extract-structure-tree.md) adds structure-tree order, the tagged headings, lists and tables, and the `clipped` and `low-contrast` flags.

> **Implementation note (L3b, 2026-11-05).** [L3b](../todos/done/L3b-extract-tables-and-serialisers.md) adds the block type `"table"` (with `rows`, `cols` and `detector`, `"lattice"` or `"stream"`, and a `cells` array of `{ row, col, rowSpan, colSpan, header, text, bbox }`), a `textRange` on every block (`{ start, end }` into the Markdown rendering, counted in Unicode code points) and the `chunks` key (`{ id, text, blockIds, headingPath, pages, regions, textRange, tokensApprox }`), all within `/1` as §7 allows. The JSON Schema is `crates/laminapdf-extract/schemas/extract.schema.json`, generated from the Rust types; its `$id` uses `https://docs.laminapdf.com/schemas/extract-1.json` until the docs site serves it.

## Consequences

- Retrieval pipelines get text an LLM can cite, with boxes in one documented frame and no hidden text, from a WebAssembly-sized core with no model download.
- Structure-tree order makes well-tagged files exact, and the plausibility check keeps badly tagged ones from doing worse than layout rules.
- Rules fail on magazines, sidebars and floating figures; `confidence` and the crate README say so, and L3c's models are the answer for those pages.
- The `clipped` and `low-contrast` flags miss some hidden text. Rendering would catch more, at the cost of a render backend in the extraction path.
- `dropped` records counts, not text, so a caller who wants to inspect hidden text has to ask for flagged spans.

## Alternatives considered

- **A model in the core** (Docling's layout model through ONNX): rejected. Its weights and runtime would multiply the WebAssembly size, and the licence check would follow every weight update into the core's release.
- **Content order only** (pdftotext's default): rejected as the default. It is right for single-column files and wrong for every multi-column one; it stays available as an option.
- **Docstrum's nearest-neighbour grouping in full:** deferred. Baseline and gap rules give the same blocks on the fixtures at a fraction of the code; revisit if real files need it.
- **Dropping hidden text silently:** rejected. Callers auditing a file need to know something was removed, so `dropped` records each removal, but by count rather than text.
- **Bottom-left coordinates as PDF has them:** rejected by ADR 0027.

> **Implementation note (L3d, 2026-11-05).** [L3d](../todos/done/L3d-extract-structure-and-furniture.md) adds §3's layout headings and list items and §4's `artifact`, `running-header`, `running-footer` and `page-number` drops. §8's switch for them is `DropOptions::furniture` (`drop: { furniture }` in TypeScript), on by default; off keeps them in the blocks. The rules run once every page is read, since the body size and repeated lines are the document's. Choices the ADR left open:
>
> - **Headings:** at most three lines and 200 characters, set at least 10% larger than the body (sizes within 10% share a level), or bold at the body size in at most two lines that don't end in `.`, `,` or `;` and don't start with a list marker. A block that starts with a bullet is never a heading.
> - **Markers:** bullets add `●`, `○` and `■`, which word processors draw. Numbers are up to three digits, one lowercase letter or a lowercase Roman numeral in its standard form, up to `xxxix`. A one-line block after `–` or `*` is an item only next to another item with the same marker, or directly below an item it is indented under.
> - **Blocks:** a line that starts with a bullet starts a block. A line that starts with a dash, an asterisk or a number does so only when the block above is a list item or ends in a colon, so a wrapped line of prose starting `(a)` stays in its paragraph. A lone marker joins the text up to four font sizes beside it, on either side for right-to-left text.
> - **Furniture:** headers repeat at a similar height when their middles are within 2% of the page's height. Page numbers are up to four digits or a lowercase Roman numeral up to `c`, alone, between dashes or brackets, after `Page` or `p.`, or as `N of M` and `N / M`; a bare number from 1900 to 2099 is taken for a year.

> **Implementation note (L3g, 2026-11-06).** [L3g](../todos/done/L3g-extract-structure-tree.md) adds §2's structure-tree order and §4's `clipped` and `low-contrast` flags. The thresholds stay at their starting values (80% coverage, 10% backward moves): the benchmark's scores didn't move with them, and the reading-order corpus has no tagged file near either threshold, so there was nothing to tune them on. Choices the ADR left open:
>
> - **Checks:** a page is checked on its own, so one bad page falls back without the rest. Coverage counts the visible characters outside artifacts that the tree's MCIDs reach. A backward move goes to a block wholly before the previous one in the line direction that starts higher up, so side-by-side blocks in a row don't count. `ReadingOrder::Tagged` skips both checks. `structSource` is `tagged` when the tree ordered every page that has tagged content, `xycut` otherwise.
> - **Blocks:** the first element along a path that isn't a grouping type (`Document`, `Part`, `Sect`, `Div`, `Art`, `Aside`, `BlockQuote`, `TOC`, `L`, `NonStruct`, `Private`, `Index`, `DocumentFragment`) and has a block type is one block. Inside a block, only a list item nested in an item's body and a table's caption start blocks of their own; inline types (`Span`, `Link`, `Em` and so on) join their block. A grouping or inline element that holds marked content outside any block is a paragraph. Content in a table but in no cell keeps its text, after the rows. Within a block, the text reads by its lines, top to bottom. A list item without an `Lbl` gets `•` as its marker. A cell's spans come from `/RowSpan` and `/ColSpan` in `/A`, at most 64 each, and a cell spanning rows takes its columns in the rows below. Coverage leaves artifacts out even when `DropOptions::furniture` keeps them in the blocks.
> - **Clipped:** the interpreter tracks the clip as a box in default user space (each `W`/`W*` path's box, each Form XObject's `/BBox`, intersected, saved by `q`); a run is clipped when its box doesn't overlap that box.
> - **Low contrast:** the colour a glyph is painted in (fill, stroke for stroked-only text, the darker of the two for both) is near white when every RGB component is at least 0.9, in a device space; a named space (such as `ICCBased`, `Separation`, `Indexed` or `Lab`) is never read as near white, and fill-and-stroke text with either colour in one counts as dark. Fills and strokes that aren't near white (a stroke's box grown by half its width), images and shadings that overlap the text, drawn before or after it, are backdrops, so text over them is kept.

> **Implementation note (T3c, 2026-11-08).** [T3c](../todos/ongoing/T3c-tables-across-pages.md) links the parts of a table that runs over a page break. §1's block gains two optional keys within `/1`: `continuedBy` on a part (the ID of the part on the next page) and `continues` on the part after it (the ID of the part before). Each part stays a block of its own, with its own `page`, `bbox`, `rows` and `cells`, so every box still lies on one page (ADR 0027); a table over three or more pages is a chain. The pass runs last, once furniture is gone and block IDs are final, and joins two table blocks when:
>
> - they are on consecutive pages, the first its page's last block and the second the next page's first, with at most one line between them that says the table goes on (the whole word `continued`, `cont` or `cont'd`, with or without a dot, in English, at most 60 characters);
> - the first reaches its page's foot (bottom edge at or below 75% of the height) and the second starts near the top of its own (top edge at or above 25%);
> - they have the same detector (never `tagged`, whose tree joins its own tables) and the same columns: each column's edges within 3 pt for ruled tables, and each column overlapping only the same column of the other part for unruled ones;
> - the second part's first row repeats the first part's header (its cells are then marked `header`), or neither part has a header, or only the first does and the second's first row differs. A second part with a header of its own starts a new table.
>
> Markdown and plain text write a chain as one table: each later part's rows without the header rows it repeats, on the next line, and one HTML `<table>` when any part has a spanning cell. A "continued" line between two parts stays a paragraph in the JSON, but the renderings write it as nothing (its `textRange` is empty), so the table isn't broken. A page that opens with the rest of a table gets its page marker after that part. Chunks never start at a later part or at that line, so a chain is one chunk, even past the budget or with `SplitOn::Pages`. The renderers join two parts only when they are neighbours (or one line apart), so a list of blocks with a part cut out still renders closed tables. `laminapdf extract --max-chars` clears every `continuedBy` that names a block it cut.

