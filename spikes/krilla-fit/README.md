# Spike: `krilla-fit` (Round B1)

- **Date:** 2026-10-28
- **Status:** done. The recommendation below is recorded in [ADR 0005 §6](../../docs/decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) and is accepted when this spike's PR merges (owner sign-off).
- **Plan:** [architecture §4](../../docs/plans/architecture.md#4-spikes-to-close-open-questions) (`krilla-fit`), [library plan §7 B1, §9 Q1](../../docs/plans/pdf-library.md#9-open-questions)
- **Code:** throwaway-quality, outside both workspaces, never imported by product code ([code organisation §1](../../docs/guidelines/code-organization.md#1-top-level-layout)).

## Question

Should lamina's generation layer (`lamina-build`, Round B5) **depend on krilla**, or **build its own builder over `lamina-cos`** while reusing krilla's lower-level building blocks (`pdf-writer`, `subsetter`, `skrifa`, HarfRust)? Specifically: **can krilla write into an existing object graph**, so that the objects it produces can live in a lamina `Document` and be edited and saved incrementally? Or can it only emit a finished byte stream?

## Answer in one paragraph

**No. krilla can only emit a finished byte stream.** `Document::finish()` returns `Vec<u8>` and is the only output. Object numbers are allocated inside a private `SerializeContext` starting at 1. At `finish`, `ChunkContainer::finish` renumbers every chunk into a fresh `pdf_writer::Pdf` and writes its own catalog, XMP, `/ID` and classic xref. All of this is `pub(crate)` in krilla 0.8.2 (`document.rs`, `serialize.rs`, `chunk_container.rs`), and nothing public exposes a `Ref`, a `Chunk` or a "numbering start". krilla's output is clean, though, and lamina-cos reads and edits it without trouble (experiment A). Grafting krilla's pages into an existing lamina document by copying and renumbering works and is fast (0.11 ms for 3 pages). The cost is that catalog-level features (outline, `Lang`, page labels, names) then need merge code, and **tagging doesn't survive**: without a structure-tree merge (`ParentTree` renumbering), the imported pages have to be stripped of their tags (experiment B). A builder written directly over lamina-cos, reusing `pdf-writer::Content`, `skrifa` and `subsetter`, produced the same document in about 400 lines of spike code, at the same size and speed as krilla (experiment C). **Recommendation: build `lamina-build` over lamina-cos and reuse krilla's building blocks (hybrid). Don't depend on krilla in the core; treat it as a reference implementation.**

## Method

One standalone Cargo project (`[workspace]` table, path dependencies on `crates/lamina-syntax` and `crates/lamina-cos`). Every experiment generates the **same three A4 pages**:

1. A heading and four lines of text in Noto Sans Regular (OFL 1.1), embedded as a subset (curly quotes, accents, `fi`/`ffi`). The page also has vector paths (a stroked semi-transparent rectangle, a triangle and a four-cubic circle) and a URI link annotation over line 3.
2. A 192×192 RGBA PNG with a radial alpha falloff (generated in `assets.rs`), embedded as an image with `/SMask`, and a 192×192 baseline JPEG (`assets/photo.jpg`, from `scripts/make-jpeg.sh`), passed through as `DCTDecode`.
3. Text only, so the outline has three entries.

The document also has an outline with one entry per page, `/Info /Title` and `/Lang en`.

| Experiment | What it does |
| --- | --- |
| **A** krilla as-is (`src/exp_a.rs`) | krilla 0.8.2 generates the document (also a tagged variant: one `P` per line). lamina-cos opens it, walks every page and object, rewrites `/Info /Title` with `incremental_update`, and reopens the result. |
| **B1** krilla → bytes → lamina graph (`src/graph.rs`) | Copies krilla's pages into an *existing* document (`fixtures/corpus/hello.pdf`, then the C document, then an already-tagged document). The transitive closure is renumbered into the base's numbering space, inherited attributes (`MediaBox`, `CropBox`, `Rotate`, `Resources`) are materialised, `/Parent` is set to the base's `/Pages` root after the rewrite, references are keyed on (number, generation), and references to missing objects are copied as `null` (§7.3.10). Catalog-level entries (`Outlines`, `Lang`, `Names`, `ViewerPreferences`) are imported when the base lacks them and reported as conflicts otherwise; `PageLabels` are always reported (they would need offsetting). Tagging is always dropped: `StructTreeRoot` and `MarkInfo` are not imported and `/StructParents`/`/StructParent` are removed from every copied object, because without a `ParentTree` merge the result can't honestly claim to be tagged. The marked-content operators (`BDC … /MCID`) stay in the content streams, where they are harmless without a structure tree. Base `/Kids` is resolved if indirect, and a non-array is an error. The tool checks every page's `/Parent` after reopening. The result is saved with `incremental_update`. |
| **B2** pdf-writer `Chunk` in lamina's numbering (`src/exp_b.rs`) | pdf-writer lets the caller choose every `Ref`. Refs are allocated from the base's next free number, a page, a content stream, a font and a link are written with pdf-writer's typed writers, the chunk bytes are parsed back into lamina `Object`s, and the result is appended incrementally. No renumbering is needed. |
| **C** lamina-native builder (`src/exp_c.rs`) | `lamina_cos::Writer` holds lamina `Object`s. `pdf_writer::Content` writes content-stream operators (bytes only, no refs), `skrifa` supplies the cmap and metrics, `subsetter` subsets the font, and `png` decodes the image. The Type 0 font object is **reserved** up front and filled at `finish()`, after the subset is known. The result is a Type 0/CIDFontType2 font with Identity-H, `/W`, ToUnicode and a deterministic subset tag. |
| **WASM size** (`wasm-size/`, `scripts/wasm-size.sh`) | The A and C generators (the same source files) are built as `wasm32-unknown-unknown` cdylibs with `opt-level = "s"`, LTO and strip. Inputs go through `black_box`, so every code path stays. |

Run it with `scripts/fetch-font.sh` (once; it checks the SHA-256) and then `cargo run --release`. The measurements table is printed to stdout and the PDFs are written to `target/krilla-fit/` (not committed). Every output passes `pypdf.PdfReader(strict=True)`: page counts are right, text extraction returns the input strings (including the curly quotes and accents), and the outline has three entries. Pages 1 and 2 of A and C rasterise identically (checked with macOS `sips`).

## Measurements

Apple Silicon laptop, Rust 1.99.1, `--release` (default release profile, no LTO), best of 30 runs.

| Measurement | krilla (A) | lamina builder (C) |
| --- | --- | --- |
| Generate the 3-page document | **3.98 ms** (tagged: 4.04 ms) | **3.51 ms** (with object and xref streams: 3.53 ms) |
| Output size | 104 497 B (tagged: 106 213 B) | 102 624 B (with object and xref streams: 100 977 B) |
| Embedded font program (subset, Flate) | 8 201 B | 8 147 B (raw subset 13 552 B) |
| Byte-identical across runs | yes | yes |
| lamina-cos open + walk every page and object | 0.068 ms | 0.057 ms |
| Incremental `/Info /Title` update with lamina-cos | 0.019 ms, +351 B; reopens with the new title, 3 pages | same; reopens with the new title |

| Graph integration (B) | Result |
| --- | --- |
| B1: import krilla's 3 tagged pages into `hello.pdf` (incremental) | **0.106 ms**, 27 objects copied, +103 449 B; `Outlines` and `Lang` imported (base had none); tagging dropped (`StructTreeRoot`, `MarkInfo`, 3 `/StructParents`); reopens with 4 pages, every `/Parent` is the base's `/Pages`, and the outline points at pages 2–4 |
| B1: the same into the C document (which has an outline and `Lang`) | 0.103 ms, 20 objects, +102 870 B; **conflicts: `Outlines`, `Lang`** (not merged: the imported pages are missing from the outline); 6 pages |
| B1: the same into an already-tagged document (tagged A into itself) | conflicts: `Outlines`, `Lang`; tagging of the imported pages dropped, so **only the base's 3 pages stay tagged**; 6 pages |
| B2: one page written by pdf-writer directly in lamina's numbering, parsed and appended | 0.029 ms, 4 objects, +813 B; reopens with 4 pages |
| Parse and re-serialise every krilla object as lamina `Object`s | 0.074 ms for 101 KiB |

| WASM (`wasm32-unknown-unknown`, opt-level s, LTO, strip; no wasm-opt) | raw | gzip |
| --- | --- | --- |
| lamina-cos only (baseline: open + page count) | 128 502 B | 59 851 B |
| + krilla generator (A) | 2 312 841 B | **829 947 B** |
| + lamina builder with pdf-writer, skrifa and subsetter (C) | 964 619 B | **377 499 B** |

The WASM numbers are upper bounds: no wasm-opt, `anyhow` and the `png` decoder in both, and C has **no shaper** yet (HarfRust will add to it in B4; krilla's figure includes rustybuzz and its JPEG/GIF/WebP decoders from the default `raster-images` feature). Even so, **both exceed the plan's 350 KB gzip budget** for `@lamina/pdf` (shaping and subsetting included): krilla by more than 2×, and the C stack (377 KB) already before a shaper is added. There is no headroom. Whichever way we go, B6 has to (a) split the WASM into a base module (COS, model, save, build without text) and a lazily loaded text module (skrifa, subsetter, HarfRust), as ADR 0016 anticipates, (b) measure again with wasm-opt and without `anyhow`/`png` (the probe includes both), and (c) revise the budget in ADR 0016 if the text module can't fit. The point here is relative: own crates give us control over what goes into each module; krilla is one indivisible dependency.

Times are dominated by Flate-compressing the 147 KB RGBA image. Both generators already fit within the plan's "hello world ≤ 5 ms" target on this machine for a heavier document. **Speed and file size don't decide it (the two approaches are equivalent). WASM size leans towards the own builder, but neither meets the budget yet.**

## Licences

`cargo deny --manifest-path spikes/krilla-fit/Cargo.toml --config deny.toml check licenses bans sources` with the repository's `deny.toml` reports **licenses ok, bans ok, sources ok**. The only warning is a duplicate `miniz_oxide` (0.8 via `png`, 0.9 via lamina and `flate2`). krilla 0.8.2 itself is `MIT OR Apache-2.0`. Its full dependency licence set with default features (`raster-images`, `simple-text`) contains only licences on our allow-list:

- `MIT OR Apache-2.0`: most crates (krilla, pdf-writer, subsetter, skrifa, read-fonts, write-fonts, font-types, flate2, png, gif, image-webp, bumpalo, indexmap, kurbo, xmp-writer, rustc-hash, siphasher, smallvec, …)
- `MIT`: rustybuzz, float-cmp, imagesize, libm, core_maths, color_quant, simd-adler32, strict-num, synstructure
- `BSD-3-Clause`: tiny-skia-path
- `Unicode-3.0`: yoke, zerofrom (+ derives)
- `MIT OR Apache-2.0 OR Zlib`: zune-jpeg, zune-core, bytemuck, miniz_oxide
- `MIT OR Unlicense`: memchr, byteorder-lite (we take MIT)
- `BSD-2-Clause`: arrayref
- `Apache-2.0 OR BSL-1.0`: ryu

**Licences don't block either option.** The spike first used `jpeg-encoder` to make the JPEG, but its licence is `(MIT OR Apache-2.0) AND IJG`, which cargo-deny rejects. That is why the JPEG is made with macOS `sips` and committed (3 KB). This is worth remembering if we ever need a JPEG *encoder*.

## krilla's status (checked 2026-10-28)

- crates.io: **0.8.2** (2026-06-04), `MIT OR Apache-2.0`, MSRV 1.92, about 3.3 M downloads. Releases: 0.4 (2025-04), 0.5 (2025-10), 0.7 (2026-04), 0.8 (2026-05). As a 0.x crate, a minor release may break the API; `main` already has breaking changes queued (for example, "Make the page media box mandatory", LaurenzV/krilla#466).
- GitHub `LaurenzV/krilla`: active (35 commits in the last 3 months; the latest, on 2026-10-28, returns an error for standard fonts in PDF 2.0 where it used to panic). It is mostly a single-maintainer project: LaurenzV has about 690 of about 770 commits. Typst depends on it.
- On `main` (unreleased), krilla has moved from `rustybuzz` to **`harfrust` 0.12**, uses `skrifa` 0.44 (0.8.2 pins `skrifa` 0.42, while crates.io has 0.49), and takes `pdf-writer` and `hayro-write` as **git dependencies**. A krilla dependency would therefore lag the font stack, and we would wait for releases.
- The optional `pdf` feature (`hayro-write`) works in the opposite direction: krilla *embeds pages of an existing PDF* into a new krilla document (`Document::embed_pdf_pages`, `Surface::draw_pdf_page`). The result is still a new file, rewritten from scratch, with no incremental save, so existing signatures are lost. This doesn't help us write into a `Document`.
- No issue or roadmap item asks for "emit into a caller's object graph" or for incremental output.

## Findings

1. **krilla is write-once by design.** Pages borrow the global `SerializeContext`. Fonts, page trees, outlines, named destinations and the structure tree are serialised only in `SerializeContext::finish`, and everything is renumbered at the end. Getting objects into a caller's graph would mean (a) a public "finish into chunks with a ref allocator" API upstream, plus a way to *not* write the catalog, `/Info`, XMP and the trailer, or (b) forking krilla. The upstream change is a sizeable redesign of a single-maintainer crate, not a small patch.
2. **The "krilla → bytes → import" adapter works for pages but not for documents.** Page content, resources, fonts, images and annotations copy cleanly (renumbering costs microseconds). But every catalog-level feature krilla produces has to be **merged** into the target. That means inserting outline items into an existing outline tree, offsetting page labels, and merging `Names`/`Dests` and `Lang`. Above all, it means **merging the structure tree**: `/StructParents` and `/StructParent` keys must be renumbered into the target's `/ParentTree`, the `/K` arrays spliced, and `RoleMap`/`ClassMap` merged. The spike doesn't do this; it strips the imported pages' tagging instead (into an untagged base too, since a base with untagged pages can't be declared `/Marked true`). That is a page-merge engine, which the editor needs anyway (full-editor merge/insert pages), but making it a prerequisite for *generation* is backwards. It also means generating twice (serialize with krilla, parse with lamina), which the numbers show is cheap but pointless.
3. **PDF/UA and PDF/A by construction don't survive the adapter.** krilla's validators check the file *krilla* writes. After import into a lamina document, `/Lang`, the title, XMP identification, `OutputIntents` and the structure tree come from the target, so conformance would have to be re-validated by lamina anyway. B8's profiles therefore need lamina-side checks whichever way we go.
4. **Font subsetting only needs a deferred object, not krilla's architecture.** "The subset is known only at `finish`" maps directly onto `Writer::reserve()` + `set()` (or a `LaminaId` placeholder in the B2 model). Page content refers to the reserved font ref, and the font program is written last. The same pattern works for an incremental "add a page with text to an open document" (B3/E2), which is what the apps need ("stamps, headers/footers, images → PDF" in the editor). krilla can't do that at all.
5. **pdf-writer fits lamina's graph.** The caller allocates every `Ref` (B2), and `Content` produces bytes with no refs. That makes pdf-writer usable as a typed serialisation helper. For object dictionaries, though, lamina already has `Object`/`Dict`, so `lamina-build` should use **`pdf_writer::Content`** for operators and write dictionaries as lamina `Object`s. Parsing pdf-writer chunks back (B2) works but adds an unneeded serialize→parse round trip.
6. **Writing our own is small for the parts we need, and large for the parts krilla is good at.** The spike's C builder (Type 0 + Identity-H + ToUnicode + `/W` + PNG/SMask + JPEG + paths + links + outline) is about 400 lines and matches krilla's output. What it leaves out is real work, and krilla is the reference for each item: shaping with HarfRust (ligatures, kerning, complex scripts; C maps one glyph per char), CFF/OTF and variable-font instancing (`subsetter` has `subset_with_variations`), compact `/W` runs, COLR/SVG/bitmap glyphs (Type 3), gradients/patterns/masks/blend modes, and above all **tagging + PDF/UA + PDF/A validators**. These are B4 and B8 effort either way.

## Recommendation (ADR 0005 §6, accepted when this PR merges)

**Hybrid: build `lamina-build` over `lamina-cos` (later `lamina-model` + `lamina-save`). Reuse krilla's building blocks as dependencies, and use krilla itself as a reference, not as a dependency of the core.**

- **Depend on:** `pdf-writer` (`Content` for operators, and possibly its typed writers in tests and goldens), `subsetter` (TrueType/CFF/CFF2 subsetting, variable instancing), `skrifa`/`read-fonts` (cmap, metrics, outlines), `harfrust` (shaping, B4). All are MIT OR Apache-2.0, maintained, and WASM-clean.
- **Don't depend on `krilla` in core crates.** It can't target our graph (finding 1), the adapter breaks tagging and catalog features (2, 3), it pins the font stack behind its release cycle, and its generator alone is about 830 KB gzipped in WASM against a 350 KB budget (our own stack is over budget too, at 377 KB before shaping, but can be split into lazily loaded modules). Its API shape (`Document → page → surface → finish`) is already what the [library plan](../../docs/plans/pdf-library.md#api-sketch-illustrative-not-final) sketches.
- **Use krilla as the reference and the oracle.** Port ideas, with attribution where code is adapted (MIT/Apache permit it): the font container and `/W` compaction, the Type 3 fallback for colour glyphs, the tag tree → `ParentTree` bookkeeping, and the validator rule lists (`src/configure/PDF_UA1.md`, `PDF_A*.md`). A dev-dependency on krilla in a test-only crate can generate the same documents for differential tests.
- **Fit with ADR 0005:** `Document` owns an `Arc<dyn ByteSource>` base plus a dirty overlay. The builder allocates its objects in the overlay as `New` entries (the spike's `Writer::reserve`/`set` is the stand-in), so a generated document is a `Document` over an empty base, and "add a page or stamp to an open document" is the same code followed by an incremental save. With krilla, the generated `Vec<u8>` could simply become the base (`Vec<u8>` moves into the `Arc`), which is easy, but it only covers new documents. Adding to an existing document would still need the B1 import into the overlay.
- **Optional escape hatch (not in core):** if someone needs krilla-only features before we have them, an edge crate such as `lamina-krilla` could expose "import a krilla-generated PDF's pages into a `Document`". This is the B1 adapter, built on the page-import code that full-editor merge/insert needs anyway, and documented as untagged-only until structure-tree merge exists.

### Trade-offs

| | Depend on krilla (adapter) | Own builder, reuse crates (**recommended**) | Upstream "emit into caller graph" in krilla |
| --- | --- | --- | --- |
| Time to first generated PDF | Fastest (works today) | B4 + B5 effort (the spike's C builder shows the core is small) | Unknown: needs maintainer buy-in and a redesign |
| Generated objects editable in the same `Document` | Only after a serialize → parse → import round trip | Yes, natively; also "add content to an open document" incrementally | Yes, if it lands |
| Tagging / PDF/UA / PDF/A | krilla's validators, but lost or re-validated after import into an existing document | We build them (B8), with krilla as the reference | krilla's, if its catalog handling is made optional |
| Font stack currency | Pinned to krilla's releases (skrifa 0.42 vs 0.49 today) | We pick versions | Same as the krilla column |
| Bus factor / API churn | krilla 0.x may break on every minor; mostly one maintainer | Our own API; dependencies are small, stable crates | Same as the krilla column |
| WASM size (gzip, no wasm-opt; budget 350 KB) | 830 KB with lamina-cos (includes rustybuzz and krilla's image decoders); one indivisible dependency | 377 KB without shaping, already over budget; HarfRust comes on top, so B6 must split modules or revise the budget (ADR 0016) | — |

### Risks

- **Effort:** text (shaping, bidi, line breaking, complex scripts) and B8 conformance are the expensive parts, and they are the parts where krilla is mature. *Mitigation:* reuse HarfRust/subsetter/skrifa as krilla does, port krilla's approaches with attribution, and use krilla-generated files as differential oracles in tests.
- **Diverging from the Typst ecosystem:** improvements to krilla won't flow to us automatically. *Mitigation:* contribute upstream to the shared crates (`subsetter`, `pdf-writer`, HarfRust), not to krilla.
- **If the owner prefers krilla anyway:** the adapter in `src/graph.rs` shows the path. Budget for outline, page-label and names merging, and accept "imported pages are untagged when the target is tagged" until a structure-tree merge exists.

## What B4 and B5 should do

- **B4 (`lamina-text`):** `skrifa` for loading and metrics, `harfrust` for shaping (the version krilla `main` uses, so the two stay comparable), `subsetter` with `subset_with_variations`, ToUnicode and `/W` writers (compact runs as krilla does), deterministic subset tags (the spike hashes the glyph set), and a `FontContainer`-like accumulator whose PDF objects are reserved at first use and written at `finish`.
- **B5 (`lamina-build`):** the builder writes lamina `Object`s into the ADR 0005 overlay (reserving object numbers for deferred objects such as fonts, the outline and the structure tree) and uses `pdf_writer::Content` for content streams. `finish()` returns a `Document`. Golden tests compare against byte-identical fixtures, and a test-only krilla generator gives differential checks (text extraction, page boxes, rendering via PDFium/hayro).
- **Later (full editor):** the page-import code from `src/graph.rs` grows into merge/insert-pages. It is also the optional krilla adapter, if one is ever wanted.

## Files

- `src/assets.rs`: inputs (font from `fonts/`, the generated PNG, the committed JPEG, the shared page text)
- `src/exp_a.rs`: the krilla generator (A)
- `src/graph.rs`: page import with renumbering (B1)
- `src/exp_b.rs`: pdf-writer chunk in lamina's numbering (B2)
- `src/exp_c.rs`: the lamina-native builder (C)
- `src/main.rs`: runs everything and prints the measurements table
- `wasm-size/`: the WASM size probe; `scripts/wasm-size.sh`
- `scripts/fetch-font.sh` (Noto Sans, OFL 1.1, `fonts/OFL.txt`), `scripts/make-jpeg.sh`
