# B4a — `lamina-text`: font loading, subsetting and Type 0 embedding with ToUnicode

| Field | Value |
| --- | --- |
| ID | `B4a` |
| Status | `ready` |
| Plan | [pdf-library §7 B4](../../plans/pdf-library.md#b4-fonts-and-text), requirement [§4 F2](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B1b](../done/B1b-krilla-fit-spike.md), [B3b](./B3b-lamina-save-writers.md), [X4](./X4-full-lane-gate.md) |
| Blocks | [B4b](./B4b-shaping-bidi-line-breaking.md), [B4c](./B4c-standard-14-and-simple-fonts.md), [B9b](./B9b-text-extraction-core.md) |
| ADRs | Follows [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (own builder; `skrifa`, `subsetter`; reserve-then-fill objects), [0001](../../decisions/0001-core-language-and-licence.md) (caller supplies bytes, determinism), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `feat/b4a-font-embedding` |
| PR title | `feat(text): font loading, subsetting and Type 0 embedding with /W and ToUnicode` |
| Size | L (≈ 900–1,300 lines) |
| PR | — |

## Goal

Embed fonts correctly from code: load TrueType/OpenType/CFF fonts (and TTC members, and variable-font instances) from caller-supplied bytes, track the glyphs a document uses, and at `finish` write a deterministic subset as a Type 0 / Identity-H font with `/W` widths and a ToUnicode CMap, so text is selectable and extractable. This is the font half of `lamina-text`; shaping is [B4b](./B4b-shaping-bidi-line-breaking.md), simple fonts and the standard 14 are [B4c](./B4c-standard-14-and-simple-fonts.md).

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
- Standard 14 fonts, simple TrueType/WinAnsi embedding: [B4c](./B4c-standard-14-and-simple-fonts.md).
- `surface.text()` API: [B5b](./B5b-builder-text-and-document-features.md).
- Reading fonts from existing PDFs for extraction: [B9b](./B9b-text-extraction-core.md).
- Colour fonts / emoji: later (03 §7.8), reported as unsupported.

## Tasks

- [ ] Load `/rust-best-practices`; read 03 §3.5–6 and §10; read `spikes/krilla-fit/src/exp_c.rs`
- [ ] Add `skrifa`, `subsetter`, `read-fonts` to workspace deps; `cargo xtask deny`; check wasm32
- [ ] Create `crates/lamina-text` (core), `font.rs` (`Font`, `FontInfo`, TTC), `variable.rs`, `fstype.rs`
- [ ] `usage.rs` (`FontUse`, GID → text map, deterministic ordering via `BTreeMap`)
- [ ] `embed/type0.rs` (font dict, descendant CIDFont, descriptor, `/W` writer), `embed/tounicode.rs` (CMap writer), `embed/subset.rs` (subsetter call, tag derivation)
- [ ] Integrate with the overlay: `FontRegistry` on a document-level builder context that reserves `ObjRef`s and fills them in `finish_fonts(&mut CosMut)`
- [ ] Fixtures: OFL fonts under `fixtures/fonts/` (Noto Sans, Noto Sans Arabic, Noto Sans Devanagari, Noto Sans Hebrew, Noto Sans CJK subset or Noto Sans SC subset, a CFF-flavoured OTF, a variable font, a TTC) with licence files and SHA-256 in `fixtures/README.md`; keep total size small (pre-subset large CJK to a test range)
- [ ] Tests below

## Acceptance criteria

- [ ] Embedding the same text twice produces byte-identical fonts and subset tags
- [ ] Generated test PDFs pass `qpdf --check` and pypdf `extract_text()` returns the input string (including `fi`/`ffi` ligature glyphs mapped via ToUnicode, accents, curly quotes, a non-BMP character)
- [ ] `/W` widths equal the subset's `hmtx` advances scaled to 1/1000 em (test compares with skrifa)
- [ ] ToUnicode CMap parses with pdf.js (round-trip extraction in the full lane) and has ≤ 100 entries per block
- [ ] A font with `fsType` restricted licensing fails without override and embeds with it; a "no subsetting" font is embedded whole
- [ ] A TTC member and a variable-font instance (wght=700) embed and extract
- [ ] A missing glyph yields `missing-glyph` with the character and font name

## Post-merge checks

None.

## Tests

- Unit: `/W` compression, CMap writer (bfrange splitting, surrogates), subset tag derivation.
- Integration `crates/lamina-text/tests/{embed,tounicode,fstype,variable}.rs`: build PDFs with the test page writer, reopen with `lamina-cos`, check font dicts; goldens `fixtures/golden/text-*.pdf` via `cargo xtask golden --update`.
- External (full lane via `cargo xtask roundtrip` style script): pypdf and pdftotext extraction equals input.

## Docs to update

- [ ] [pdf-library §10 B4](../../plans/pdf-library.md#10-to-dos): tick `Font::from_bytes` + TTC + variable instancing, `subsetter` + subset tags + `/W` + ToUnicode, missing-glyph diagnostics
- [ ] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick `lamina-text` (generation half)
- [ ] `fixtures/README.md` (fonts, licences), `CLAUDE.md` layout, crate README

## Risks and open questions

- `subsetter`'s variable-font instancing support and CFF2 handling: verify on the pinned version; if missing, ship instancing as a follow-up todo and error clearly meanwhile.
- `skrifa` versions move fast (krilla pinned 0.42 while 0.49 is current); pin one and track with Dependabot.
- CJK fixture size: commit only a pre-subset test font; full fonts fetched in the nightly lane.
- WASM size: `skrifa` + `subsetter` belong in the text module ([B6a](./B6a-lamina-wasm-and-adr-0016.md)); keep `lamina-text` separable from base crates (no dependency from `lamina-build`'s non-text parts on it — feature-gate).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
