# T4f — Builder groundwork for flowing pages: elements across pages, measuring, artifact subtypes, marks under PDF/UA-1

| Field | Value |
| --- | --- |
| ID | `T4f` |
| Status | `ready` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4a](../ongoing/T4a-adr-0032-layout-engine.md) |
| ADRs | Implements [ADR 0032](../../decisions/0032-flowing-layout-and-document-inputs.md) [§4](../../decisions/0032-flowing-layout-and-document-inputs.md#4-tagging) (tagging) and [§10](../../decisions/0032-flowing-layout-and-document-inputs.md#10-what-lands-when) (the groundwork); follows [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space) and [0026](../../decisions/0026-error-and-diagnostic-contract.md) (codes) |
| Branch | `feat/t4f-flow-builder-groundwork` |
| PR title | `feat(build): tagged elements across pages, text measured without drawing, artifact subtypes and marks under PDF/UA-1` |
| Size | L (≈ 500–700 lines of code with unit tests) |
| PR | — |

## Goal

Give `laminapdf-build` the four things the [`flow-layout` spike](../../../spikes/flow-layout/README.md) found missing when it paginated a 100-page report over today's `Surface`. Each one is useful on its own, and the flow layer ([T4b](./T4b-flow-layout-core.md)) needs all four:

1. **A tagged element continues on the next page.** The spike's paginator made two `P` elements of each of 37 split paragraphs, and 23 `Table` elements of 10 tables, where Typst kept each one whole.
2. **Text is measured without being drawn.** Keep rules had to assume a line height, and table columns couldn't be sized to their content, so a cell too narrow clipped silently.
3. **Pagination artifacts carry a `/Subtype`.** Typst writes `/Subtype /Header` or `/Footer` with `/Attached`; the builder writes a bare `/Pagination`.
4. **PDF/UA-1 accepts Arabic with diacritics.** The builder refused the spike's Arabic sentence with `glyph-without-unicode` on a mark that shares a cluster with its letter, while Typst wrote it and veraPDF passed it.

## Context

- The spike's README ("What our own engine lacked") and [ADR 0032 §4](../../decisions/0032-flowing-layout-and-document-inputs.md#4-tagging), which requires elements across pages, artifact subtypes and a Unicode value for every glyph in flowing output.
- Tagging: `crates/laminapdf-build/src/tag/elem.rs` collects elements page by page and numbers them when their page is written; `Surface::tag` documents "Elements are per page: one doesn't continue on the next page". `src/tag/parent_tree.rs` writes a one-leaf `ParentTree`; `src/tag/artifact.rs` writes `/Artifact << /Type /Pagination >>`.
- Text: `Surface::text_box` (`src/surface/mod.rs`, `surface/textbox.rs`) shapes and draws in one call; the crate-private `lay_out` and `Spec` in `surface/text.rs` shape a paragraph once. `TextLayout` gives line boxes, `bottom()` and `overflow`.
- Unicode for shaped glyphs: `laminapdf-text`'s `pdf_run` and the font registry (B4d gives each (glyph, text) its own CID, and a textless glyph a format-character CID mapped to `<>`); `src/profile/checks.rs` raises `glyph-without-unicode` under PDF/UA-1 and PDF/A-2u.
- ISO 32000-2 §14.7.5.2 (marked-content references: an `/MCR` dictionary with its own `/Pg`), §14.7.5.4 (the `ParentTree`), §14.8.2.2 (artifacts: `/Type /Pagination` with `/Subtype /Header`, `/Footer` or `/Watermark`, and `/Attached`) and §9.10.3 (ToUnicode: a code may map to several characters). Research 03 [§9.5](../../research/pdf/03-text-and-fonts.md#95-generating-tagged-pdf-from-code).
- The spike's Arabic sentence is `ARABIC` in `spikes/flow-layout/src/inputs.rs`; its fonts are `fixtures/fonts/NotoSansArabic-Regular.ttf` (OFL 1.1).

## Scope

**In scope**

- **Elements across pages:** a public way to open an element on one page and add content to it on later pages, then close it (for example `Surface::open_element(tag) -> ElementId`, `Surface::resume(&id, |s| …)` and `Builder::close_element(id)`; names settle in review). Content on later pages joins its `/K` as `/MCR` dictionaries with their own `/Pg`, each page's `ParentTree` entries point at the element, and the element's object is written when it closes. Elements opened and closed on one page write exactly what they write today.
- **Measuring without drawing:** `Surface::measure_text_box` (or `TextBoxOptions` on a new `Builder::measure`; names settle in review) returns the `TextLayout` that `text_box` would draw, without drawing it. A crate-private command draws a measured layout without shaping it again, for T4b's emitter.
- **Artifact subtypes:** `ArtifactKind::Pagination` takes a subtype (header, footer, watermark) and the edges it is attached to (`/Attached [/Top]`). The bare form stays.
- **Marks under PDF/UA-1:** every glyph a shaped run draws gets a Unicode value, a mark that shares a cluster with its letter included (for example: the cluster's text on its first glyph and the combining character on each mark, or `/ActualText` around the cluster; pick what veraPDF and pdf.js both accept). Profiles keep raising `glyph-without-unicode` for a glyph that truly has none.
- **The bindings:** the subtypes in `laminapdf-wasm`'s name tables and `cargo xtask gen boundary`. Elements across pages and measuring stay Rust only until T4b's surfaces need them.

**Out of scope** (and which todo covers it)

- The flow layer that uses all four: [T4b](./T4b-flow-layout-core.md).
- Styled spans in one paragraph: [T4g](./T4g-flow-styled-spans.md).
- A glyph-run API on `Surface` (drawing shaped glyphs as they come): not planned; ADR 0032 rejected the hybrid that needed it.
- Tagging pages appended to an open document (`Builder::on`): later.

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`
- [ ] Read ADR 0032 §4, the spike's README and `src/tag/`, and the ISO clauses in Context
- [ ] `src/tag/`: elements that outlive their page (an open-element table on the `Builder`, MCR kids, `ParentTree` entries per page), with unit tests for a paragraph across three pages and for an element never closed (`Builder::finish` fails with a code: `tag-element-unclosed`, new, in `buildErrors`)
- [ ] `src/surface/`: `measure_text_box` beside `text_box`, sharing `lay_out`; the crate-private command that draws a measured layout
- [ ] `src/tag/artifact.rs`: subtypes and `/Attached`
- [ ] `laminapdf-text` and the registry: a Unicode value for every glyph of a cluster; a test with the spike's Arabic sentence under `Profile::PdfUa1`
- [ ] The code `tag-element-unclosed`: its `codes.json` entry, its `docs/errors/` page and `cargo xtask gen codes`
- [ ] The bindings' name tables and `cargo xtask gen boundary`
- [ ] The docs (Docs to update)

## Acceptance criteria

- [ ] A paragraph opened on page 1 and resumed on pages 2 and 3 is one `P` element whose content is on all three pages (`laminapdf_model::Document::structure()`), and the file passes `cargo xtask verapdf` under PDF/UA-1
- [ ] `measure_text_box` returns the same `TextLayout` as `text_box` for the inputs of `crates/laminapdf-build/tests/`, and the page's content stream doesn't change
- [ ] A footer drawn as a pagination artifact with the footer subtype writes `/Artifact << /Type /Pagination /Subtype /Footer /Attached [/Bottom] >>`, and `laminapdf-extract` drops it as before
- [ ] The spike's Arabic sentence, in a text box under PDF/UA-1, builds, passes veraPDF, and pdf.js and pdftotext read it back as the input (whitespace and bidi controls removed)
- [ ] Every byte golden in `fixtures/golden/` that uses none of the new calls is unchanged, or the PR explains each change
- [ ] `cargo xtask gen codes --check` and `cargo xtask gen boundary --check` pass

## Post-merge checks

None.

## Tests

- Unit tests in `src/tag/` (MCR kids, `ParentTree` entries, an unclosed element), `surface/` (measuring equals drawing) and `artifact.rs` (each subtype).
- `crates/laminapdf-build/tests/tagging.rs`: the three-page paragraph read back through `laminapdf-model`.
- A text test with the Arabic sentence under PDF/UA-1, and its extraction facts in `fixtures/build/facts.json` (`tools/fixtures/build_facts.py`).
- `cargo xtask verapdf` over a new build golden, `fixtures/golden/tagged-across-pages.pdf`.

## Docs to update

- [ ] `laminapdf-build`'s README and `lib.rs` docs (elements across pages, measuring, artifact subtypes)
- [ ] `CLAUDE.md`'s Building API paragraph (tagged PDF: elements are no longer per page only)
- [ ] `fixtures/README.md`: the new golden's row

## Risks and open questions

- **Numbering changes.** Writing an element when it closes, not when its page is written, can renumber objects in existing goldens. Keep the old order for elements that open and close on one page.
- **Which Unicode form readers accept.** pdf.js and pdftotext treat `/ActualText` differently: the spike's pdftotext read none of the three Arabic paragraphs back. Test both readers before choosing.
- **An open element across a `Builder::page` boundary** must survive `PageBuilder::finish`. Check that `q`/`Q` and the MCID counter per page stay right.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
