# B4d — One CID per (glyph, text): exact extraction of shaped text in every script

| Field | Value |
| --- | --- |
| ID | `B4d` |
| Status | `ready` |
| Plan | [pdf-library §7 B4](../../plans/pdf-library.md#b4-fonts-and-text), requirement [§4 F3](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B4b](../done/B4b-shaping-bidi-line-breaking.md) |
| Blocks | None |
| ADRs | Follows [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges), [0001](../../decisions/0001-core-language-and-licence.md) (determinism) |
| Branch | `feat/b4d-per-text-cids` |
| PR title | `feat(text): one CID per glyph and text, so shaped text extracts exactly` |
| Size | M (≈ 400–700 lines) |
| PR | — |

## Goal

Make the text drawn from shaped runs extract as the input in pypdf, pdf.js and pdftotext for all five scripts: the part of B4b's acceptance criterion it left unmet (its `fixtures/text/extraction-exceptions.json`).

## Context

- B4b's extraction facts: Latin, CJK, the Arabic line and the mixed bidi line extract exactly with pypdf, Latin and CJK with pdf.js. The rest fails because a ToUnicode CMap maps one code to one text (ISO 32000-2 §9.10.3) while shaping reuses glyphs with different texts (a pre-base matra, Noto Sans Arabic's separate dots: `<cid> <>`), and neither pypdf nor pdf.js `getTextContent` honours marked-content `/ActualText` (research [03 §5.3](../../research/pdf/03-text-and-fonts.md#53-ligatures-reordering-and-actualtext)).
- B4a's embedding draws a glyph with one CID (its subset glyph ID, `/CIDToGIDMap /Identity`).

## Scope

**In scope**

- `FontUse`: CIDs keyed by (glyph, text), so a glyph used with two texts gets two CIDs, both mapped to the same subset glyph; `/CIDToGIDMap` as a stream when any CID differs from its glyph ID (`CIDFontType2`); for CFF (`CIDFontType0`) the subset's charset maps CIDs to glyphs: decide (duplicate charset entries, or keep `/ActualText` for CFF) and record.
- `pdf_run`: assign every glyph of a reordered cluster the text extractors need in drawing order (or a separate CID with the cluster's text and empty ones for the rest), so pypdf and pdf.js read the logical text without `/ActualText`; drop the empty `<cid> <>` entries where a separate CID makes them unnecessary.
- Right-to-left lines: check whether drawing a right-to-left run as one `TJ` (marks positioned with numbers only, no splits) fixes pypdf's and pdf.js's piecewise reversal of pointed Hebrew.
- pdftotext facts next to pypdf and pdf.js in `tools/fixtures/text_facts.py` (poppler pinned in `doctor.rs`).

**Out of scope**

- Text extraction in lamina itself: [B9b](./B9b-text-extraction-core.md).

## Tasks

- [ ] `FontUse` with (glyph, text) CIDs and `/CIDToGIDMap` streams; byte-deterministic; B4a goldens unchanged when no glyph has two texts
- [ ] `pdf_run` text assignment without relying on `/ActualText`
- [ ] pdftotext facts; regenerate `fixtures/text/extracted.json`
- [ ] Empty `fixtures/text/extraction-exceptions.json` (or each remaining entry explained as an extractor bug with an upstream link)

## Acceptance criteria

- [ ] pypdf, pdf.js and pdftotext extract `text-shaped.pdf`'s lines exactly in all five scripts
- [ ] `qpdf --check` passes; output stays byte-deterministic

## Post-merge checks

None.

## Tests

- `crates/lamina-text/tests/extraction.rs` with the new facts; unit tests for (glyph, text) CIDs and `/CIDToGIDMap`.

## Docs to update

- [ ] [pdf-library §10 B4](../../plans/pdf-library.md#10-to-dos): the extraction to-do
- [ ] Crate README, `fixtures/README.md`

## Risks and open questions

- CFF CIDFonts address glyphs through the CFF charset, not `/CIDToGIDMap`: duplicating CIDs may need a rebuilt charset.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
