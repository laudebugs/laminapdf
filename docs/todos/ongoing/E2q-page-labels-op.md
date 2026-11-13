# E2q — The `SetPageLabels` op

| Field | Value |
| --- | --- |
| ID | `E2q` |
| Status | `ongoing` |
| Plan | [E2 page organisation §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2j](../done/E2j-page-tree-and-tree-writers.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2q-page-labels-op` |
| PR title | `feat(ops): set page label ranges` |
| Size | M (200–400): one op over [E2j](../done/E2j-page-tree-and-tree-writers.md)'s number-tree writer, with its bindings and example |
| PR | — |

## Goal

Edit a document's page labels (style, prefix and first number per range) as one op, in core and the TypeScript library, without waiting for the interface. Split out of [E2g](../ready/E2g-page-labels.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos), so the core and `pdf` columns of `I.10` don't wait on the page-setup dialogs.

## Context

- [E2 plan §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions): `SetPageLabels { ranges }`
- [04 §1.5 Page labels](../../research/pdf/04-interactive-security.md#15-page-labels-thumbnails-viewer-preferences-page-modelayout)
- ISO 32000-2 §12.4.2 (page labels)

## Scope

**In scope**

- `SetPageLabels { ranges: Vec<LabelRange> }`: the whole `/PageLabels` number tree, written with [E2j](../done/E2j-page-tree-and-tree-writers.md)'s writer; an empty list removes it
- Ranges validated: first pages in order and inside the document, a known style, a first number of at least 1

**Out of scope** (and which todo covers it)

- Keeping labels right as pages move is E2j's `PageLabels::remap`, which every page op calls
- The labels editor dialog and thumbnail captions → [E2g](../ready/E2g-page-labels.md)
- The test that labels survive every page op, merge and extract → [E2i](../ready/E2i-signed-gating-and-close.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.10` page labels (core, `pdf`)
- **Crates and packages:** `laminapdf-ops`, the façade and bindings
- **`Op` variants:** `SetPageLabels` (class `Document`)
- **`SaveConstraint`:** class `Document`; the verdicts per constraint are the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint)
- **Web vs desktop:** identical (core)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `I.10` → `round-trip` (core, pdf)

## Tasks

- [ ] `SetPageLabels` with validation and undo
- [ ] Bindings (`doc.pages.setLabels()`) and examples in Rust and TypeScript

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict of the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Labels shown by pdf.js and PDFium match the op's ranges on 5 fixtures (roman, alpha, prefixed, restarts)

## Post-merge checks

None.

## Tests

- Unit tests for range validation
- Round-trip with pdf.js and PDFium label facts

## Docs to update

- API docs: page labels
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
