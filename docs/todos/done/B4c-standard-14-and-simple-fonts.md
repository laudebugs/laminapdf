# B4c — Standard 14 fonts with AFM metrics and simple-font encoding

| Field | Value |
| --- | --- |
| ID | `B4c` |
| Status | `done` |
| Plan | [pdf-library §7 B4](../../plans/pdf-library.md#b4-fonts-and-text), requirement [§4 F2](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B4a](./B4a-font-loading-and-subsetting.md) |
| Blocks | [B5b](../ready/B5b-builder-text-and-document-features.md), [B8b](../ready/B8b-conformance-profiles-and-verapdf.md), [B9b](../ready/B9b-text-extraction-core.md), [E4e](../ready/E4e-freetext-callouts.md), [E6c](../ready/E6c-add-text.md), [E13c](../ready/E13c-ocr-text-layer.md), [E16c](../ready/E16c-images-and-fonts.md) |
| ADRs | Follows [0001](../../decisions/0001-core-language-and-licence.md) (licences of bundled metrics, determinism), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `feat/b4c-standard-14` |
| PR title | `feat(text): standard 14 fonts with bundled AFM metrics and simple-font encodings` |
| Size | M (≈ 400–600 lines of code + generated metric tables) |
| PR | [#54](https://github.com/laudebugs/lamina/pull/54) |

## Goal

Let callers write text without supplying font bytes, using the 14 standard Type 1 fonts by name with correct metrics, and provide the encoding tables (StandardEncoding, WinAnsiEncoding, MacRomanEncoding, PDFDocEncoding, Symbol, ZapfDingbats) and the Adobe Glyph List that both generation and extraction ([B9b](../ready/B9b-text-extraction-core.md)) need. Mark these fonts as non-embedded so the PDF/A and PDF/UA profiles in [B8b](../ready/B8b-conformance-profiles-and-verapdf.md) can reject them.

## Context

- [Research 03 §3.1](../../research/pdf/03-text-and-fonts.md#31-type-1-and-the-standard-14-fonts), [§3.8 `/Widths`](../../research/pdf/03-text-and-fonts.md#38-widths-simple-fonts), [§4.1 simple-font encodings](../../research/pdf/03-text-and-fonts.md#41-simple-font-encodings); ISO 32000-2 §9.6.2.2 and Annex D (encodings).
- Adobe Core 14 AFM files (licence permits redistribution of the metrics; verify and record) and the Adobe Glyph List (BSD-3-Clause).

## Scope

**In scope**

- `Standard14` enum (Helvetica, -Bold, -Oblique, -BoldOblique, Times-Roman, -Bold, -Italic, -BoldItalic, Courier ×4, Symbol, ZapfDingbats); `Font::standard(Standard14)`.
- Generated Rust tables from the AFM files (widths per glyph name, kerning pairs optional, ascent/descent/cap height/bbox) via a generator in `tools/` (not a build script), output committed under `crates/lamina-text/src/std14/generated/` with a header naming the source and licence.
- Encoding tables (code → glyph name) and AGL (glyph name → Unicode) as generated tables shared with extraction; PDFDocEncoding is not generated here but taken from `lamina_syntax::text` ([B2c](../done/B2c-lamina-model-corpus-fuzz.md)).
- Simple-font writer: `/Type /Font /Subtype /Type1 /BaseFont … /Encoding /WinAnsiEncoding` (Symbol/ZapfDingbats use built-in), `/FirstChar`, `/LastChar`, `/Widths` (PDF 2.0 requires widths for standard 14 too); text encoding to WinAnsi codes, unencodable characters → `missing-glyph`.
- `Font::requires_embedding() == false` flag consulted by profiles later; `Font::is_standard14()`.

**Out of scope** (and which todo covers it)

- Simple TrueType embedding with a custom `/Differences` encoding: later (Type 0 covers embedding now).
- Profile enforcement: [B8b](../ready/B8b-conformance-profiles-and-verapdf.md).
- Reading arbitrary simple fonts from files: [B9b](../ready/B9b-text-extraction-core.md).

## Tasks

- [x] Load `/rust-best-practices`; read 03 §3.1, §4.1, Annex D
- [x] Fetch the AFM files and AGL into `tools/std14-gen/data/` with SHA-256 and licence notes; write the generator (`cargo run -p std14-gen`, a workspace tool crate; `-- --check` verifies)
- [x] Commit generated tables; add a test that regenerating produces no diff (like goldens): `committed_tables_are_current` in `tools/std14-gen` (compares, never writes)
- [x] `std14/mod.rs` (`Standard14`, metrics lookup), `encoding/mod.rs`, `agl.rs`; `encoding/pdfdoc.rs` only adapts the PDFDocEncoding table in `lamina_syntax::text` (added by [B2c](../done/B2c-lamina-model-corpus-fuzz.md)) to code → glyph name via the AGL, so the workspace keeps one PDFDocEncoding table. *Deviation:* one `Encoding` enum over generated tables (`std14/generated/encodings.rs`) instead of a file per encoding
- [x] Simple-font dictionary writer and text encoding (`simple/mod.rs`), plus `/Differences` for Latin glyphs outside WinAnsi (see Outcome)
- [x] Tests below

## Acceptance criteria

- [x] Widths for every glyph of every standard 14 font match the AFM source (`tests/std14.rs` parses the committed AFM files)
- [x] A generated page using each standard 14 font passes `qpdf --check` and extracts the input text with pypdf (WinAnsi-representable strings, plus Symbol, ZapfDingbats and a `/Differences` line). *Deviation:* not pdftotext: Poppler isn't installed on the dev machine or pinned in `doctor.rs`; the facts stay pypdf + qpdf like B4a's
- [x] Unencodable characters (e.g. "é" U+00E9 in Symbol, whose built-in encoding has no `eacute`; CJK in Helvetica) report `missing-glyph`; "€" encodes in Helvetica (WinAnsi 0x80) and in Symbol (`Euro`, code 160 in Symbol.afm)
- [x] `Font::standard(..).requires_embedding()` is `false`; for `Font::from_bytes` it is `true`
- [x] Regenerating the tables is byte-identical

## Post-merge checks

None.

## Tests

- Unit tests for encodings, per encoding (WinAnsi, Standard, Symbol built-in, ZapfDingbats built-in): every **defined** code maps to a glyph name and the name maps back to that code (for names with one code) or to the canonical one; duplicate codes map to the same glyph (WinAnsi 040/240 → `space`, 055/255 → `hyphen`; the canonical code for encoding is the lower one); every undefined code has no glyph name and nothing encodes to it; name ↔ Unicode via the AGL for every defined name.
- `crates/lamina-text/tests/std14.rs`; golden `fixtures/golden/text-std14.pdf` (named `text-*` so `text_facts.py` and the extraction tests cover it).

## Docs to update

- [x] [pdf-library §10 B4](../../plans/pdf-library.md#10-to-dos): tick "Bundle standard 14 AFM metrics" (rejection under profiles stays for B8b)
- [x] `fixtures/README.md` / crate README (data provenance and licences); [`tools/std14-gen/data/README.md`](../../../tools/std14-gen/data/README.md) holds the sources, SHA-256 and licence terms

## Risks and open questions

- AFM licence wording: confirm redistribution terms before committing; if unclear, use metric-compatible open data (e.g. from URW base35 metrics under a permissive licence, if available) — owner decides if neither is clean.
- Generated tables add WASM size (~30–50 KB raw); measure; they belong in the text module.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome

- **Data** (`tools/std14-gen/data/`, provenance in its README): Adobe's `Core14_AFMs.zip` (the 14 AFM files + `MustRead.html`, unmodified, CRLF kept), fetched from the Internet Archive's copy of Adobe's former URL; its terms allow use, copying and distribution with or without modification provided the copyright notices are kept, the AFMs travel with `MustRead.html` and modifications are noted, which the generated `metrics.rs` does. The Adobe Glyph List and ITC Zapf Dingbats Glyph List (BSD-3-Clause) at a pinned commit. `latin-encodings.txt`: Annex D Table D.2's STD/MAC/WIN columns, written for lamina.
- **Generated** (`crates/lamina-text/src/std14/generated/`, about 200 KB of source, most of it the 4,281-entry AGL): metrics (one sorted glyph-name table for the 12 Latin fonts, which share the 315-glyph set, widths per font), five encoding tables, Unicode → glyph tables, the two glyph lists. Kerning pairs are not bundled (optional in the scope).
- **Beyond the scope**: Latin glyphs outside WinAnsi (`Ł`, `ş`, `−`, `ﬁ`) are reachable through `/Differences` on the 37 codes WinAnsi leaves undefined, assigned in order of first use; past that, the new error `encoding-full` (`limit`). `<name>greek` aliases let `Δ Ω μ` reach Symbol's `Delta`/`Omega`/`mu` (the AGL maps those to U+2206, U+2126, U+00B5); U+00A0 and U+00AD map to `space` and `hyphen`.
- **Findings**: without a ToUnicode CMap pypdf 6.21 extracts code 32 of Symbol and ZapfDingbats as U+00A0; since the review every simple font gets a one-byte ToUnicode CMap, so the golden's symbolic lines keep their spaces.
- **Review (#54, 28 findings)**: `use_glyph` returns a `GlyphCode` (one byte for simple fonts); standard 14 glyph IDs are name index + 1 (0 is `.notdef`); ToUnicode and `tounicode-conflict` for simple fonts; `FontRegistry::usage` returns `FontUsage`; Symbol's Greek letters read back as Greek; Ț ț map to `Tcommaaccent`/`tcommaaccent`; `encoding-full` reports `free`; `MacExpertEncoding` is `UnsupportedEncoding::MacExpert`; `EmbeddedFont::embedded`; `Font::is_standard14` dropped for `standard14()`; generator hash-checks every input, rejects duplicate codes and emits reverse code tables.
