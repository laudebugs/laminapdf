# E8b — Form filling and data: feature plan

| Field | Value |
| --- | --- |
| ID | `E8b` |
| Status | `ready` |
| Plan | [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E5a](./E5a-review-stamps-interchange-feature-plan.md), [E8a](./E8a-adr-0021-javascript-policy.md) |
| Blocks | [E8c](./E8c-acroform-field-model.md), [E9a](./E9a-form-authoring-feature-plan.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e8b-form-filling-feature-plan` |
| PR title | `docs(editor): E8 feature plan (form filling and data)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E8 ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)): filling every AcroForm field type with generated appearances, tab order, required/read-only, declarative validation/calculation/hide, FDF/XFDF/JSON data, reset, programmatic population, flattening, submit (off by default), and XFA detection. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)
- [04 §3 Forms](../../research/pdf/04-interactive-security.md#3-forms)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e8-form-filling.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E8 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01` text fields (single/multi-line); `E.02` checkboxes and radio buttons; `E.03` dropdowns and list boxes; `E.04` push buttons; `E.05` date/formatted input; `E.06` required and read-only; `E.07` input validation; `E.08` automatic calculations; `E.09` tab order; `E.10` conditional behaviour; `E.11` import and export form data; `E.12` populate fields programmatically; `E.13` reset form fields; `E.14` flatten forms; `E.16` submit to external endpoints; `E.17` XFA forms (cross-cutting)
- **Crates and packages:** (planned for the round) `laminapdf-model` AcroForm, field appearance generation, widgets as DOM overlays
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) incremental; filling allowed without certification and under DocMDP P2/P3 (not P1), blocked for fields locked by FieldMDP/`/Lock`
- **Web vs desktop:** identical
- **Server features:** none (SubmitForm off by default, confirmed per submit)
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e8-form-filling.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`E.01`, `E.02`, `E.03`, `E.04`, `E.05`, `E.06`, `E.07`, `E.08`, `E.09`, `E.10`, `E.11`, `E.12`, `E.13`, `E.14`, `E.16`, `E.17`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaPdfId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `laminapdf` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E8"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E8c onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E8"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: `/NeedAppearances true` files: regenerate all appearances on open (changes the file) or only on edit?
- Open question: JSON form-data schema: our own, or match an existing one (e.g. pdf-lib/Instant JSON)?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
