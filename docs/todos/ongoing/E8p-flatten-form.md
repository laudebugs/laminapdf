# E8p — Flatten forms: `FlattenForm` draws fields into their pages

| Field | Value |
| --- | --- |
| ID | `E8p` |
| Status | `ongoing` |
| Plan | [E8 plan §2.1](../../plans/editor/e8-form-filling.md#21-ops) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E2e](../done/E2e-content-append-api.md), [E5e](../done/E5e-flatten-and-summary.md), [E8d](../done/E8d-field-appearances.md), [E8g](../done/E8g-form-data-and-flatten.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) |
| Branch | `feat/e8p-flatten-form` |
| PR title | `feat(forms): flatten form fields into their pages` |
| Size | M (300–600 of code; render goldens beyond) |
| PR | — |

## Goal

A form can be flattened: `FlattenForm` draws each listed field's widgets into their pages and removes the fields, never touching a signature field, so the answers stay on the page and can no longer be changed.

## Context

- Split from [E8g](../done/E8g-form-data-and-flatten.md), the fallback its risks section named, after E8g's form data, fill and reset passed the size limit.
- [E8 plan §2.1, `FlattenForm`](../../plans/editor/e8-form-filling.md#21-ops), §3 and §5; decision 6 in [§10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions)
- [04 §2.5 Flattening](../../research/pdf/04-interactive-security.md#25-flattening); ISO 32000-2 §12.5.5, §12.7
- E5e's `FlattenAnnots` (`crates/laminapdf-ops/src/annot/flatten.rs`): `prepare` and `write_page` draw an appearance into its page, and `flatten-widget` refuses widgets until this op exists

## Scope

**In scope**

- `FlattenForm { fields, intent }` (class `Content`): appearances generated first where missing, or stale while `/NeedAppearances` is true; widgets hidden for the intent removed without being drawn; signature fields refused (`field-is-signature`) and left out of "all"; `/CO` entries removed; `/NeedAppearances`, `/CO` and `/XFA` removed when only signature fields are left, and `/XFA` removed with `xfa-removed` when other fields are left; tagged pages as E5e (artifacts, the `Form` element and its `OBJR` removed, `flatten-untagged` on PDF/UA-1)
- `Op::form_field` widened so the planner checks every field `FlattenForm` names against field locks
- Façade and SDK: `flatten_form`/`flattenForm`, whose "all" leaves out signature fields and fields a lock covers
- `docs/errors/flatten-widget.md`: point at `flatten_form`

**Out of scope** (and which todo covers it)

- The Flatten dialog → [E8k](../ready/E8k-form-data-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.14` flatten forms
- **Crates and packages:** `laminapdf-ops`, `laminapdf-content`, the façade and bindings
- **`Op` variants:** `FlattenForm` (class `Content`, one class per op, ADR 0006 §1)
- **`SaveConstraint`:** a violation under any certification and on a locked field, append-only with approval signatures
- **Web vs desktop:** identical (core)
- **Server features:** none
- **Capability-matrix rows to flip:** `E.14` → `round-trip` (core, pdf)

## Tasks

- [ ] `FlattenForm` over E5e's functions and the append API, with E8d's builders for missing appearances
- [ ] The planner's field-level check over every field the op names
- [ ] The façade, the bindings and the SDK; a frozen sample in `fixtures/ops/v1/`
- [ ] Render goldens and the round-trip suite's case

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7); frozen sample in `fixtures/ops/v1/`
- [ ] Apply then undo leaves the overlay empty and the next save byte-identical; redo re-applies the same bytes
- [ ] `would_violate`: `FlattenForm` a violation on P1, P2, P3 and on a locked field
- [ ] Flattened forms render as filled within E4g's threshold and keep only signature fields; on an approval-signed file, `flatten_form` over every field keeps the signature valid (pyHanko) after the incremental save

## Post-merge checks

None.

## Tests

- Render diffs of flattened forms against their filled originals; the round-trip suite's case
- The count of tagged forms in the corpus that flattening drops answers from, recorded for E15a (E8 plan §10, question 6)

## Docs to update

- API docs: flatten a form
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)
- `docs/errors/flatten-widget.md`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Fields whose widgets share an appearance stream: drawing each widget's own `/AS` entry is enough, as for annotations.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
