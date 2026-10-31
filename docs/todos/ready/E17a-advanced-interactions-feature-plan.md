# E17a — Advanced interactions: feature plan

| Field | Value |
| --- | --- |
| ID | `E17a` |
| Status | `ready` |
| Plan | [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E8a](./E8a-adr-0021-javascript-policy.md), [E16a](./E16a-optimisation-print-feature-plan.md) |
| Blocks | [E17b](./E17b-action-editor.md), [E17c](./E17c-layers.md), [E17d](./E17d-media.md), [E17e](./E17e-measurement.md), [E18a](./E18a-adr-0012-collaboration.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e17a-advanced-interactions-feature-plan` |
| PR title | `docs(editor): E17 feature plan (advanced interactions)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E17 ([full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions)): action editor (JS authoring per ADR 0021, buttons, events, open actions), layers (OCGs), media playback, measurement and geospatial (read), and 3D detection with poster fallback. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e17-advanced-interactions.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E17 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `M.01` add/edit JavaScript actions; `M.02` interactive buttons; `M.03` event-triggered actions; `M.04` manage/show/hide layers; `M.05` embed audio and video; `M.06` embed 3D models; `M.07` measurement tools and scales; `M.08` geospatial coordinates; `M.10` document open actions; `M.11` rich-media annotations; `I.08` optional content groups (layers); `D.09` audio annotations
- **Crates and packages:** (planned for the round) `lamina-model` actions, OCGs, measurement dictionaries
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) incremental on `Free`
- **Web vs desktop:** media playback depends on platform codecs; JavaScript never runs outside the ADR 0021 sandbox
- **Server features:** none
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e17-advanced-interactions.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`M.01`, `M.02`, `M.03`, `M.04`, `M.05`, `M.06`, `M.07`, `M.08`, `M.10`, `M.11`, `I.08`, `D.09`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `lamina` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E17"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E17b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E17"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Media playback on web: which codecs/containers are allowed (browser-native only)?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
