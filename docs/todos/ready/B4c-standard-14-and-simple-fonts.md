# B4c — Standard 14 fonts with AFM metrics and simple-font encoding

| Field | Value |
| --- | --- |
| ID | `B4c` |
| Status | `ready` |
| Plan | [pdf-library §7 B4](../../plans/pdf-library.md#b4-fonts-and-text), requirement [§4 F2](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B4a](./B4a-font-loading-and-subsetting.md) |
| Blocks | [B5b](./B5b-builder-text-and-document-features.md), [B8b](./B8b-conformance-profiles-and-verapdf.md), [B9b](./B9b-text-extraction-core.md), [E4e](./E4e-freetext-callouts.md), [E6c](./E6c-add-text.md), [E13c](./E13c-ocr-text-layer.md), [E16c](./E16c-images-and-fonts.md) |
| ADRs | Follows [0001](../../decisions/0001-core-language-and-licence.md) (licences of bundled metrics, determinism), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `feat/b4c-standard-14` |
| PR title | `feat(text): standard 14 fonts with bundled AFM metrics and simple-font encodings` |
| Size | M (≈ 400–600 lines of code + generated metric tables) |
| PR | — |

## Goal

Let callers write text without supplying font bytes, using the 14 standard Type 1 fonts by name with correct metrics, and provide the encoding tables (StandardEncoding, WinAnsiEncoding, MacRomanEncoding, PDFDocEncoding, Symbol, ZapfDingbats) and the Adobe Glyph List that both generation and extraction ([B9b](./B9b-text-extraction-core.md)) need. Mark these fonts as non-embedded so the PDF/A and PDF/UA profiles in [B8b](./B8b-conformance-profiles-and-verapdf.md) can reject them.

## Context

- [Research 03 §3.1](../../research/pdf/03-text-and-fonts.md#31-type-1-and-the-standard-14-fonts), [§3.8 `/Widths`](../../research/pdf/03-text-and-fonts.md#38-widths-simple-fonts), [§4.1 simple-font encodings](../../research/pdf/03-text-and-fonts.md#41-simple-font-encodings); ISO 32000-2 §9.6.2.2 and Annex D (encodings).
- Adobe Core 14 AFM files (licence permits redistribution of the metrics; verify and record) and the Adobe Glyph List (BSD-3-Clause).

## Scope

**In scope**

- `Standard14` enum (Helvetica, -Bold, -Oblique, -BoldOblique, Times-Roman, -Bold, -Italic, -BoldItalic, Courier ×4, Symbol, ZapfDingbats); `Font::standard(Standard14)`.
- Generated Rust tables from the AFM files (widths per glyph name, kerning pairs optional, ascent/descent/cap height/bbox) via a generator in `tools/` (not a build script), output committed under `crates/lamina-text/src/std14/generated/` with a header naming the source and licence.
- Encoding tables (code → glyph name) and AGL (glyph name → Unicode) as generated tables shared with extraction; PDFDocEncoding is not generated here but taken from `lamina_syntax::text` ([B2c](./B2c-lamina-model-corpus-fuzz.md)).
- Simple-font writer: `/Type /Font /Subtype /Type1 /BaseFont … /Encoding /WinAnsiEncoding` (Symbol/ZapfDingbats use built-in), `/FirstChar`, `/LastChar`, `/Widths` (PDF 2.0 requires widths for standard 14 too); text encoding to WinAnsi codes, unencodable characters → `missing-glyph`.
- `Font::requires_embedding() == false` flag consulted by profiles later; `Font::is_standard14()`.

**Out of scope** (and which todo covers it)

- Simple TrueType embedding with a custom `/Differences` encoding: later (Type 0 covers embedding now).
- Profile enforcement: [B8b](./B8b-conformance-profiles-and-verapdf.md).
- Reading arbitrary simple fonts from files: [B9b](./B9b-text-extraction-core.md).

## Tasks

- [ ] Load `/rust-best-practices`; read 03 §3.1, §4.1, Annex D
- [ ] Fetch the AFM files and AGL into `tools/std14-gen/data/` with SHA-256 and licence notes; write the generator (`cargo run -p std14-gen` or an xtask subcommand `cargo xtask gen std14`)
- [ ] Commit generated tables; add a test that regenerating produces no diff (like goldens)
- [ ] `std14/mod.rs` (`Standard14`, metrics lookup), `encoding/{standard,winansi,macroman,symbol,zapf}.rs`, `agl.rs`; `encoding/pdfdoc.rs` only adapts the PDFDocEncoding table in `lamina_syntax::text` (added by [B2c](./B2c-lamina-model-corpus-fuzz.md)) to code → glyph name via the AGL, so the workspace keeps one PDFDocEncoding table
- [ ] Simple-font dictionary writer and text encoding
- [ ] Tests below

## Acceptance criteria

- [ ] Widths for every glyph of every standard 14 font match the AFM source (generated test)
- [ ] A generated page using each standard 14 font passes `qpdf --check` and extracts the input text with pypdf and pdftotext (WinAnsi-representable strings)
- [ ] Unencodable characters (e.g. "é" U+00E9 in Symbol, whose built-in encoding has no `eacute`; CJK in Helvetica) report `missing-glyph`; "€" encodes in Helvetica (WinAnsi 0x80) and in Symbol (`Euro`, code 160 in Symbol.afm)
- [ ] `Font::standard(..).requires_embedding()` is `false`; for `Font::from_bytes` it is `true`
- [ ] Regenerating the tables is byte-identical

## Post-merge checks

None.

## Tests

- Unit tests for encodings, per encoding (WinAnsi, Standard, Symbol built-in, ZapfDingbats built-in): every **defined** code maps to a glyph name and the name maps back to that code (for names with one code) or to the canonical one; duplicate codes map to the same glyph (WinAnsi 040/240 → `space`, 055/255 → `hyphen`; the canonical code for encoding is the lower one); every undefined code has no glyph name and nothing encodes to it; name ↔ Unicode via the AGL for every defined name.
- `crates/lamina-text/tests/std14.rs`; golden `fixtures/golden/std14.pdf`.

## Docs to update

- [ ] [pdf-library §10 B4](../../plans/pdf-library.md#10-to-dos): tick "Bundle standard 14 AFM metrics" (rejection under profiles stays for B8b)
- [ ] `fixtures/README.md` / crate README (data provenance and licences)

## Risks and open questions

- AFM licence wording: confirm redistribution terms before committing; if unclear, use metric-compatible open data (e.g. from URW base35 metrics under a permissive licence, if available) — owner decides if neither is clean.
- Generated tables add WASM size (~30–50 KB raw); measure; they belong in the text module.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
