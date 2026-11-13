# E5e — Flatten annotations into page content

| Field | Value |
| --- | --- |
| ID | `E5e` |
| Status | `ready` |
| Plan | [E5 plan §2.1](../../plans/editor/e5-review-stamps-interchange.md#21-ops) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E2e](../done/E2e-content-append-api.md), [E4c](../done/E4c-appearances-markup-notes-caret.md), [E4d](./E4d-appearances-ink-and-shapes.md), [E5a](../done/E5a-review-stamps-interchange-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5e-flatten` |
| PR title | `feat(ops): flatten annotations into page content` |
| Size | M (200–400): one op over the content append API, and the appearance functions moved to the model |
| PR | — |

## Goal

`FlattenAnnots`: each chosen annotation's normal appearance drawn into its page through the content append API, then the annotation removed, as [E5 plan §2.1](../../plans/editor/e5-review-stamps-interchange.md#21-ops) sets out.

## Context

- [E5 plan §2.1, §3 and §5](../../plans/editor/e5-review-stamps-interchange.md#21-ops)
- [04 §2.5 Flattening](../../research/pdf/04-interactive-security.md#25-flattening); ISO 32000-2 §12.5.5
- [E2e](../done/E2e-content-append-api.md): the content append API

## Scope

**In scope**

- `FlattenAnnots { annots, intent }` (class `Content`): `/AS` resolved, `wrap_existing`, `add_xobject`, `q <A> cm /FmN Do Q`, the annotation, its Popup and its replies removed
- Skipped: hidden for the intent, optional content that is off; `flatten-no-appearance` when no appearance can be made; `flatten-widget` for widgets
- Tagged files: the appearance drawn as an artifact and its `Annot` element removed; `flatten-untagged` on PDF/UA-1 files
- `still_in_earlier_revisions` set
- The appearance and matrix functions moved from `laminapdf-extract` to `laminapdf-model`, both crates using one copy
- `fixtures/annotations/flatten.pdf`

**Out of scope** (and which todo covers it)

- The comment summary → [E5f](E5f-comment-summary.md)
- The dialog → [E5i](E5i-interchange-flatten-summary-ui.md)
- Form fields → [E8g](E8g-form-data-and-flatten.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.14` flatten annotations into content
- **Crates and packages:** `laminapdf-ops`, `laminapdf-model`, `laminapdf-content`, `laminapdf-extract`, `laminapdf`, `@laminapdf/sdk`
- **`Op` variants:** `FlattenAnnots` (class `Content`)
- **`SaveConstraint`:** as [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint): a violation under any certification
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `D.14` → `round-trip` (core, pdf)

## Tasks

- [ ] The op
- [ ] The move of the appearance functions
- [ ] Fixtures and the corpus test
- [ ] Bindings and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new op and function is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdicts of [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in Lamina PDF, pdf.js and PDFium preserves the change and everything else; `qpdf --check` is clean
- [ ] PDFium's render of each flattened page of `flatten.pdf` matches its render before flattening within E4g's threshold
- [ ] The pdf.js corpus's annotated files flatten and render the same, or are listed with their reason

## Post-merge checks

None.

## Tests

- Unit tests per flag, intent and subtype
- `save_roundtrip` gains the op
- The flatten corpus test

## Docs to update

- API docs: flattening
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
