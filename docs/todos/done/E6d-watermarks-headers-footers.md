# E6d — Watermarks, backgrounds, headers/footers, page numbers and Bates numbering

| Field | Value |
| --- | --- |
| ID | `E6d` |
| Status | `done` |
| Plan | [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [B8a](./B8a-structure-tree.md), [B8b](./B8b-conformance-profiles-and-verapdf.md), [E6c](./E6c-add-text.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e6d-watermarks-headers-footers` |
| PR title | `feat(content): watermarks, backgrounds, headers, footers and Bates numbers` |
| Size | L (600–1,500) |
| PR | [#835](https://github.com/laudebugs/laminapdf/pull/835) |

## Goal

Stamp watermarks (append) and backgrounds (prepend) as Form XObjects, optionally in an OCG and marked `/Artifact`; headers/footers with page-number and date tokens; Bates numbering with prefix/suffix/start/digits across one or many documents; update/remove previously added ones.

## Context

- [T4b](../done/T4b-flow-layout-core.md) proposes page-number fields (`{page}`, `{pages}`) for flowing layout in `laminapdf-build`. This todo and T4b use one syntax for these tokens: whichever lands first defines it, and the second reuses it.
- **Design (Paper):** [A03 Watermark, header and footer dialog](../../design/paper/add-edit-content.md#a03-watermark-header-and-footer-dialog). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [05 §3.3 Stamping and watermarking](../../research/pdf/05-creating-pdfs-from-code.md#33-stamping-and-watermarking)
- [03 §9](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149): artifacts in tagged PDF

## Scope

**In scope**

- `AddWatermark`, `AddBackground`, `SetHeaderFooter`, `ApplyBates`, `UpdatePageMark` and `DeletePageMark`, with the payloads of the [E6 plan §2.1](../../plans/editor/e6-adding-content.md#21-ops)
- Batch-ready: Bates across files uses the same op sequence (E14d)

**Out of scope** (and which todo covers it)

- The Page marks dialog and Manage page marks → [E6g](../ready/E6g-page-marks-dialog.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `B.10` background images and watermarks; `C.14` labels, page numbers, headers/footers
- **Crates and packages:** `laminapdf-content`, `laminapdf-build`, `laminapdf-ops`
- **`Op` variants:** the ops above (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked unless the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock` (the signature panel, E10h, reports the later content change)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `B.10` → `round-trip` (core, pdf); `C.14` → `round-trip` (core, pdf) (web and desktop flip with the dialog, in [E6g](../ready/E6g-page-marks-dialog.md))

## Tasks

- [x] Ops and builders: the six ops, their specs and `page_marks` in `laminapdf-ops`' `added/`; the façade's `watermark`, `background`, `header_footer`, `bates`, `update_mark`, `delete_mark` and `page_marks`; `doc.content.addWatermark`, `addBackground`, `setHeaderFooter`, `applyBates`, `updatePageMark`, `deletePageMark` and `pageMarks`
- [x] Token expansion: `{page}`, `{pages}`, `{label}` (the page label), `{date}` and `{file}` (the payload's), `{{` and `}}` for braces, filled when the op applies; one syntax with the flowing layout's `fill_fields`, which has `{section}` and `{title}` instead of the last three
- [x] OCG and Artifact marking: one optional content group for a hideable mark, named in the layers panel; artifacts of their kind in a tagged file
- [x] The overlap check's runs in the header and footer bands, for E6g's notice: `Document::page_bands`

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Headers/footers are Artifacts in tagged files (veraPDF passes): the artifact is written and read back in the ops' tests; veraPDF itself runs on CI only
- [x] Watermark OCG toggles in pdf.js layer panel: pdf.js lists the group, visible, in the layers order

## Post-merge checks

None.

## Tests

- Render goldens
- veraPDF
- e2e

## Docs to update

- API docs: the recipe [Add watermarks, headers and Bates numbers](../../../apps/docs/src/content/docs/recipes/add-page-marks.mdx). The reader's user help for watermarks and headers waits for the dialog, which is [E6g](../ready/E6g-page-marks-dialog.md)'s
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- A font given by its bytes is embedded once for each distinct text, so a header with `{page}` in such a font carries a subset a page: a long document should use Helvetica, Times or Courier for its page numbers, or a font that is small. Text that is the same on every page of one size, a watermark's, is drawn and embedded once.
- PDF/A-1 forbids optional content, and `hideable` there is not refused or warned of: the recipe and the dialog's design note say to leave it out, and a save plan's warning is a later change if one is wanted.
- A slot is one line: a text wider than the page between its margins wraps and what doesn't fit is clipped, with no warning. The dialog is to measure the text before it adds the mark.
- `Document::page_bands` is the façade's only; the SDK and `@laminapdf/wasm` have no export for it, so [E6g](../ready/E6g-page-marks-dialog.md) either reads `content.textRuns` with the page's size or adds one.
- Round trip, pdf.js and PDFium: the round trip's `addWatermark`, `addBackground`, `setHeaderFooter` and `applyBates` cases check that Lamina PDF finds each mark again and name the phrase each reads back, but only the Lamina leg ran locally; pdf.js read the layer of the example's hideable watermark in a script; veraPDF, PDFium's reading of the text and the parity matrix's render goldens run on CI only.
- `UpdateContentObject`'s `order` counts a mark among the objects on its layer, as it counts every added form; moving an object past a mark is possible, and the page-marks dialog never offers it.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
