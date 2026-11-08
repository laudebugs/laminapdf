# E6d — Watermarks, backgrounds, headers/footers, page numbers and Bates numbering

| Field | Value |
| --- | --- |
| ID | `E6d` |
| Status | `ready` |
| Plan | [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [B8a](../done/B8a-structure-tree.md), [B8b](../done/B8b-conformance-profiles-and-verapdf.md), [E6c](./E6c-add-text.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e6d-watermarks-headers-footers` |
| PR title | `feat(content): watermarks, backgrounds, headers, footers and Bates numbers` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Stamp watermarks (append) and backgrounds (prepend) as Form XObjects, optionally in an OCG and marked `/Artifact`; headers/footers with page-number and date tokens; Bates numbering with prefix/suffix/start/digits across one or many documents; update/remove previously added ones.

## Context

- [T4b](../done/T4b-flow-layout-core.md) proposes page-number fields (`{page}`, `{pages}`) for flowing layout in `laminapdf-build`. This todo and T4b use one syntax for these tokens: whichever lands first defines it, and the second reuses it.
- **Design (Paper):** [A03 Watermark, header and footer dialog](../../design/paper/add-edit-content.md#a03-watermark-header-and-footer-dialog). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [05 §3.3 Stamping and watermarking](../../research/pdf/05-creating-pdfs-from-code.md#33-stamping-and-watermarking)
- [03 §9](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149): artifacts in tagged PDF

## Scope

**In scope**

- `AddWatermark`, `AddBackground`, `SetHeaderFooter { template, pages, tokens }`, `ApplyBates { spec }`, and their removal ops for lamina-added items
- Dialogs with preview
- Batch-ready: Bates across files uses the same op sequence (E14d)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `B.10` background images and watermarks; `C.14` labels, page numbers, headers/footers
- **Crates and packages:** `laminapdf-content`, `laminapdf-build`, `laminapdf-ops`, `@laminapdf/ui`
- **`Op` variants:** the ops above (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked unless the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock` (the signature panel, E10h, reports the later content change)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `B.10` → `round-trip` (core, pdf, web, desktop); `C.14` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Ops and builders
- [ ] Token expansion (page label, number, total, date from payload)
- [ ] OCG and Artifact marking
- [ ] Dialogs

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Headers/footers are Artifacts in tagged files (veraPDF passes)
- [ ] Watermark OCG toggles in pdf.js layer panel

## Post-merge checks

None.

## Tests

- Render goldens
- veraPDF
- e2e

## Docs to update

- User help: watermarks and headers
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
