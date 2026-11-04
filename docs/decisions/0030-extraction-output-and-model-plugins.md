# 0030 — Extraction output (`laminapdf.extract/1`) and model plug-ins

- Status: Accepted (2026-11-04, when L3a's PR merges)
- Date: 2026-11-04

## Context

Text extraction ([B9b](../todos/done/B9b-text-extraction-core.md), [B9d](../todos/done/B9d-text-runs-quads-and-page-text.md)) gives runs of glyphs with boxes, in the order the content stream draws them. An LLM or a retrieval pipeline needs more: paragraphs, headings and list items in reading order, each with a box it can cite, and no text a reader can't see. Fewer than 20% of PDFs are tagged, so reading order usually has to be inferred from the layout. Rules do this well enough to be the default: OpenDataLoader's vendor benchmark puts rule-based reading order at about 0.90 at 15 ms a page ([report: rule-based extraction in core, models as licence-clean plug-ins](../research/libraries/shareable-libraries.md#rule-based-extraction-in-core-models-as-licence-clean-plug-ins)). Layout models do better on magazines and complex tables, but most of them carry weights under licences ADR 0001 rules out: MinerU's custom conditions, Marker and Surya's OpenRAIL-M, PyMuPDF's AGPL ([AI extraction note §B](../research/libraries/notes/llm-api-and-ai-extraction.md#b-ai-consumption-primitives-the-2026-landscape-algorithms-bboxprovenance-schema-tagged-pdf-and-rule-based-vs-model-based)).

Hidden text is a security problem as much as a quality one. Text in render mode 3, white on white, clipped away or drawn off the page reaches an LLM's prompt while no person reading the page sees it, which makes it a prompt-injection channel. Extraction is where to stop it.

The output is a contract. Pipelines will store it, index it and compare it across versions, so its keys, its coordinates and its determinism have to be fixed before the first serialiser ships ([L3b](../todos/ready/L3b-extract-tables-and-serialisers.md)). [ADR 0026](0026-error-and-diagnostic-contract.md) (errors), [ADR 0027](0027-coordinates-units-and-page-indexing.md) (coordinates) and [ADR 0028](0028-facade-rules-jobs-and-cancellation.md) (jobs) already constrain it. This ADR fixes the `laminapdf.extract/1` model, the rules that build it, and the policy that keeps models out of the core.

## Decision

### 1. The `laminapdf.extract/1` document

One extraction is one JSON object with these top-level keys, in this order:

| Key | Holds |
| --- | --- |
| `schema` | `"laminapdf.extract/1"` |
| `generator` | `{ "name": "laminapdf", "version", "mode": "rules", "models": [] }`: `mode` is `"rules"` or `"rules+models"`; `models` names each plug-in used, with its version and weight hash (§6) |
| `source` | `{ "sha256", "pageCount", "tagged", "structSource" }`: the input's SHA-256 (hex), its page count, whether `/MarkInfo /Marked` is true, and where reading order came from: `"tagged"`, `"xycut"` or `"content"` |
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
- `spans`: `{ "text", "bbox", "baseline", "font", "size", "flags" }` per run, in reading order. ADR 0027 §5 asks for `baseline` beside `bbox`. `flags` is empty for visible text.
- `confidence`: in [0, 1], 1 for structure from the tree and lower for inferred structure. Callers may threshold it; its values are not otherwise specified.

Rust has the same model with snake-case names (`bbox_norm`, `struct_path`), behind a `serde` feature; the JSON names are camel case.

### 2. Reading order

Three sources, tried in this order unless the caller picks one:

1. **The structure tree**, when `/MarkInfo /Marked` is true and the tree is plausible. Plausible means its MCIDs resolve to content, structure elements cover at least 80% of the visible characters outside artifacts, and its order jumps backwards across columns on at most 10% of a page's block boundaries. The tree's `/K` order is the reading order; `RoleMap` maps custom types to standard ones; untagged content outside artifacts goes after it on its page by XY-Cut++, with `extract-untagged-content`. An implausible tree is ignored with `extract-struct-tree-ignored`. The thresholds are starting values to tune on the corpus; a change is recorded here.
2. **XY-Cut++** otherwise: blocks wider than 60% of the page, sitting above or below the others, are masked first so a full-width title doesn't stop the column split; then recursive projection-profile cuts on the block boxes, the widest gap first, vertical gaps before horizontal ones on a tie; then the masked blocks go back where their top edge falls.
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
- `offPage`: its box lies outside the CropBox.
- `tiny`: drawn below 1 pt.
- `clipped`: its box lies outside the clip's bounding box. The clip is tracked as a box, not a path, so text inside a curved clip's box counts as visible.
- `lowContrast`: filled within a short distance of white with no fill painted under it. This is a heuristic without rendering.

The last two are approximations, and both lean towards keeping text: a false negative leaves visible noise, a false positive loses real text. A rendered check can come later through the render backend ([ADR 0004](0004-render-backend.md)).

`dropped` lists each removal with its `reason`, `page`, `bbox` and `chars` (a character count, never the text itself, so a log of the output can't leak what was hidden). Reasons are the five flags above, `artifact` (marked content tagged `Artifact`, authoritative when present), `running-header`, `running-footer` and `page-number`. Running headers and footers are lines in the top or bottom 10% of the page that repeat at a similar position on at least three pages once digits are ignored, and page numbers are lines there that hold only a number pattern. Callers can keep flagged spans (`Drop` options), but never in `text`.

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

`laminapdf_extract::extract(&Document, &ExtractOptions) -> ExtractJob`, a resumable job per ADR 0028 §3. `step(&doc, Budget)` extracts one page per work unit and returns `Step::Progress` (pages done of the page count) or `Step::Done(Extraction)`. A `CancelToken` is polled per page and inside the XY-Cut recursion; a cancelled job fails with `cancelled` and returns no partial output. `ExtractOptions` is plain owned data (`pages`, `reading_order`, `drop`, `headers_footers`, `dehyphenate`) with a `Default`, so it crosses UniFFI and WASM as a record. Soft limits (blocks per page, XY-Cut depth) drop work with a warning, as [ADR 0003](0003-reader-tolerance-and-limits.md) asks of every limit.

## Consequences

- Retrieval pipelines get text an LLM can cite, with boxes in one documented frame and no hidden text, from a WebAssembly-sized core with no model download.
- Structure-tree order makes well-tagged files exact, and the plausibility check keeps badly tagged ones from doing worse than layout rules.
- Rules fail on magazines, sidebars and floating figures; `confidence` and the crate README say so, and L3c's models are the answer for those pages.
- The `clipped` and `lowContrast` flags miss some hidden text. Rendering would catch more, at the cost of a render backend in the extraction path.
- `dropped` records counts, not text, so a caller who wants to inspect hidden text has to ask for flagged spans.

## Alternatives considered

- **A model in the core** (Docling's layout model through ONNX): rejected. Its weights and runtime would multiply the WebAssembly size, and the licence check would follow every weight update into the core's release.
- **Content order only** (pdftotext's default): rejected as the default. It is right for single-column files and wrong for every multi-column one; it stays available as an option.
- **Docstrum's nearest-neighbour grouping in full:** deferred. Baseline and gap rules give the same blocks on the fixtures at a fraction of the code; revisit if real files need it.
- **Dropping hidden text silently:** rejected. Callers auditing a file need to know something was removed, so `dropped` records each removal, but by count rather than text.
- **Bottom-left coordinates as PDF has them:** rejected by ADR 0027.
