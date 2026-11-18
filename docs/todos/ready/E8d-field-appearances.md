# E8d — Value appearances for every field type

| Field | Value |
| --- | --- |
| ID | `E8d` |
| Status | `ready` |
| Plan | [E8 plan §2.2](../../plans/editor/e8-form-filling.md#22-fields-appearances-and-needappearances) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E4e](../done/E4e-freetext-callouts.md), [E8c](./E8c-acroform-field-model.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e8d-field-appearances` |
| PR title | `feat(build): appearance streams for form field values` |
| Size | L (400–800 of code; goldens and fixtures beyond) |
| PR | — |

## Goal

`SetFieldValue` regenerates the appearance of every widget it changes, so a filled form looks the same in pdf.js, PDFium and Acrobat. Text fields lay out their value from `/DA` (auto-size, multiline, comb, quadding, rotation, password); check boxes and radio buttons keep their on-states; combo and list boxes draw their selection. Opening a file never changes it, whatever `/NeedAppearances` says.

## Context

- [E8 plan §2.2 and §3](../../plans/editor/e8-form-filling.md#22-fields-appearances-and-needappearances), decision 1 in [§10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions)
- [04 §3.5 Filling forms: generating appearances](../../research/pdf/04-interactive-security.md#35-filling-forms-generating-appearances-the-hard-part), [04 §2.4](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter)
- ISO 32000-2 §12.7.4.3 (variable text), §12.5.5 (appearance streams), §12.7.5 (field types)
- E4e's `/DA` parser and FreeText layout in `laminapdf-build/src/annot/appearance/freetext/`

## Scope

**In scope**

- Builders in `laminapdf-build/src/annot/appearance/field/`: text (auto-size, multiline, comb, `/Q`, `/MK /R`, password, `/Tx BMC … EMC`), check box and radio (an appearance only where none exists), combo and list box (`/TI`, highlighted rows)
- Font fallback: a bundled font subset, embedded and added to `/DR`; an embedded font on a file that claims PDF/A
- `appearance-needs-text` in the base module, as FreeText
- The `/NeedAppearances` rule: only changed fields regenerate; the flag is never set or cleared
- `fixtures/forms/text-layout.pdf`, `need-appearances.pdf`, `pdfa-form.pdf`; goldens in `fixtures/golden/forms/`

**Out of scope** (and which todo covers it)

- Formatted values (`$1,234.50`) → [E8f](E8f-declarative-behaviour.md), which passes the formatted text to these builders
- The Read-mode note for `/NeedAppearances` → [E8e](E8e-field-widgets-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01` text fields; `E.02` check boxes and radio buttons; `E.03` dropdowns and list boxes
- **Crates and packages:** `laminapdf-build`, `laminapdf-text`, `laminapdf-ops`
- **`Op` variants:** none new; appearance regeneration inside `SetFieldValue`
- **`SaveConstraint`:** as E8c; regeneration touches only the changed field's widgets, so the class stays `FormFill`
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `E.01`, `E.02`, `E.03` → `write` (core, pdf), their E8 target; they reach `round-trip` once, in [E9b](E9b-field-creation-ops.md), when they can also be created

## Tasks

- [ ] Builders and the `Appearances` hook for fields
- [ ] Auto-size and comb algorithms
- [ ] Font fallback and `/DR` updates
- [ ] Goldens; render comparisons with pdf.js and PDFium

## Acceptance criteria

- [ ] A test proves regeneration writes nothing but the changed field's widgets and the fonts it adds to `/DR`, and `would_violate` stays `FormFill`'s verdict on every signed fixture
- [ ] Round-trip: open → `SetFieldValue` → save → reopen in Lamina PDF, pdf.js and PDFium keeps the value and everything else; `qpdf --check` is clean
- [ ] Filled values render within E4g's threshold in pdf.js and PDFium; the comparison with Acrobat's references runs once the owner supplies them ([E8 plan §10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions), question 8)
- [ ] Non-Latin values render and extract correctly; veraPDF passes `pdfa-form.pdf` after a fill
- [ ] Opening `need-appearances.pdf` and saving with no op writes no new revision

## Post-merge checks

None.

## Tests

- Goldens per field type; render diffs; the round-trip suite's `setFieldValue` cases
- `ops_json` gains the op's values

## Docs to update

- API docs: filling forms
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Acrobat's auto-size and baseline rules are undocumented; pdf.js's and PDFium's are the references until Acrobat's files arrive
- The extract module's size: measure against ADR 0016 §6 and ask the owner before raising a budget

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
