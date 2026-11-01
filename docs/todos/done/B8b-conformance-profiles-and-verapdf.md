# B8b — Conformance profiles (PDF/UA-1, PDF/A-2b/2u/3b), output intents, veraPDF nightly and `0.2.0-alpha`

| Field | Value |
| --- | --- |
| ID | `B8b` |
| Status | `done` |
| Plan | [pdf-library §7 B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa), requirement [§4 F6](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — **M4 Library 0.2** |
| Depends on | [B4c](./B4c-standard-14-and-simple-fonts.md), [B7b](./B7b-versioning-and-0-1-0-alpha.md), [B8a](./B8a-structure-tree.md), [X3a](./X3a-nightly-lane.md) |
| Blocks | [E1b](../ready/E1b-xmp-packet-model.md), [E1c](../ready/E1c-metadata-ops.md), [E2b](../ready/E2b-page-structure-ops.md), [E2d](../ready/E2d-merge-and-split.md), [E3e](../ready/E3e-xmp-editor.md), [E3f](../ready/E3f-attachments-and-associated-files.md), [E6c](../ready/E6c-add-text.md), [E6d](../ready/E6d-watermarks-headers-footers.md), [E7i](../ready/E7i-structure-and-conformance-maintenance.md), [E15a](../ready/E15a-accessibility-feature-plan.md), [E15b](../ready/E15b-structure-tree-editing.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) tier 10 (veraPDF as an external process, any violation fails from B8), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (veraPDF never linked), [0001](../../decisions/0001-core-language-and-licence.md) (determinism), 0017 (version bump) |
| Branch | `feat/b8b-conformance-profiles` |
| PR title | `feat(build): PDF/UA-1 and PDF/A-2b/2u/3b profiles with build-time errors; veraPDF nightly` |
| Size | L (≈ 900–1,400 lines incl. ICC data handling, examples and workflow) |
| PR | [#143](https://github.com/laudebugs/lamina/pull/143) |

## Goal

Make accessible and archival output the easy path: choosing a profile turns every machine-checkable rule violation into a coded error at `finish()`, the builder writes the identification XMP, output intent and embedded sRGB profile the standards need, and veraPDF validates every generated example nightly with zero tolerance. Tag `0.2.0-alpha` locally (M4).

## Context

- [Research 03 §9.4 PDF/UA, WTPDF, Matterhorn](../../research/pdf/03-text-and-fonts.md#94-pdfua-wtpdf-matterhorn), [§9.3](../../research/pdf/03-text-and-fonts.md#93-language-alt-text-reading-order); [05 §3.7 conversion to PDF/A](../../research/pdf/05-creating-pdfs-from-code.md#37-conversion-to-pdfa); [04 §5.2 XMP](../../research/pdf/04-interactive-security.md#52-xmp-1432-iso-16684-1), [§6.2 associated files](../../research/pdf/04-interactive-security.md#62-associated-files-af-pdf-20-pdfa-3).
- ISO 14289-1 (PDF/UA-1), ISO 19005-2/-3 (PDF/A-2/3), Matterhorn Protocol 1.1 (machine-checkable conditions).
- [pdf-library §4 F6](../../plans/pdf-library.md#functional) (the error list) and [§8 risks](../../plans/pdf-library.md#8-risks) (human checks).
- krilla's validator rule lists (reference, attribution if ported).

## Scope

**In scope**

- `Profile::{None, PdfUa1, PdfA2b, PdfA2u, PdfA3b}` in `BuildOptions` (combinable UA + A where valid, e.g. `PdfA2u + PdfUa1`).
- `finish()` checks with coded errors (added to `fixtures/codes.json`): `missing-alt-text`, `missing-lang`, `missing-title` (UA: plus `DisplayDocTitle`), `untagged-content`, `link-without-structure`, `link-without-contents`, `font-not-embedded` (standard 14 rejected), `unmapped-glyph` (no Unicode for a glyph, A-2u/UA), `transparency-without-blending-space`/`device-colour-without-output-intent` (A), `encryption-forbidden` (A), `forbidden-action` (JavaScript, launch; A), `embedded-file-not-associated` (A-3b requires `/AF` + `AFRelationship`).
- XMP: `pdfaid:part`/`conformance`, `pdfuaid:part`, extension schemas when needed; Info ↔ XMP sync enforced.
- Output intent `GTS_PDFA1` with an embedded sRGB ICC profile (a permissively licensed sRGB profile, e.g. the ICC's sRGB v4 or a compact v2 profile; licence recorded) — bundled only in the text-free base or behind a feature to protect WASM size.
- Associated files API for PDF/A-3b (`builder.attach(name, bytes, mime, relationship)` → `/EmbeddedFiles` + `/AF`).
- Examples: `pdf_ua_report`, `pdf_a_archive` (2b and 2u), `pdf_a3_invoice_with_xml` (Rust + TS), docs pages "Accessible PDFs" and "Archival PDFs".
- veraPDF job in the nightly workflow ([X3a](../done/X3a-nightly-lane.md)): Docker image pinned by digest, run over every generated example for its profile (`--flavour ua1`, `2b`, `2u`, `3b`), any violation fails; also a manual `cargo xtask verapdf` for local runs.
- Version bump to `0.2.0-alpha.0` via a changeset; tag locally after merge.

**Out of scope** (and which todo covers it)

- Validating or fixing existing (non-lamina) files, auto-tagging: full-editor E15.
- PDF/A-1, PDF/A-4, PDF/UA-2/WTPDF: later.
- Human-only Matterhorn checks: documented as the caller's responsibility.

## Tasks

- [x] Load `/rust-best-practices`; read 03 §9.4, the Matterhorn machine checks list, ISO 19005-2 §6
- [x] `lamina-build/src/profile/{mod,checks,pdfa}.rs` (the PDF/UA-1 and PDF/A rules share `checks.rs`); error codes
- [x] XMP identification and Info sync; output intent + ICC data (`crates/lamina-build/data/` with its provenance, pointed to from `fixtures/README.md`; `include_bytes!` in the crate)
- [x] Attachments API and `/AF`
- [x] Examples (Rust + TS), docs pages
- [x] Nightly veraPDF job; `cargo xtask verapdf`
- [x] Changeset for `0.2.0-alpha.0`; tag after merge (owner)

## Acceptance criteria

- [x] All generated examples pass veraPDF for their profile (PDF/UA-1, PDF/A-2b, 2u, 3b) via `cargo xtask verapdf` (the same pinned veraPDF CLI as the nightly job, not a Docker image: see Outcome; output summarised in the PR); the nightly veraPDF job passes `actionlint`
- [x] Missing alt text, missing `Lang`, missing title, an untagged path and a standard 14 font each fail `finish()` with their coded error under the matching profile (one test per rule)
- [x] The machine-checkable Matterhorn conditions lamina can produce are covered by tests (listed in the PR with test names)
- [x] Outputs are byte-identical across runs; `qpdf --check` clean
- [x] `0.2.0-alpha.0` version sync passes (`cargo xtask version sync --check`)

## Outcome

- **API.** `BuildOptions::with_profile(Profile)` (TS `profiles: ["PDF/UA-1", "PDF/A-2u"]`): PDF/UA-1 combines with one PDF/A profile, a second PDF/A profile replaces the first and `Profile::None` clears them; PDF/UA-1 makes the document tagged. `Builder::attach(Attachment::new(name, bytes, mime).with_relationship(AfRelationship::Alternative))` (TS `attach(name, bytes, mime, { relationship, description, modDate })`). The checks run in `Builder::finish` before anything is written, and the first broken rule is the error.
- **Codes** (15, `buildErrors`): `missing-title`, `missing-lang`, `untagged-content`, `missing-alt-text`, `heading-level-skipped`, `unnumbered-heading-misused`, `invalid-structure-nesting`, `link-without-structure`, `link-without-contents`, `font-not-embedded`, `glyph-without-unicode`, `device-color-without-output-intent`, `embedded-file-forbidden`, `embedded-file-without-mime-type`, `profile-version-conflict`. `glyph-without-unicode` replaces the scope's `unmapped-glyph` (already an extraction warning code); `device-color-without-output-intent` is spelt like the existing `color-*` codes.
- **Not added, because lamina can't produce them:** `encryption-forbidden` (the builder never encrypts), `forbidden-action` (links are URI, page and named targets only), `transparency-without-blending-space` (every PDF/A document gets an output intent, which covers transparency) and `embedded-file-not-associated` (`attach` always writes `/AF` and `/AFRelationship`). PDF/A-2 attachments became `embedded-file-forbidden` instead. Five codes were added beyond the scope, for rules the builder's API lets a caller break: `heading-level-skipped` (Matterhorn 14-002, 14-003), `unnumbered-heading-misused` (14-006, 14-007), `invalid-structure-nesting` (09-004, 09-005), `embedded-file-without-mime-type` (ISO 19005-3 §6.8) and `profile-version-conflict` (PDF 2.0 under a PDF 1.7 profile; veraPDF 6.1.2-1). The builder also writes an `/ID` for every `Note` element and the `IDTree` (Matterhorn 19-003, 19-004), and requires alternate text on a `Formula` as on a `Figure` (17-002).
- **Output intent.** `GTS_PDFA1` with `sRGB-v4.icc` from Clinton Ingram's Compact ICC Profiles (480 bytes, CC0-1.0; provenance in `crates/lamina-build/data/README.md`). It is small enough for every WASM module, so no feature gates it. So `DeviceRGB` and `DeviceGray` are covered and `DeviceCMYK` is refused; ICC-based CMYK is fine.
- **veraPDF runs as its Java CLI, not Docker** (a deviation from the scope's "Docker image pinned by digest"): the Docker daemon isn't running on the CI Mac, whose runners share it. `cargo xtask verapdf` installs the pinned greenfield release 1.30.3 (installer SHA-256 in `doctor.rs`) with its unattended installer into `$RUNNER_TOOL_CACHE/lamina-verapdf` or `target/verapdf`, and runs it with `$LAMINA_JAVA`, Homebrew's `openjdk@21` or `java` on `PATH` (17+). veraPDF is GPL-3.0-or-later or MPL-2.0; it runs as an external process only, never linked or bundled (ADR 0019). Each PDF's flavours come from its XMP identification, so a new example is validated without an xtask change.
- **Examples.** `pdf_ua_report`, `pdf_a_archive` (`-2b` and `-2u`, the latter also PDF/UA-1, which exercises the `pdfuaid` extension schema) and `pdf_a3_invoice_with_xml`, in Rust and TS, byte-identical, with render goldens. veraPDF validates the Rust outputs only; the TS ones are covered because they are byte-identical to them (the TS example suite compares each with its Rust golden). `cargo xtask examples` takes an example's `<name>-<variant>.pdf` outputs.

## Post-merge checks

- [ ] The nightly veraPDF job is green in the first nightly run on `main` (link it in a follow-up PR or the done todo)
- [ ] The owner tags `v0.2.0-alpha.0` on `main`; the release lane on the tag is green and publishes nothing

## Tests

- `crates/lamina-build/tests/{profile_ua,profile_a,attachments}.rs`; TS tests for profile errors.
- Nightly veraPDF over `target/examples-out/` (both languages).

## Docs to update

- [x] [pdf-library §10 B8](../../plans/pdf-library.md#10-to-dos): tick profiles, XMP/output intents, veraPDF nightly, examples/docs (the `0.2.0-alpha` tag stays open for the owner); "reject [standard 14] under PDF/A and PDF/UA profiles" in B4 was already ticked
- [x] `fixtures/README.md` (ICC profile licence), docs site pages, `CLAUDE.md` (`cargo xtask verapdf`)
- [ ] [roadmap](../../plans/roadmap.md) M4 status (parent/owner)

## Risks and open questions

- sRGB ICC licence and size (v4 profile ~ 60 KB vs compact v2 ~ 3 KB); prefer a compact profile with a clear licence.
- veraPDF version differences change verdicts; pin and update deliberately.
- Some rules (e.g. reading order sanity) are only partially checkable; docs must say so.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
