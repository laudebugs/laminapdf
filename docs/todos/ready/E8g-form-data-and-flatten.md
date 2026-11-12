# E8g — Form data import/export (FDF, XFDF, JSON), reset, programmatic fill and flatten

| Field | Value |
| --- | --- |
| ID | `E8g` |
| Status | `ready` |
| Plan | [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E2e](../done/E2e-content-append-api.md), [E5d](./E5d-xfdf-fdf.md), [E8d](./E8d-field-appearances.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e8g-form-data-and-flatten` |
| PR title | `feat(forms): form data interchange, reset and flattening` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Import/export field values as FDF, XFDF and JSON; reset form; a library API to populate fields programmatically (`form.fill({...})`) with appearance generation; and flatten forms (stamp widget appearances into content via the append API, remove fields).

## Context

- **Design (Paper):** [P03 Form data](../../design/paper/forms.md#p03-form-data). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §3.6 FDF and XFDF](../../research/pdf/04-interactive-security.md#36-fdf-and-xfdf)
- [04 §2.5 Flattening](../../research/pdf/04-interactive-security.md#25-flattening)

## Scope

**In scope**

- Interchange readers/writers (shared with E5d)
- JSON schema per E8b
- `FlattenForm { fields | all }` op. Signature fields (`/FT /Sig`) and their widgets are **never** flattened: `all` means every non-signature terminal field, the signature fields stay in `/AcroForm /Fields` (with `/SigFlags` kept), and an explicit `fields` list naming a signature field is refused with `OpError::SignatureField`. Flattening a signed signature's widget would leave a visible signature with nothing to validate and, in a signed document, break it
- UI menu items

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.11` import and export form data; `E.12` populate fields programmatically; `E.13` reset form fields; `E.14` flatten forms
- **Crates and packages:** `laminapdf-model`, `laminapdf-content`, `laminapdf-ops`
- **`Op` variants:** `ImportFormData` (class `FormFill`), `FlattenForm` (class `Content`: one class per op, ADR 0006 §1; never `FormFill`, because it removes fields and rewrites page content rather than setting values, so it must not pass as form filling under DocMDP P2/P3)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `FormFill` is allowed under DocMDP P2 and P3 and under approval signatures, except for fields locked by FieldMDP (`/Lock`) or by a signature's field-lock dictionary → blocked with the locking signature named. Flattening is a `Content` change: under a certification signature a violation at every DocMDP level, saved only through the override; with approval signatures only `AppendOnly`, except flattening a field locked by FieldMDP/`/Lock`, which is a violation
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `E.11` → `round-trip` (core, pdf, web, desktop); `E.12` → `round-trip` (core, pdf, web, desktop); `E.13` → `round-trip` (core, pdf, web, desktop); `E.14` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Interchange: FDF/XFDF readers and writers (shared with E5d)
- [ ] JSON: schema and `form.fill({...})` per E8b
- [ ] Flatten: `FlattenForm` stamping widget appearances via the E2e append API, then removing non-signature fields and widgets; signature fields excluded
- [ ] Library examples (fill an invoice form)

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug. Expected: `ImportFormData` (`FormFill`) allowed on P2, P3 and approval-only, violation on P1 and on locked fields; `FlattenForm` (`Content`) violation on P1, P2 and P3, and on the FieldMDP-lock fixture when it flattens a locked field
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Export → import into a blank copy reproduces values; flattened forms render identically and have no `/AcroForm` fields left except signature fields (`/FT /Sig`), which are kept with their widgets; on a document with an approval signature, `FlattenForm { all }` keeps the signature field and the signature still validates (EU DSS / pyHanko) after the incremental save

## Post-merge checks

None.

## Tests

- Golden data files
- Render diffs

## Docs to update

- API docs: form data
- Example: fill a form from JSON
- [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
