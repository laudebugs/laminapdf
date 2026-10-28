# B8b — Conformance profiles (PDF/UA-1, PDF/A-2b/2u/3b), output intents, veraPDF nightly and `0.2.0-alpha`

| Field | Value |
| --- | --- |
| ID | `B8b` |
| Status | `ready` |
| Plan | [pdf-library §7 B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa), requirement [§4 F6](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — **M4 Library 0.2** |
| Depends on | [B4c](./B4c-standard-14-and-simple-fonts.md), [B7b](./B7b-versioning-and-0-1-0-alpha.md), [B8a](./B8a-structure-tree.md), [X3a](./X3a-nightly-lane.md) |
| Blocks | [E1b](./E1b-xmp-packet-model.md), [E1c](./E1c-metadata-ops.md), [E2b](./E2b-page-structure-ops.md), [E2d](./E2d-merge-and-split.md), [E3e](./E3e-xmp-editor.md), [E3f](./E3f-attachments-and-associated-files.md), [E6c](./E6c-add-text.md), [E6d](./E6d-watermarks-headers-footers.md), [E7i](./E7i-structure-and-conformance-maintenance.md), [E15a](./E15a-accessibility-feature-plan.md), [E15b](./E15b-structure-tree-editing.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) tier 10 (veraPDF as an external process, any violation fails from B8), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (veraPDF never linked), [0001](../../decisions/0001-core-language-and-licence.md) (determinism), 0017 (version bump) |
| Branch | `feat/b8b-conformance-profiles` |
| PR title | `feat(build): PDF/UA-1 and PDF/A-2b/2u/3b profiles with build-time errors; veraPDF nightly` |
| Size | L (≈ 900–1,400 lines incl. ICC data handling, examples and workflow) |
| PR | — |

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
- veraPDF job in the nightly workflow ([X3a](./X3a-nightly-lane.md)): Docker image pinned by digest, run over every generated example for its profile (`--flavour ua1`, `2b`, `2u`, `3b`), any violation fails; also a manual `cargo xtask verapdf` for local runs.
- Version bump to `0.2.0-alpha.0` via a changeset; tag locally after merge.

**Out of scope** (and which todo covers it)

- Validating or fixing existing (non-lamina) files, auto-tagging: full-editor E15.
- PDF/A-1, PDF/A-4, PDF/UA-2/WTPDF: later.
- Human-only Matterhorn checks: documented as the caller's responsibility.

## Tasks

- [ ] Load `/rust-best-practices`; read 03 §9.4, the Matterhorn machine checks list, ISO 19005-2 §6
- [ ] `lamina-build/src/profile/{mod,ua1,pdfa,checks}.rs`; error codes
- [ ] XMP identification and Info sync; output intent + ICC data (`fixtures/icc/` provenance; `include_bytes!` in the crate)
- [ ] Attachments API and `/AF`
- [ ] Examples (Rust + TS), docs pages
- [ ] Nightly veraPDF job; `cargo xtask verapdf`
- [ ] Changeset for `0.2.0-alpha.0`; tag after merge (owner)

## Acceptance criteria

- [ ] All generated examples pass veraPDF for their profile (PDF/UA-1, PDF/A-2b, 2u, 3b) via `cargo xtask verapdf` (same pinned Docker image as the nightly job; output summarised in the PR); the nightly veraPDF job passes `actionlint`
- [ ] Missing alt text, missing `Lang`, missing title, an untagged path and a standard 14 font each fail `finish()` with their coded error under the matching profile (one test per rule)
- [ ] The machine-checkable Matterhorn conditions lamina can produce are covered by tests (list them in the PR with test names)
- [ ] Outputs are byte-identical across runs; `qpdf --check` clean
- [ ] `0.2.0-alpha.0` version sync passes (`cargo xtask version sync --check`)

## Post-merge checks

- [ ] The nightly veraPDF job is green in the first nightly run on `main` (link it in a follow-up PR or the done todo)
- [ ] The owner tags `v0.2.0-alpha.0` on `main`; the release lane on the tag is green and publishes nothing

## Tests

- `crates/lamina-build/tests/{profile_ua,profile_a,attachments}.rs`; TS tests for profile errors.
- Nightly veraPDF over `target/examples-out/` (both languages).

## Docs to update

- [ ] [pdf-library §10 B8](../../plans/pdf-library.md#10-to-dos): tick profiles, XMP/output intents, veraPDF nightly, examples/docs, `0.2.0-alpha` tag; also "reject [standard 14] under PDF/A and PDF/UA profiles" in B4
- [ ] `fixtures/README.md` (ICC profile licence), docs site pages, `CLAUDE.md` (`cargo xtask verapdf`)
- [ ] [roadmap](../../plans/roadmap.md) M4 status (parent/owner)

## Risks and open questions

- sRGB ICC licence and size (v4 profile ~ 60 KB vs compact v2 ~ 3 KB); prefer a compact profile with a clear licence.
- veraPDF version differences change verdicts; pin and update deliberately.
- Some rules (e.g. reading order sanity) are only partially checkable; docs must say so.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
