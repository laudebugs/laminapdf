# B4d — One CID per (glyph, text): exact extraction of shaped text in every script

| Field | Value |
| --- | --- |
| ID | `B4d` |
| Status | `done` |
| Plan | [pdf-library §7 B4](../../plans/pdf-library.md#b4-fonts-and-text), requirement [§4 F3](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B4b](./B4b-shaping-bidi-line-breaking.md) |
| ADRs | Follows [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges), [0001](../../decisions/0001-core-language-and-licence.md) (determinism) |
| Branch | `feat/b4d-per-text-cids` |
| PR title | `feat(text): one CID per glyph and text, so shaped text extracts exactly` |
| Size | M (≈ 400–700 lines) |
| PR | [#62](https://github.com/laudebugs/lamina/pull/62) |

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

- Text extraction in lamina itself: [B9b](../done/B9b-text-extraction-core.md).

## Tasks

- [x] `FontUse` with (glyph, text) CIDs and `/CIDToGIDMap` streams; byte-deterministic; B4a goldens unchanged when no glyph has two texts (only `text-shaped.pdf` changed)
- [x] `pdf_run` text assignment without relying on `/ActualText` (TrueType outlines; CFF keeps B4b's mapping, see Outcome)
- [x] pdftotext facts (poppler 26.10.0, pinned in `doctor.rs`); regenerate `fixtures/text/extracted.json`
- [x] Empty `fixtures/text/extraction-exceptions.json` (or each remaining entry explained as an extractor bug with an upstream link): seven entries remain, each with its reason and an `upstream` link to the extractor's code

## Acceptance criteria

- [ ] pypdf, pdf.js and pdftotext extract `text-shaped.pdf`'s lines exactly in all five scripts: **partly** (see Outcome). pypdf: every line but the pointed-Hebrew line; pdf.js: every line but the pointed-Hebrew and Devanagari lines; pdftotext: Latin, Devanagari and CJK, not its right-to-left lines. Each remaining line is an extractor bug, listed with an upstream link
- [x] `qpdf --check` passes; output stays byte-deterministic

## Post-merge checks

None.

## Tests

- `crates/lamina-text/tests/extraction.rs` with the new facts; unit tests for (glyph, text) CIDs and `/CIDToGIDMap`.

## Docs to update

- [x] [pdf-library §10 B4](../../plans/pdf-library.md#10-to-dos): the extraction to-do
- [x] Crate README, `fixtures/README.md` (and `CLAUDE.md`'s Fonts paragraph, `lib.rs`)

## Risks and open questions

- CFF CIDFonts address glyphs through the CFF charset, not `/CIDToGIDMap`: duplicating CIDs may need a rebuilt charset.

## Outcome

- **CIDs** (`FontUse`): one CID per (glyph, text). In a subset, CIDs are dense in order of first (glyph, text) use; with no glyph drawn for two texts they equal the subset's glyph IDs, so `/CIDToGIDMap /Identity` and every B4a golden stay byte-identical. Otherwise `/CIDToGIDMap` is a Flate stream. A font embedded whole keeps CID = GID for a glyph's first text and puts further CIDs past its glyph count.
- **Textless glyphs** (Noto Sans Arabic's separate dots, the space glyph a hidden ZWJ is drawn with): a CID among the code points of format characters (U+200B–200F, U+202A–202E, U+2060–2064, U+2066–206F, U+FEFF, U+FFF9–FFFB; 29 slots, ordinary CIDs skip them), mapped to `<>`. pypdf and pdftotext read the empty destination as nothing; pdf.js reads it as the code, and skips a code that is a format character (`category.isInvisibleFormatMark`). After the slots (and in a subset of a font with over 65 507 glyphs, which gets none), a textless glyph gets an ordinary CID and `pdf_run` covers its cluster with `/ActualText`, as it does a second text once the CIDs run out. CFF and the program's kind are decided by one predicate (`subset::has_cff_table`).
- **CFF decision**: `CIDFontType0` addresses glyphs through the CFF charset (one CID per charstring), so a CFF font keeps one CID per glyph and B4b's mapping: the first text, `tounicode-conflict` and `/ActualText`. Duplicating CIDs would need duplicated charstrings in the subset; not worth it while no fixture needs it.
- **`pdf_run`**: with TrueType outlines the cluster's characters are dealt out in reading order (drawing order, reversed for RTL), each glyph keeping its share's length, so a pre-base matra, a reph or reordered marks read back from the codes. `/ActualText` remains only for CFF fonts, runs whose clusters are out of visual order, and clusters holding default-ignorables.
- **Right-to-left lines**: drawing a run as one `TJ` (no `/ActualText` pieces) fixed pointed Hebrew within a run, but the line's runs (Hebrew words around a comma from Noto Sans) were still read in visual order, because pypdf flushes its right-to-left reversal at every `Tf` and pdf.js keeps the content order of its text items. New `Line::drawing_order()` gives the runs in logical order; the test page draws them that way (B5b should too).
- **Remaining exceptions** (each an extractor bug, with an `upstream` link in `fixtures/text/extraction-exceptions.json`): pypdf appends a space for a large `TJ` number (the Hebrew points' offsets) to text it builds by prepending (pointed Hebrew); pdf.js loses the space at a run boundary drawn right to left (pointed Hebrew) and treats any glyph text containing a nonspacing mark as zero-width (an unanchored regex), which moves a space after the `दु` ligature (Devanagari); pdftotext writes right-to-left text in display order inside U+202B…U+202C embeddings (lines 3, 4, 7) and drops the ZapfDingbats line's spaces in `text-std14.pdf`.
- **pdftotext**: poppler 26.10.0 (`POPPLER_VERSION` in `doctor.rs`, an optional tool; `doctor` now reads a version printed on stderr), default reading-order mode; `extraction.rs` checks the facts' version against the pin.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
