# E4a — Markup and drawing annotations: feature plan

| Field | Value |
| --- | --- |
| ID | `E4a` |
| Status | `ready` |
| Plan | [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E2e](./E2e-content-append-api.md), [E3a](../done/E3a-navigation-attachments-inspection-feature-plan.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e4a-markup-and-drawing-annotations-feature-plan` |
| PR title | `docs(editor): E4 feature plan (markup and drawing annotations)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E4 ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)): markup (highlight, underline, strike-through, squiggly), notes, caret corrections, ink, shapes/lines/arrows, callouts and text boxes, with appearance-stream generation for every subtype and overlay rendering from lamina's model until save. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)
- [04 §2 Annotations](../../research/pdf/04-interactive-security.md#2-annotations-125)
- [04 §2.4 Why appearance streams matter](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter): lamina always writes `/AP` for what it creates ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 3)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e4-markup-and-drawing-annotations.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E4 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.01` highlight / underline / strike through; `D.02` sticky notes and text comments; `D.03` text-correction annotations; `D.04` freehand ink; `D.05` shapes, lines, arrows; `D.06` callouts and text boxes; `D.16` generate appearance streams (cross-cutting); `B.06` arrows, paths, freehand drawing
- **Crates and packages:** (planned for the round) `laminapdf-model` annotations (`/NM` from `LaminaPdfId`), appearance generator in `laminapdf-build`, overlay rendering in `@laminapdf/viewer`
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) incremental; signed files: annotations only under DocMDP P3 or without certification, otherwise warn
- **Web vs desktop:** identical; stylus pressure for ink where reported
- **Server features:** none
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e4-markup-and-drawing-annotations.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`D.01`, `D.02`, `D.03`, `D.04`, `D.05`, `D.06`, `D.16`, `B.06`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaPdfId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `laminapdf` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E4"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E4b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E4"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Rich text (`/RC`) in FreeText: which XHTML subset do we write, and do we render `/RC` or only `/DA`?
- Open question: Ink smoothing: store raw points or smoothed Béziers in `/InkList` (PDF 2.0 `/Path`)?
- Open question: Do edited annotations render from lamina's overlay layer only (hide PDFium's render of that annotation), and how is the switch-over after save done without flicker?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
