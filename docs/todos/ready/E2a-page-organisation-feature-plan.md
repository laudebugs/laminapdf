# E2a — Page organisation: feature plan

| Field | Value |
| --- | --- |
| ID | `E2a` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B9c](../done/B9c-text-extraction-bindings-and-differential.md), [E1a](./E1a-editing-foundation-feature-plan.md), [E1f](./E1f-metadata-ui-capability-page.md) |
| Blocks | [E2b](./E2b-page-structure-ops.md), [E2e](./E2e-content-append-api.md), [E3a](./E3a-navigation-attachments-inspection-feature-plan.md) |
| Dependency notes | [E1f](E1f-metadata-ui-capability-page.md) (closes round E1, so every later round inherits the edit session E1d and Save/banner E1e), [E1a](E1a-editing-foundation-feature-plan.md), `B9` ([pdf-library §B9](../../plans/pdf-library.md#b9-text-extraction): content reading and text extraction) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e2a-page-organisation-feature-plan` |
| PR title | `docs(editor): E2 feature plan (page organisation)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E2 ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)): page insert/delete/duplicate/reorder/rotate/reverse/replace, extract and export, merge and split, crop, resize, page boxes, orientation, page labels, and the `lamina-content` **append API** that resize, flattening (E5) and adding content (E6) use. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)
- [05 §3.1 Merge and split](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split): remapping outlines, destinations, fields, struct tree
- [full-editor §6](../../plans/full-editor.md#6-design-summary): the three steps of `lamina-content` (read, append, edit)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e2-page-organisation.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E2 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.01` insert blank pages; `C.02` insert pages from other PDFs; `C.03` delete and duplicate pages; `C.04` reorder pages; `C.05` extract selected pages; `C.06` merge multiple documents; `C.07` split by count, ranges, bookmarks; `C.08` rotate pages 90/180/270; `C.09` crop display area; `C.10` resize page dimensions; `C.11` modify margins and page boxes; `C.12` portrait ↔ landscape; `C.13` reverse page order; `C.14` labels, page numbers, headers/footers; `C.16` replace pages, preserving the rest; `H.10` export pages/ranges; `I.10` page labels
- **Crates and packages:** (planned for the round) `lamina-content` (append API, new here), `lamina-ops` page ops, `lamina-model` merge remapping
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) `Free` → incremental; under a certification signature page insertion/deletion/reordering is a DocMDP violation at every level (ADR 0007 §2) and is allowed only through the override that invalidates the named signatures; with approval signatures only it is `AppendOnly` (incremental), unless it changes a field locked by FieldMDP/`/Lock`
- **Web vs desktop:** identical; desktop merges a multi-file Finder/Explorer selection
- **Server features:** none
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e2-page-organisation.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`C.01`, `C.02`, `C.03`, `C.04`, `C.05`, `C.06`, `C.07`, `C.08`, `C.09`, `C.10`, `C.11`, `C.12`, `C.13`, `C.14`, `C.16`, `H.10`, `I.10`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `lamina` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E2"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E2b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E2](../../plans/full-editor.md#e2-page-organisation)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E2"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Page delete when outlines/links/named destinations point at the page: retarget to nearest page, drop, or keep dangling with a warning?
- Open question: Merge field-name clashes in AcroForm: rename with suffix, or merge same-named fields (same value)?
- Open question: Does "headers/footers" in C.14 stay in E6 (stamped content) with only labels here? (Plan: yes)
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
