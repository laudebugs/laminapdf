# E8c — The form view, `SetFieldValue`, tab order and field-level locks

| Field | Value |
| --- | --- |
| ID | `E8c` |
| Status | `done` |
| Plan | [E8 plan §2.1–§2.2](../../plans/editor/e8-form-filling.md#21-ops) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8b](./E8b-form-filling-feature-plan.md) |
| ADRs | Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md), [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) (amends §9's FieldMDP rule) |
| Branch | `feat/e8c-acroform-field-model` |
| PR title | `feat(model): AcroForm field view and form-fill ops` |
| Size | L (400–800 of code; fixtures and tests beyond): the view, one op, the planner's field check |
| PR | [#733](https://github.com/laudebugs/laminapdf/pull/733) |

## Goal

The library reads every form field and sets its value. `doc.form()` walks the AcroForm tree with inheritance, `SetFieldValue` writes a value of any field type (appearances follow in E8d), `tab_order(page)` follows `/Tabs`, and the save planner checks FieldMDP and `/Lock` field by field, so a lock refuses only the fields it covers.

## Context

- [E8 plan §2.1, §2.2 and §5](../../plans/editor/e8-form-filling.md#21-ops)
- [04 §3.1 AcroForm structure](../../research/pdf/04-interactive-security.md#31-acroform-structure-127-interactive-form-dictionary-1273-17-1272) and [§3.2 Field types and flags](../../research/pdf/04-interactive-security.md#32-field-types-and-flags)
- ISO 32000-2 §12.7.3 (interactive form dictionary), §12.7.4 (field dictionaries, names, inheritance), §12.7.5 (field types), §7.7.3.3 (`/Tabs`), §12.8.2.4 (FieldMDP)
- `laminapdf-model`'s `widgets.rs` (`page_widgets`, fully qualified names) and ADR 0007 §9's E2r field index, which this todo reuses

## Scope

**In scope**

- `Form`, `Field`, `FieldKind` (text, check box, radio group, push button, combo box, list box, signature), `FieldFlags`, `FieldValue`, `Widget`; inheritance of `/FT`, `/Ff`, `/V`, `/DV`, `/DA`, `/Q`; fields sharing a name; orphan widgets and widgetless fields reported
- `SetFieldValue { field, value }` (class `FormFill`) with `field-value-invalid` and `field-is-signature`; `/V`, `/AS`, `/I` and RadiosInUnison kept in step; `/RV` dropped on a text change
- `tab_order(page)` for `/Tabs` `R`, `C`, `S`, `A`, `W` and none (`/Annots` order)
- The planner's field-level FieldMDP and `/Lock` check for form ops; ADR 0007 §9 amended
- `fixtures/forms/fields.pdf` and `tabs.pdf` (`tools/fixtures/forms.py`); `fixtures/signed/form-certified-p2.pdf` and `form-locked-include.pdf` (`make.py`)
- Façade `form`, `tab_order`; SDK `formFields`, `tabOrder`, the op through `apply`

**Out of scope** (and which todo covers it)

- Appearances → [E8d](../done/E8d-field-appearances.md)
- Reset, import and `fill_form` → [E8g](../done/E8g-form-data-and-flatten.md)
- XFA → [E8h](../ready/E8h-xfa-detection-and-notice.md)
- The live form differential against pypdf and pdf.js → [E8o](../ready/E8o-live-form-differential.md), split from this todo to keep the PR reviewable

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.06` required and read-only; `E.09` tab order
- **Crates and packages:** `laminapdf-model` (`forms/`), `laminapdf-ops`, `laminapdf-save`, the façade, `laminapdf-wasm`, `@laminapdf/sdk`
- **`Op` variants:** `SetFieldValue` (class `FormFill`, no rewrite)
- **`SaveConstraint`:** as [E8 plan §5](../../plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint): allowed under approval signatures and DocMDP P2/P3, a violation under P1 and on a field a lock covers
- **Web vs desktop:** identical (core)
- **Server features:** none
- **Capability-matrix rows to flip:** `E.06` → `round-trip` (core, pdf); `E.09` → `read` (core, pdf)

## Tasks

- [x] `forms/` in `laminapdf-model`: the view, tolerant reading with `max_tree_depth` and cycle checks, the reconciliation report
- [x] `SetFieldValue` in `laminapdf-ops`' `forms/`, its errors (codes in `fixtures/codes.json`, pages in `docs/errors/`)
- [x] `tab_order`
- [x] The planner's field-level lock check; ADR 0007 §9's text (and a new §12)
- [x] Fixtures, façade, bindings, rustdoc and TSDoc, a runnable example (`fill_form`), frozen samples in `fixtures/ops/v1/`

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7) for the op and both functions
- [x] Apply then undo leaves the overlay empty and the next save byte-identical (ADR 0006 §3); redo re-applies the same bytes
- [x] `would_violate` on every `fixtures/signed/` file gives the plan's §5 verdict; on `form-locked-include.pdf`, a locked field is a violation naming the signature and an unlocked one isn't
- Moved to [E8o](../ready/E8o-live-form-differential.md): field names, types and values match pypdf and pdf.js on the pinned corpus's forms. The fixtures' values were checked against pdf.js and PDFium by hand, and the round-trip suite has both read the filled values back
- [x] Save-time metadata (moved from [E1c](../done/E1c-metadata-ops.md)): a `SetFieldValue` on `fixtures/signed/form-certified-p2.pdf` (`certified-p2.pdf` has no field but its signature's) saved with `metadataDate` saves `AppendOnly`, `SaveReport::metadata_not_updated` is set, and the appended revision touches neither the Info dictionary nor `/Metadata`

## Post-merge checks

None.

## Tests

- Unit tests per field kind, refusal and `/Tabs` value
- The form corpus differential (nightly), now [E8o](../ready/E8o-live-form-differential.md), and a `form_tree` fuzz target, added to `laminapdf-model`'s regression guard list
- The planner verdicts in `laminapdf-save`'s tests

## Docs to update

- API docs: reading and filling forms
- ADR 0007 §9 (the field-level rule)
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Fields reached both from `/Fields` and only from `/Annots` can disagree on their parent. Decided in the PR: the view follows the tree (`/Fields` and `/Kids`), as qpdf does, and reports each kid whose `/Parent` names another field (`Form::parent_mismatches`); pdf.js follows `/Parent`, a difference E8o's differential excuses

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
