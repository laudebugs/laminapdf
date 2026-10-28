# E15a — Accessibility: feature plan

| Field | Value |
| --- | --- |
| ID | `E15a` |
| Status | `ready` |
| Plan | [full-editor §E15](../../plans/full-editor.md#e15-accessibility) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [B8b](./B8b-conformance-profiles-and-verapdf.md), [E14a](./E14a-compare-revisions-batch-feature-plan.md) |
| Blocks | [E15b](./E15b-structure-tree-editing.md), [E16a](./E16a-optimisation-print-feature-plan.md) |
| Dependency notes | [E14a](E14a-compare-revisions-batch-feature-plan.md), `B8` ([pdf-library §B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa): tagged PDF, PDF/UA, PDF/A) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e15a-accessibility-feature-plan` |
| PR title | `docs(editor): E15 feature plan (accessibility)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E15 ([full-editor §E15](../../plans/full-editor.md#e15-accessibility)): structure-tree editing (tags, reading order, alt text, language, artifacts, form and link labels), Matterhorn machine checks with fix-ups, missing-font/Unicode checks, and PDF/UA output with veraPDF agreement. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E15](../../plans/full-editor.md#e15-accessibility): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)
- [03 §9 Tagged PDF](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149)
- [library B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e15-accessibility.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E15 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `L.01` add structure tags; `L.02` logical reading order; `L.03` alt text for images; `L.04` document language; `L.05` mark decorative artifacts; `L.06` accessible form labels; `L.07` accessible link descriptions; `L.08` check missing fonts / Unicode mappings; `L.09` run accessibility validation; `L.10` fix tagging and structure issues; `L.11` PDF/UA-compliant output; `I.07` define reading order; logical structure tags; `H.12` convert to PDF/A, PDF/X, PDF/UA
- **Crates and packages:** (planned for the round) structure-tree editing in `lamina-model`; checks shared with B8; veraPDF external
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) incremental on `Free`; tagging a certified file is a DocMDP violation at every level (gated, override names the signatures); with approval signatures only it is an `AppendOnly` incremental update
- **Web vs desktop:** identical; tag tree panel and reading-order overlay on both
- **Server features:** optional veraPDF Container (GPL, never linked)
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e15-accessibility.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`L.01`, `L.02`, `L.03`, `L.04`, `L.05`, `L.06`, `L.07`, `L.08`, `L.09`, `L.10`, `L.11`, `I.07`, `H.12`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `lamina` and `@lamina/pdf`), and the UI tools/panels in `@lamina/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E15"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E15b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E15](../../plans/full-editor.md#e15-accessibility)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E15"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E15](../../plans/full-editor.md#e15-accessibility) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Auto-tagging of untagged documents: heuristic layout analysis now, ML later, or out of scope?
- Open question: WTPDF and PDF/UA-2 targets in addition to PDF/UA-1?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
