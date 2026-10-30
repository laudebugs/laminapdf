# B5b — Builder text, links, destinations, outlines, page labels and metadata

| Field | Value |
| --- | --- |
| ID | `B5b` |
| Status | `done` |
| Plan | [pdf-library §7 B5](../../plans/pdf-library.md#b5-builder-and-the-lamina-crate), requirements [§4 F2–F5](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B4b](./B4b-shaping-bidi-line-breaking.md), [B4c](./B4c-standard-14-and-simple-fonts.md), [B5a](./B5a-lamina-build-core.md) |
| Blocks | [B5c](../ongoing/B5c-lamina-facade-and-examples.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md) §5–§6 (IDs for created pages/outline items; overlay), [0001](../../decisions/0001-core-language-and-licence.md) (determinism: dates and IDs from options) |
| Branch | `feat/b5b-builder-text-and-navigation` |
| PR title | `feat(build): text runs and text boxes, links, destinations, outlines, page labels, Info and XMP` |
| Size | L (≈ 1,000–1,400 lines) |
| PR | [#63](https://github.com/laudebugs/lamina/pull/63) |

## Goal

Complete the builder's feature list for M1: shaped text runs and single-box paragraphs on a surface, link annotations (URI and internal), named destinations, outlines, page labels, the Info dictionary and an XMP metadata stream kept in sync with it, document language and viewer preferences. After this PR, every example in the library plan can be written in Rust.

## Context

- [pdf-library §5 API sketch](../../plans/pdf-library.md#api-sketch-illustrative-not-final): `s.text(...)`, `s.textBox(...)`, `link: { uri, contents }`, `b.outline([...])`.
- [Research 03 §2](../../research/pdf/03-text-and-fonts.md#2-text-objects-and-operators-iso-32000-2-9294) (text operators, `Tr` modes incl. invisible), [§5.3 ActualText](../../research/pdf/03-text-and-fonts.md#53-ligatures-reordering-and-actualtext), [§11.2](../../research/pdf/03-text-and-fonts.md#112-concrete-requirements) (real space glyphs).
- [Research 04 §1](../../research/pdf/04-interactive-security.md#1-navigation) (destinations, outline, actions, link annotations, page labels, viewer preferences), [§5.1–5.3](../../research/pdf/04-interactive-security.md#5-metadata) (Info, XMP, IDs).
- `lamina-text` from [B4a](../done/B4a-font-loading-and-subsetting.md)/[B4b](../done/B4b-shaping-bidi-line-breaking.md)/[B4c](../done/B4c-standard-14-and-simple-fonts.md).

## Scope

**In scope**

- Positions in page space as `lamina-build`'s surface takes them (ADR 0027; `PointPt`/`RectPt`, mapped through the page's `PageFrame` like paths); **`at`'s `y` is the baseline** (ADR 0027 §5; the acceptance criterion carried over from [B5a](../done/B5a-lamina-build-core.md)).
- `surface.text(str, &Font, size, at)` returning a `TextRun` handle (`.fill(color)`, `.features(...)`, `.render_mode(...)`, `.tag(...)` stubbed for [B8a](../ready/B8a-structure-tree.md)); shaped via B4b; emits `BT … Tf … Tm … TJ … ET`, real U+0020 space glyphs kept in the run (03 §11.2), `ActualText` marked content for clusters flagged by B4b.
- `surface.text_box(str, &Font, size, Rect, TextBoxOptions { align, line_height, overflow: Clip | Error })` using B4b's greedy breaking, bidi-aware; returns `TextLayout` with line boxes and overflow info.
- Links: `surface.link(Rect, LinkTarget::{Uri(String), Page { page: PageRef, fit }, Named(String)}, contents: Option<String>)` → `/Annot /Subtype /Link` with `/A /URI` or `/Dest`, `/Contents`, `/F 4` (print), `/Border [0 0 0]`; also the `link:` option on `text()` covering the run's bounding box.
- Named destinations: `builder.destination(name, page, fit)` → `/Names /Dests` name tree (sorted, deterministic).
- Outlines: `builder.outline(items)` (nested, `open`, colour/style flags) → `/Outlines` with `/First/Last/Prev/Next/Parent/Count`.
- Page labels: `builder.page_labels(ranges)` → `/PageLabels` number tree.
- Metadata: `Info { title, author, subject, keywords, creator, producer, creation_date, mod_date }` (dates supplied, never read from a clock); XMP packet writer (`dc:title`, `dc:creator`, `dc:description`, `pdf:Keywords`, `pdf:Producer`, `xmp:CreateDate`, `xmp:ModifyDate`, `xmp:CreatorTool`) uncompressed `/Metadata` stream; `lang` → `/Lang`; viewer preferences (`DisplayDocTitle`, page mode, page layout).
- Created pages and outline items get `LaminaId`s through the overlay allocator.

**Out of scope** (and which todo covers it)

- Structure tree, `tag()`, artifacts, `Link` structure elements with OBJR, alt text: [B8a](../ready/B8a-structure-tree.md).
- PDF/A identification XMP schemas and output intents: [B8b](../ready/B8b-conformance-profiles-and-verapdf.md).
- Editing Info/XMP on existing documents in sync: full-editor E1.
- Font fallback across fonts within one run: later.

## Tasks

- [x] Load `/rust-best-practices`; read 03 §2, §5.3, §11.2 and 04 §1, §5
- [x] `surface/text.rs` (runs), `surface/textbox.rs`, content emission with `pdf_writer::Content`
- [x] `annot/link.rs`, `nav/{dest,outline,labels}.rs`, `meta/{info,xmp,prefs}.rs`
- [x] PDF text-string encoding shared with `lamina-ops` `InfoPatch` (move to a common place if duplicated): already one function, `lamina_syntax::text::encode_text_string`, which both use; nothing to move
- [x] XMP writer (deterministic attribute order, escaped values); tests that Info and XMP agree
- [x] Goldens: `text-latin.pdf`, `text-multiscript.pdf`, `links-and-outlines.pdf`, `metadata.pdf` (deviation: the text ones are `build-text-latin.pdf` and `build-text-multiscript.pdf`, as `text-latin.pdf` is lamina-text's and `text-*.pdf` is the glob `tools/fixtures/text_facts.py` reads)
- [x] Full-lane checks: pdf.js link/outline extraction equals the builder's input; pypdf/pdftotext text equals input (as committed facts, the repository's pattern: `tools/fixtures/build_facts.py` + `pdfjs_build.mjs` write `fixtures/build/facts.json`, `tests/external.rs` checks it; the goldens also joined the round-trip suite's `PLAIN_BUILT`, so the full lane's `cargo xtask roundtrip` runs qpdf, pdf.js and PDFium on them and their saves; pdftotext 26.10.0 joined the facts once B4d pinned it)

## Acceptance criteria

- [x] Text in five scripts (Latin, Arabic, Devanagari, Hebrew, CJK) generated via `text()` and `text_box()` extracts equal to the input in pdf.js, pypdf and pdftotext (after B4d: every line exact in pypdf; Latin, CJK and the standard 14 exact in all three; the extractor limitations already known from `text-shaped.pdf` are listed with reasons in `fixtures/build/extraction-exceptions.json`: pdf.js's space before the Devanagari i-matra, and pdftotext writing right-to-left lines in display order inside U+202B…U+202C)
- [x] A text box never produces a line wider than its box; overflow is reported or clipped per option
- [x] URI and internal links, named destinations and outlines resolve correctly in pdf.js (`getOutline`, `getDestination`, annotations) and in `lamina-model` views (round-trip through B2c's views)
- [x] Page labels read back through `lamina-model::PageLabels` and pypdf `page_labels` (and pdf.js `getPageLabels`)
- [x] Info and XMP carry the same values; `qpdf --check` clean; byte-identical across runs
- [x] No ambient time: building twice with the same options gives identical bytes

## Deviations

- **API names.** `Font` is the builder's registered-font handle (`Builder::font`, `font_with`, `standard_font`; also on `Appender`); outline items are `OutlineEntry` and label ranges `PageLabel` (`lamina-model` already has `OutlineItem` and `LabelRange` for reading); the Info values are `Metadata` (written as Info and XMP, set with `Builder::metadata`).
- **Page targets are 0-based indexes** (`LinkTarget::Page { index: u32, fit }`, ADR 0027 §6), resolved when the build finishes so a link can name a later page; there is no `PageRef` in the core (ADR 0028 F2 puts handles in the edges).
- **`tag(...)` is not stubbed**: a public no-op would promise behaviour B8a defines; B8a adds it.
- **`text()` and `text_box()` return `Result`** (shaping errors such as `missing-glyph` come back at the call); `TextRun`'s shaping changes (`features`, `language`, `direction`) do too.
- **Standard 14 fonts** are laid out by the builder (one glyph per character with AFM widths, UAX #14 breaks, left to right), since `lamina-text` refuses to shape them.
- **Overflow::Clip** draws the lines that fit and reports where the rest starts; it does not add a clipping path (no drawn line crosses the box).
- **Runs are drawn in logical order** within a line (each at its own x), through B4d's `Line::drawing_order`, so extractors read multi-run right-to-left lines in order.
- **XMP drops C0 control characters** other than tab, line feed and carriage return (XML 1.0 can't hold them), so Info and XMP differ only for such text.
- **Goldens**: see the tasks above.
- **Code review** (20 findings, fixed in one commit; see its message): appends resolve every link before changing anything and roll the overlay back on a failed write; a failed page is rolled back with the font registry, resources and links it recorded (`FontRegistry` became `Clone`, a two-word additive change in `lamina-text`, which also now exports `is_hard_break`/`is_trailing_space` for the standard 14 layout); `/FitH`/`/FitV` operands are user-space y/x, `null` on pages turned sideways; a negative zoom is `invalid-geometry`, zoom 0 is `null`; Info and XMP drop the same XML-illegal characters.

## Post-merge checks

None.

## Tests

- Unit tests per module; `crates/lamina-build/tests/{text,textbox,links,outline,labels,metadata}.rs`.
- Goldens under `fixtures/golden/`; external checks in the full lane ([X4](../done/X4-full-lane-gate.md)).

## Docs to update

- [x] [pdf-library §10 B5](../../plans/pdf-library.md#10-to-dos): tick text/text box part of "Surface ops" and "Document ops"
- [x] Crate README; `fixtures/README.md`

## Risks and open questions

- Link rectangles for rotated text: use the run's transformed bounding box; `QuadPoints` for rotated links later.
- XMP vs Info precedence on read is an editor concern (E1); here they are written identically.
- `text()` options grow quickly; keep a builder-style API with `#[non_exhaustive]` option structs.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
