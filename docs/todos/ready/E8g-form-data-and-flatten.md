# E8g — Form data (FDF, XFDF, JSON), programmatic fill, reset and flatten

| Field | Value |
| --- | --- |
| ID | `E8g` |
| Status | `ready` |
| Plan | [E8 plan §2.1](../../plans/editor/e8-form-filling.md#21-ops) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E2e](../done/E2e-content-append-api.md), [E5d](../ongoing/E5d-xfdf-fdf.md), [E5e](../done/E5e-flatten-and-summary.md), [E8f](./E8f-declarative-behaviour.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) |
| Branch | `feat/e8g-form-data-and-flatten` |
| PR title | `feat(forms): form data interchange, fill, reset and flattening` |
| Size | L (400–800 of code; data fixtures and goldens beyond) |
| PR | — |

## Goal

Answers move in and out of a form, and a form can be filled from code, reset and flattened. `export_form_data` and `import_form_data` speak FDF, XFDF and Lamina PDF's JSON; `fill_form` takes the same JSON object; `reset_fields` restores defaults; `FlattenForm` draws fields into their pages and removes them, never touching a signature field.

## Context

- [E8 plan §2.1, §3 and §5](../../plans/editor/e8-form-filling.md#21-ops); decisions 2, 4 and 6 in [§10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions)
- [04 §3.6 FDF and XFDF](../../research/pdf/04-interactive-security.md#36-fdf-and-xfdf), [04 §2.5 Flattening](../../research/pdf/04-interactive-security.md#25-flattening)
- ISO 32000-2 §12.7.8 (FDF), ISO 19444-1 (XFDF), §12.7.6.3 (reset-form)
- E5d's `laminapdf-interchange` and E5e's `FlattenAnnots` (`flatten-widget`, shared appearance and matrix functions)

## Scope

**In scope**

- Form fields in E5d's XFDF and FDF readers and writers, matched by fully qualified name
- The JSON format and its schema (`fields` by fully qualified name), published with the docs
- `fill_form`, `import_form_data` and `reset_fields` as one transaction of `SetFieldValue` each, calculating once at the end; locked fields and unknown names listed; `xfa-not-updated` when the form has XFA
- `FlattenForm { fields, intent }` (class `Content`): appearances generated first where missing or stale, signature fields refused (`field-is-signature`) and left out of "all", `/CO` and `/XFA` cleaned (`xfa-removed`), tagged pages as E5e
- `fixtures/forms/data/`; the library example: fill an invoice form from JSON
- Façade and SDK: `fill_form`/`fillForm`, `import_form_data`/`importFormData`, `export_form_data`/`exportFormData`, `reset_fields`/`resetFields`, `flatten_form`/`flattenForm`

**Out of scope** (and which todo covers it)

- The Form data menu and dialogs → [E8k](E8k-form-data-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.11` import and export form data; `E.12` populate fields programmatically; `E.13` reset form fields; `E.14` flatten forms
- **Crates and packages:** `laminapdf-interchange`, `laminapdf-model`, `laminapdf-content`, `laminapdf-ops`, the façade and bindings
- **`Op` variants:** `FlattenForm` (class `Content`, one class per op, ADR 0006 §1); import, fill and reset are transactions of `SetFieldValue` (no `ImportFormData` op)
- **`SaveConstraint`:** `FormFill` transactions as E8c; `FlattenForm` a violation under any certification and on a locked field, append-only with approval signatures
- **Web vs desktop:** identical (core)
- **Server features:** none
- **Capability-matrix rows to flip:** `E.11`, `E.12`, `E.13`, `E.14` → `round-trip` (core, pdf)

## Tasks

- [ ] Fields in XFDF and FDF; the JSON reader and writer within `Limits`
- [ ] `fill_form`, `import_form_data`, `export_form_data`, `reset_fields`
- [ ] `FlattenForm` over E5e's functions and the append API
- [ ] The example and its goldens

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7); frozen samples in `fixtures/ops/v1/`
- [ ] Apply then undo leaves the overlay empty and the next save byte-identical; redo re-applies the same bytes
- [ ] `would_violate`: the fill transaction allowed on P2, P3 and approval-only, a violation on P1 and on locked fields; `FlattenForm` a violation on P1, P2, P3 and on a locked field
- [ ] Export → import into a blank copy → export gives the same file, in each format
- [ ] Flattened forms render as filled within E4g's threshold and keep only signature fields; on an approval-signed file, `flatten_form` over every field keeps the signature valid (pyHanko) after the incremental save
- [ ] An XFDF with a DTD is refused

## Post-merge checks

None.

## Tests

- Golden data files; render diffs; the round-trip suite's cases
- The `fdf` and `xfdf` fuzz targets gain fields; a `form_json` target

## Docs to update

- API docs: form data; example: fill a form from JSON
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/errors/flatten-widget.md`: point at `flatten_form`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- E5d hasn't started; if it slips, this todo waits. Splitting `FlattenForm` into its own todo is the fallback if the PR passes ~1,500 lines

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
