# B4a — `lamina-text`: font loading, subsetting and Type 0 embedding with ToUnicode

| Field | Value |
| --- | --- |
| ID | `B4a` |
| Status | `done` |
| Plan | [pdf-library §7 B4](../../plans/pdf-library.md#b4-fonts-and-text), requirement [§4 F2](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B1b](./B1b-krilla-fit-spike.md), [B3b](./B3b-lamina-save-writers.md), [X4](./X4-full-lane-gate.md) |
| ADRs | Follows [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (own builder; `skrifa`, `subsetter`; reserve-then-fill objects), [0001](../../decisions/0001-core-language-and-licence.md) (caller supplies bytes, determinism), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `feat/b4a-font-embedding` |
| PR title | `feat(text): font loading, subsetting and Type 0 embedding with /W and ToUnicode` |
| Size | L (≈ 900–1,300 lines) |
| PR | [#48](https://github.com/laudebugs/lamina/pull/48) |

## Goal

Embed fonts correctly from code: load TrueType/OpenType/CFF fonts (and TTC members, and variable-font instances) from caller-supplied bytes, track the glyphs a document uses, and at `finish` write a deterministic subset as a Type 0 / Identity-H font with `/W` widths and a ToUnicode CMap, so text is selectable and extractable. This is the font half of `lamina-text`; shaping is [B4b](./B4b-shaping-bidi-line-breaking.md), simple fonts and the standard 14 are [B4c](../done/B4c-standard-14-and-simple-fonts.md).

## Context

- [Research 03 §3.5–3.8](../../research/pdf/03-text-and-fonts.md#35-type-0-composite-fonts-and-cidfonts) (Type 0, CIDFontType0/2, descriptors, `/W`), [§4.3–4.4](../../research/pdf/03-text-and-fonts.md#43-cids-vs-gids-vs-unicode-code-points) (CID vs GID, `CIDToGIDMap`), [§5](../../research/pdf/03-text-and-fonts.md#5-tounicode-cmaps-and-actualtext-910) (ToUnicode syntax, ligatures → multi-char `bfchar`), [§6](../../research/pdf/03-text-and-fonts.md#6-font-embedding--subsetting-from-code) (sfnt tables, subsetting algorithm, [§6.3 fsType](../../research/pdf/03-text-and-fonts.md#63-licensing-os2fstype), [§6.4 libraries](../../research/pdf/03-text-and-fonts.md#64-libraries-state-as-of-late-2026)), [§7.9 variable fonts](../../research/pdf/03-text-and-fonts.md#79-variable-fonts), [§10 worked example](../../research/pdf/03-text-and-fonts.md#10-worked-verified-example), [§11.2 requirements](../../research/pdf/03-text-and-fonts.md#112-concrete-requirements).
- [krilla-fit experiment C](../../../spikes/krilla-fit/src/exp_c.rs): ≈ 400 lines doing exactly this with `skrifa` + `subsetter`; port with attribution, not copy-paste.
- ISO 32000-2 §9.6–9.10.

## Scope

**In scope**

- New core crate `crates/lamina-text` (deps: `skrifa`, `read-fonts`, `subsetter`; check licences and wasm32 build).
- `Font::from_bytes(bytes)` / `Font::from_collection(bytes, index)`; `FontInfo` (family, PostScript name, units per em, ascent/descent/cap height/x-height, italic angle, flags, bbox, `is_cff`, `is_variable`, axes); `Font::instance(&[(Tag, f32)])` for variable fonts (instanced at embed time via `subsetter`'s instancing if supported, else error `variable-instancing-unsupported` with a clear message — see risks).
- `fsType` handling: restricted licence → `Error::FontNotEmbeddable { fs_type }` unless `EmbedPolicy::Override` is set explicitly; bitmap-only / no-subsetting bits respected (embed whole font when subsetting is forbidden).
- Glyph usage tracking: `FontUse` per document collecting (GID → Unicode string) pairs; conflicting mappings (one GID, two strings) → keep the first, report `tounicode-conflict` and leave `ActualText` to the builder.
- Embedding at finish: reserve font objects on first use (overlay `New`), fill at `finish()`; subset with GID remapping preserved deterministically (old → new GID map), subset tag = 6 uppercase letters derived from a hash of (font bytes hash, sorted used GIDs); `FontFile2` (TrueType) or `FontFile3 /OpenType` (CFF), Flate-compressed; `CIDFontType2` with `/CIDToGIDMap /Identity` (after remapping) or `CIDFontType0`; `/W` compressed (runs of equal widths, consecutive arrays); ToUnicode CMap with `bfrange`/`bfchar` in chunks of ≤ 100 per block (§9.10.3), UTF-16BE incl. surrogates.
- Missing-glyph diagnostics: `Font::glyph_for(char)` returning `None` is surfaced as `TextError::MissingGlyph { ch, font }` (code `missing-glyph`), never `.notdef` silently.
- Test-support page writer (behind `#[cfg(test)]`/a test helper) that places glyph IDs with `pdf_writer::Content` so B4a can produce real PDFs before the builder exists.

**Out of scope** (and which todo covers it)

- Shaping, bidi, line breaking, OpenType features: [B4b](./B4b-shaping-bidi-line-breaking.md).
- Standard 14 fonts, simple TrueType/WinAnsi embedding: [B4c](../done/B4c-standard-14-and-simple-fonts.md).
- `surface.text()` API: [B5b](../done/B5b-builder-text-and-document-features.md).
- Reading fonts from existing PDFs for extraction: [B9b](../done/B9b-text-extraction-core.md).
- Colour fonts / emoji: later (03 §7.8), reported as unsupported.

## Tasks

- [x] Load `/rust-best-practices`; read 03 §3.5–6 and §10; read `spikes/krilla-fit/src/exp_c.rs`
- [x] Add `skrifa`, `subsetter`, `read-fonts` to workspace deps; `cargo xtask deny`; check wasm32 (`read-fonts` is used through `skrifa::raw`, see deviations)
- [x] Create `crates/lamina-text` (core), `font.rs` (`Font`, `FontInfo`, TTC), `variable.rs`, `fstype.rs` (instancing lives in `font.rs`, see deviations)
- [x] `usage.rs` (`FontUse`, GID → text map, deterministic ordering via `BTreeMap`)
- [x] `embed/type0.rs` (font dict, descendant CIDFont, descriptor, `/W` writer), `embed/tounicode.rs` (CMap writer), `embed/subset.rs` (subsetter call, tag derivation)
- [x] Integrate with the overlay: `FontRegistry` on a document-level builder context that reserves `ObjRef`s and fills them in `finish_fonts(&mut CosMut)`
- [x] Fixtures: OFL fonts under `fixtures/fonts/` (Noto Sans, Noto Sans Arabic, Noto Sans Devanagari, Noto Sans Hebrew, Noto Sans CJK subset or Noto Sans SC subset, a CFF-flavoured OTF, a variable font, a TTC) with licence files and SHA-256 in `fixtures/README.md`; keep total size small (pre-subset large CJK to a test range)
- [x] Tests below

## Acceptance criteria

- [x] Embedding the same text twice produces byte-identical fonts and subset tags
- [x] Generated test PDFs pass `qpdf --check` and pypdf `extract_text()` returns the input string (including `fi`/`ffi` ligature glyphs mapped via ToUnicode, accents, curly quotes, a non-BMP character)
- [x] `/W` widths equal the subset's `hmtx` advances scaled to 1/1000 em (test compares with skrifa)
- [x] ToUnicode CMap parses with pdf.js (round-trip extraction in the full lane) and has ≤ 100 entries per block (pdf.js checked by hand, not in the full lane: see deviations)
- [x] A font with `fsType` restricted licensing fails without override and embeds with it; a "no subsetting" font is embedded whole
- [x] A TTC member and a variable-font instance (wght=700) embed and extract
- [x] A missing glyph yields `missing-glyph` with the character and font name

## Post-merge checks

None.

## Tests

- Unit: `/W` compression, CMap writer (bfrange splitting, surrogates), subset tag derivation.
- Integration `crates/lamina-text/tests/{embed,tounicode,fstype,variable}.rs`: build PDFs with the test page writer, reopen with `lamina-cos`, check font dicts; goldens `fixtures/golden/text-*.pdf` via `cargo xtask golden --update`.
- External (full lane via `cargo xtask roundtrip` style script): pypdf and pdftotext extraction equals input.

## Docs to update

- [x] [pdf-library §10 B4](../../plans/pdf-library.md#10-to-dos): tick `Font::from_bytes` + TTC + variable instancing, `subsetter` + subset tags + `/W` + ToUnicode, missing-glyph diagnostics
- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): `lamina-text` (generation half) noted as done; the item stays open for B4b/B4c
- [x] `fixtures/README.md` (fonts, licences), `CLAUDE.md` layout, crate README

## Risks and open questions

- `subsetter`'s variable-font instancing support and CFF2 handling: verify on the pinned version; if missing, ship instancing as a follow-up todo and error clearly meanwhile.
- `skrifa` versions move fast (krilla pinned 0.42 while 0.49 is current); pin one and track with Dependabot.
- CJK fixture size: commit only a pre-subset test font; full fonts fetched in the nightly lane.
- WASM size: `skrifa` + `subsetter` belong in the text module ([B6a](../done/B6a-lamina-wasm-and-adr-0016.md)); keep `lamina-text` separable from base crates (no dependency from `lamina-build`'s non-text parts on it — feature-gate).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings): run by the orchestrating session after the PR opened
- [ ] CI green (see the PR)
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome and deviations

- **Crate.** `crates/lamina-text` (core, `#![forbid(unsafe_code)]`, builds for wasm32/iOS/Android): `font.rs` (`Font`, `FontInfo`, `Axis`, `Tag`, `GlyphId`; TTC via `Font::from_collection`, `Font::collection_len`; `Font::instance`), `fstype.rs` (`FsType`, `EmbedPolicy`), `usage.rs` (`FontUse`), `embed/{mod,subset,type0,tounicode}.rs` (`FontRegistry`, `FontKey`, `Encoded`, `EmbeddedFont`), `error.rs` (`Error`, `Warning`). Dependencies: `skrifa =0.42.0` (default features off, `std`), `subsetter =0.2.6` (its default `variable-fonts` feature: `write-fonts`, `kurbo`), `sha2` and `lamina-cos`; dev: `lamina-save`, `pdf-writer =0.15.0`, `serde_json`. All `MIT OR Apache-2.0` (transitively also `Zlib OR Apache-2.0 OR MIT` for `bytemuck`, `Apache-2.0 OR BSL-1.0` for `ryu`); `cargo xtask deny` passes, `deny.toml` unchanged.
- **`read-fonts` is not a direct dependency**: `skrifa` re-exports it as `skrifa::raw`, so one pinned version serves both. `skrifa` is pinned to 0.42.0, the version `subsetter` 0.2.6 uses, so the graph has a single copy (0.49 is current; moving needs a `subsetter` release).
- **`variable.rs` folded into `font.rs`**: `Font::instance` is ~30 lines next to the `FontInfo` it recomputes. Instancing is supported by `subsetter::subset_with_variations` (TrueType `gvar`/`HVAR`; CFF2 comes back as TrueType), so the `variable-instancing-unsupported` error the risks section anticipated is not needed; asking for an axis the font lacks is `unknown-axis`. An instance's `/BaseFont` gets `_wght700`-style suffixes. Advances are rounded to whole font units, as the instanced `hmtx` stores them.
- **One error type**, `lamina_text::Error` (the todo named both `Error::FontNotEmbeddable` and `TextError::MissingGlyph`), with codes `invalid-font`, `collection-index`, `unknown-axis`, `font-not-embeddable`, `missing-glyph`, `glyph-out-of-range`, `unknown-font`, `subset-failed` (plus `lamina-cos`'s through `Error::Cos`), and `Warning::ToUnicodeConflict` (`tounicode-conflict`), listed in `fixtures/codes.json` as `textErrors`/`textWarnings` and checked by `tests/codes.rs`. Following [ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md) (merged from `main` during this work): each code has a `docs/errors/<code>.md` page and a kind in `docs/errors/README.md` (`input`: `invalid-font`, `missing-glyph`; `usage`: `collection-index`, `unknown-axis`, `font-not-embeddable`, `glyph-out-of-range`, `unknown-font`, `tounicode-conflict`; `unsupported`: `subset-failed`), and `Display` messages carry only numbers and fixed text (font names, characters, axis tags and texts stay in the variant fields; tested in `tests/codes.rs` with a marker string, which also checks every code has a page). `kind()` in Rust waits for L1c's table, like the other crates.
- **CFF embedding** is `CIDFontType0` with `/FontFile3 /Subtype /CIDFontType0C` (the subset's bare CFF table, CID-keyed by `subsetter`), not `/FontFile3 /OpenType`: PDF 1.3+, no `cmap` question (Table 126), and what pypdf, pdf.js and PDFKit read without trouble. `/W` comes from the subset's own `hmtx`, so it matches an instance; CFF widths are checked against the source font.
- **Subset tag**: SHA-256 over the font bytes' digest, the collection index, the instance coordinates and the glyphs **in CID order** (which is what determines the subset, a superset of the "sorted used GIDs" in the scope), six letters `A + byte % 26`; a collision inside one document re-hashes with a salt. A "no subsetting" font has no tag (it is not a subset).
- **Reserve at first use**: `FontRegistry::reference(key, &doc)` reserves the number on first call; `finish_fonts(self, &mut CosMut)` writes fonts in registration order and skips fonts never referenced.
- **`Limits`**: no font limit was added to ADR 0003's `Limits`; font bytes come from the caller and `skrifa`/`subsetter` bounds-check every read. Worth revisiting if fonts from untrusted PDFs (B9b) are ever re-subset.
- **Fixtures** (`fixtures/fonts/`, ≈ 0.65 MB, OFL 1.1, provenance and SHA-256 in `fixtures/README.md`) are made by `tools/fixtures/fonts.py` (fontTools 4.68.0, `tools/fixtures/requirements.txt`) from checked downloads; `fsType` variants are patched in memory by the tests rather than committed. The CFF-flavoured OTFs are Noto Sans (name-keyed) and the Noto Sans SC subset (CID-keyed).
- **Goldens and extraction.** `fixtures/golden/text-{latin,cff,variable-collection}.pdf` come from `crates/lamina-text/examples/golden.rs`; `cargo xtask golden` now runs every crate's `golden` example (`GOLDEN_CRATES` in `tools/xtask/src/harness.rs`: a 6-line change). pypdf 6.21.3 extraction and `qpdf --check` (12.4.2) are precomputed into `fixtures/text/extracted.json` by `tools/fixtures/text_facts.py` and checked by `tests/extraction.rs` (facts tied to each golden's SHA-256), the repo's pattern for Python oracles; nothing new runs in CI. **pdf.js** (`pdfjs-dist` 4.10.38, `getTextContent`) was run by hand on the three goldens and returned exactly the input lines; **pdftotext** was not available locally. Wiring pdf.js/pdftotext extraction into the full lane is left to the `cargo xtask roundtrip`-style job (B3c/X3a own `tools/xtask` and `.github` right now).
- **Not done (follow-ups for the builder):** `/CIDSet` (PDF/A-1–3 profiles, B8), `/ActualText` for conflicts (B5b), colour glyphs. Hinting tables are kept by `subsetter` except for instanced fonts.
- Rendering was checked by eye (macOS PDFKit via `sips`): all three goldens draw correctly, the `wght` 700 instance and the TTC Bold member in bold.
- **After the `/code-review high` round (29 findings, fixed in one commit; regressions in `tests/review_fixes.rs`):** instancing rejects NaN/∞ (`invalid-axis-value`) and malformed `fvar` ranges (`invalid-font`, where `f32::clamp` would have panicked), drops coordinates equal to an axis default, and caps PostScript names at 56 characters with a hash suffix; an instance's `/FontBBox` comes from its instanced outlines; "no subsetting" fonts are embedded byte for byte (`/FontFile2`, or `/FontFile3 /OpenType` for CFF; a TTC member copied out standalone), and an instance or CFF2 font that forbids subsetting is rebuilt whole with the `whole-font-rebuilt` warning; `FontKey`s carry their registry's identity; `finish_fonts` refuses another document (`wrong-document`) before writing; `encode` checks every glyph before recording any; the subset-tag search is bounded and errors instead of an empty tag; the Type 0 `/BaseFont` of a CFF font is `Name-Identity-H` (§9.7.6.1); `FontUse` stores the font's name and glyph count (`use_glyph(gid, text)`); fonts are parsed once per `encode`, the font digest is computed lazily, and widths are computed once with the program. pypdf (now in `tools/fixtures/requirements.txt`) and qpdf (now pinned in `doctor.rs` as an optional tool) versions are recorded in the facts and checked by the tests. New codes: `invalid-axis-value`, `wrong-document` (errors, `usage`), `whole-font-rebuilt` (warning, `usage`), each with a `docs/errors/` page.
