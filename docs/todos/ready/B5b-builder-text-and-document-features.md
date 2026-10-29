# B5b — Builder text, links, destinations, outlines, page labels and metadata

| Field | Value |
| --- | --- |
| ID | `B5b` |
| Status | `ready` |
| Plan | [pdf-library §7 B5](../../plans/pdf-library.md#b5-builder-and-the-lamina-crate), requirements [§4 F2–F5](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B4b](./B4b-shaping-bidi-line-breaking.md), [B4c](./B4c-standard-14-and-simple-fonts.md), [B5a](./B5a-lamina-build-core.md) |
| Blocks | [B5c](./B5c-lamina-facade-and-examples.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md) §5–§6 (IDs for created pages/outline items; overlay), [0001](../../decisions/0001-core-language-and-licence.md) (determinism: dates and IDs from options) |
| Branch | `feat/b5b-builder-text-and-navigation` |
| PR title | `feat(build): text runs and text boxes, links, destinations, outlines, page labels, Info and XMP` |
| Size | L (≈ 1,000–1,400 lines) |
| PR | — |

## Goal

Complete the builder's feature list for M1: shaped text runs and single-box paragraphs on a surface, link annotations (URI and internal), named destinations, outlines, page labels, the Info dictionary and an XMP metadata stream kept in sync with it, document language and viewer preferences. After this PR, every example in the library plan can be written in Rust.

## Context

- [pdf-library §5 API sketch](../../plans/pdf-library.md#api-sketch-illustrative-not-final): `s.text(...)`, `s.textBox(...)`, `link: { uri, contents }`, `b.outline([...])`.
- [Research 03 §2](../../research/pdf/03-text-and-fonts.md#2-text-objects-and-operators-iso-32000-2-9294) (text operators, `Tr` modes incl. invisible), [§5.3 ActualText](../../research/pdf/03-text-and-fonts.md#53-ligatures-reordering-and-actualtext), [§11.2](../../research/pdf/03-text-and-fonts.md#112-concrete-requirements) (real space glyphs).
- [Research 04 §1](../../research/pdf/04-interactive-security.md#1-navigation) (destinations, outline, actions, link annotations, page labels, viewer preferences), [§5.1–5.3](../../research/pdf/04-interactive-security.md#5-metadata) (Info, XMP, IDs).
- `lamina-text` from [B4a](../ongoing/B4a-font-loading-and-subsetting.md)/[B4b](./B4b-shaping-bidi-line-breaking.md)/[B4c](./B4c-standard-14-and-simple-fonts.md).

## Scope

**In scope**

- `surface.text(str, &Font, size, at)` returning a `TextRun` handle (`.fill(color)`, `.features(...)`, `.render_mode(...)`, `.tag(...)` stubbed for [B8a](./B8a-structure-tree.md)); shaped via B4b; emits `BT … Tf … Tm … TJ … ET`, real U+0020 space glyphs kept in the run (03 §11.2), `ActualText` marked content for clusters flagged by B4b.
- `surface.text_box(str, &Font, size, Rect, TextBoxOptions { align, line_height, overflow: Clip | Error })` using B4b's greedy breaking, bidi-aware; returns `TextLayout` with line boxes and overflow info.
- Links: `surface.link(Rect, LinkTarget::{Uri(String), Page { page: PageRef, fit }, Named(String)}, contents: Option<String>)` → `/Annot /Subtype /Link` with `/A /URI` or `/Dest`, `/Contents`, `/F 4` (print), `/Border [0 0 0]`; also the `link:` option on `text()` covering the run's bounding box.
- Named destinations: `builder.destination(name, page, fit)` → `/Names /Dests` name tree (sorted, deterministic).
- Outlines: `builder.outline(items)` (nested, `open`, colour/style flags) → `/Outlines` with `/First/Last/Prev/Next/Parent/Count`.
- Page labels: `builder.page_labels(ranges)` → `/PageLabels` number tree.
- Metadata: `Info { title, author, subject, keywords, creator, producer, creation_date, mod_date }` (dates supplied, never read from a clock); XMP packet writer (`dc:title`, `dc:creator`, `dc:description`, `pdf:Keywords`, `pdf:Producer`, `xmp:CreateDate`, `xmp:ModifyDate`, `xmp:CreatorTool`) uncompressed `/Metadata` stream; `lang` → `/Lang`; viewer preferences (`DisplayDocTitle`, page mode, page layout).
- Created pages and outline items get `LaminaId`s through the overlay allocator.

**Out of scope** (and which todo covers it)

- Structure tree, `tag()`, artifacts, `Link` structure elements with OBJR, alt text: [B8a](./B8a-structure-tree.md).
- PDF/A identification XMP schemas and output intents: [B8b](./B8b-conformance-profiles-and-verapdf.md).
- Editing Info/XMP on existing documents in sync: full-editor E1.
- Font fallback across fonts within one run: later.

## Tasks

- [ ] Load `/rust-best-practices`; read 03 §2, §5.3, §11.2 and 04 §1, §5
- [ ] `surface/text.rs` (runs), `surface/textbox.rs`, content emission with `pdf_writer::Content`
- [ ] `annot/link.rs`, `nav/{dest,outline,labels}.rs`, `meta/{info,xmp,prefs}.rs`
- [ ] PDF text-string encoding shared with `lamina-ops` `InfoPatch` (move to a common place if duplicated)
- [ ] XMP writer (deterministic attribute order, escaped values); tests that Info and XMP agree
- [ ] Goldens: `text-latin.pdf`, `text-multiscript.pdf`, `links-and-outlines.pdf`, `metadata.pdf`
- [ ] Full-lane checks: pdf.js link/outline extraction equals the builder's input; pypdf/pdftotext text equals input

## Acceptance criteria

- [ ] Text in five scripts (Latin, Arabic, Devanagari, Hebrew, CJK) generated via `text()` and `text_box()` extracts equal to the input in pdf.js, pypdf and pdftotext
- [ ] A text box never produces a line wider than its box; overflow is reported or clipped per option
- [ ] URI and internal links, named destinations and outlines resolve correctly in pdf.js (`getOutline`, `getDestination`, annotations) and in `lamina-model` views (round-trip through B2c's views)
- [ ] Page labels read back through `lamina-model::PageLabels` and pypdf `page_labels`
- [ ] Info and XMP carry the same values; `qpdf --check` clean; byte-identical across runs
- [ ] No ambient time: building twice with the same options gives identical bytes

## Post-merge checks

None.

## Tests

- Unit tests per module; `crates/lamina-build/tests/{text,textbox,links,outline,labels,metadata}.rs`.
- Goldens under `fixtures/golden/`; external checks in the full lane ([X4](../done/X4-full-lane-gate.md)).

## Docs to update

- [ ] [pdf-library §10 B5](../../plans/pdf-library.md#10-to-dos): tick text/text box part of "Surface ops" and "Document ops"
- [ ] Crate README; `fixtures/README.md`

## Risks and open questions

- Link rectangles for rotated text: use the run's transformed bounding box; `QuadPoints` for rotated links later.
- XMP vs Info precedence on read is an editor concern (E1); here they are written identically.
- `text()` options grow quickly; keep a builder-style API with `#[non_exhaustive]` option structs.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
