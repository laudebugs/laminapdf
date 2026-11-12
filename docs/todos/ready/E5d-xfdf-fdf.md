# E5d — XFDF, FDF and Markdown: import and export comments

| Field | Value |
| --- | --- |
| ID | `E5d` |
| Status | `ready` |
| Plan | [E5 plan §2.3](../../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E5a](../done/E5a-review-stamps-interchange-feature-plan.md), [E5b](./E5b-comment-threads-and-review.md), [E5c](./E5c-stamps-and-attachments.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5d-xfdf-fdf` |
| PR title | `feat(interchange): import and export comments as XFDF, FDF and Markdown` |
| Size | L (400–800): a new crate with two readers, three writers and the import planner |
| PR | — |

## Goal

A new core crate, `laminapdf-interchange`, that exports comments as XFDF, FDF or Markdown and imports XFDF and FDF as one transaction of ordinary ops, in C06's three modes.

## Context

- [E5 plan §2.3](../../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary) and [§10](../../plans/editor/e5-review-stamps-interchange.md#10-decisions-on-the-open-questions)
- [04 §3.6 FDF and XFDF](../../research/pdf/04-interactive-security.md#36-fdf-and-xfdf); ISO 19444-1:2016 (XFDF); ISO 32000-2 §12.7.8 (FDF)
- **Design (Paper):** [C06 Import and export comments](../../design/paper/comment-review.md#c06-import-and-export-comments)

## Scope

**In scope**

- `laminapdf-interchange` (layer core): the XFDF reader on quick-xml with no DTD and only XML's five entities, within `Limits`; the FDF reader on `laminapdf-syntax`'s parser; XFDF, FDF and Markdown writers
- `export_annotations(&ExportOptions)` and `import_annotations(bytes, &ImportOptions)` in the façade, WASM and SDK
- Import modes Merge (by `/NM` on the same page), Replace and Add; appearances regenerated; `interchange-skipped` for subtypes no spec covers and for form fields; `interchange-document-mismatch` for another file's `/ID` or name; a malformed file's error names its line
- `fixtures/interchange/` and `tools/fixtures/interchange.py`
- The `xfdf` and `fdf` fuzz targets, their regressions folders in the model's guard list

**Out of scope** (and which todo covers it)

- The dialogs → [E5i](E5i-interchange-flatten-summary-ui.md)
- Form fields → [E8g](E8g-form-data-and-flatten.md), which reuses the crate's readers

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.13` import and export annotation data
- **Crates and packages:** `laminapdf-interchange` (new), `laminapdf`, `laminapdf-wasm`, `@laminapdf/sdk`
- **`Op` variants:** none new: an import is one transaction of E4's and E5b's ops
- **`SaveConstraint`:** as [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint); on usage rights an import needs the rights for each op it applies
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `D.13` → `round-trip` (core, pdf)

## Tasks

- [ ] The crate and its layer
- [ ] Readers and writers
- [ ] The import planner and its modes
- [ ] Fixtures and fuzz targets
- [ ] Bindings and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new op and function is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for an import, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Round-trip: importen → import → save → reimporten in Lamina PDF, pdf.js and PDFium preserves the change and everything else; `qpdf --check` is clean
- [ ] Exporting `review.pdf`, importing into a copy without comments and exporting again gives the same file
- [ ] An XFDF with a DTD is refused, and a malformed one reports its line
- [ ] Acrobat-made exports from [issue #521](https://github.com/laudebugs/laminapdf/issues/521) import without loss, once supplied

## Post-merge checks

None.

## Tests

- Unit tests per mode and per subtype
- The `xfdf` and `fdf` fuzz targets
- The round trip of export and import

## Docs to update

- API docs: importing and exporting comments
- `fixtures/README.md`: `fixtures/interchange/`
- CLAUDE.md: the new crate in the layout
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- XFDF from other apps varies; record each difference in the crate's README

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
