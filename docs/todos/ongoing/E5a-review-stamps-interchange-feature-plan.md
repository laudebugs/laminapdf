# E5a — Review, stamps and interchange: feature plan

| Field | Value |
| --- | --- |
| ID | `E5a` |
| Status | `ongoing` |
| Plan | [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e5a-review-stamps-interchange-feature-plan` |
| PR title | `docs(editor): E5 feature plan (review, stamps and interchange)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E5 ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)): comment threads and review states, filter/search, custom stamps, file-attachment annotations, audio annotations (read), XFDF/FDF import/export, flattening via the E2 append API and comment summaries. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)
- [04 §2.3 replies and review state](../../research/pdf/04-interactive-security.md#23-markup-annotations-replies-review-state-12562)
- [04 §2.5 Flattening](../../research/pdf/04-interactive-security.md#25-flattening)
- [04 §3.6 FDF and XFDF](../../research/pdf/04-interactive-security.md#36-fdf-and-xfdf)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e5-review-stamps-interchange.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E5 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.07` custom stamps; `D.08` attach files as annotations; `D.09` audio annotations; `D.10` replies to comments; `D.11` status and review properties; `D.12` filter and search comments; `D.13` import and export annotation data; `D.14` flatten annotations into content; `D.15` delete or summarise annotations
- **Crates and packages:** (planned for the round) as E4 plus XFDF/FDF and flattening via the E2 append API
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) flattening is a content change → incremental on `Free`; gated under any certification (DocMDP violation at every level); `AppendOnly` with approval signatures only
- **Web vs desktop:** identical
- **Server features:** none (shared review is E18)
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e5-review-stamps-interchange.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`D.07`, `D.08`, `D.09`, `D.10`, `D.11`, `D.12`, `D.13`, `D.14`, `D.15`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaPdfId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `laminapdf` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E5"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E5b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E5"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Deleting a parent annotation: delete replies too, or re-parent them?
- Open question: Comment summary layout (Acrobat-like side-by-side vs list) and whether it is tagged PDF
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
