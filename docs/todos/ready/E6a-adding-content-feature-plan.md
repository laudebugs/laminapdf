# E6a — Adding content to pages: feature plan

| Field | Value |
| --- | --- |
| ID | `E6a` |
| Status | `ready` |
| Plan | [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E2e](../done/E2e-content-append-api.md), [E5a](../done/E5a-review-stamps-interchange-feature-plan.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e6a-adding-content-feature-plan` |
| PR title | `docs(editor): E6 feature plan (adding content to pages)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E6 ([full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages)): adding text, symbols, images, shapes, logos and stamps as page content via Form XObjects, watermarks and backgrounds, headers/footers/page numbers and Bates numbering, and measuring touched-page reload time. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)
- [05 §3.3 Stamping and watermarking](../../research/pdf/05-creating-pdfs-from-code.md#33-stamping-and-watermarking)
- [architecture §6](../../plans/architecture.md#6-risks): re-render latency risk

## Scope

**In scope**

- The feature plan `docs/plans/editor/e6-adding-content.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E6 implementation todos (split, merge, dependency fixes)
- The `AppendContent` op over [E2e](../done/E2e-content-append-api.md)'s `PageContentWriter`: its payload, including how its form's images and fonts are carried (blob store `BlobId`s from [E2c](E2c-cross-document-pages.md) or a form spec), and which E6 todo implements it ([E2 plan §8](../../plans/editor/e2-page-organisation.md#8-todos) moved it here from E2e)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.01` add text anywhere on a page; `A.12` insert special characters and symbols; `B.01` insert, delete, replace images; `B.02` resize, rotate, crop, reposition; `B.04` adjust transparency/opacity; `B.05` draw lines, rectangles, circles, polygons; `B.09` add logos, icons, stamps; `B.10` background images and watermarks; `B.12` transformations on graphical objects; `C.14` labels, page numbers, headers/footers
- **Crates and packages:** (planned for the round) `laminapdf-content` append API, `laminapdf-build` surfaces on existing pages, `laminapdf-text`
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) incremental on `Free`; content changes are DocMDP violations under any certification (gated, override names the signatures); with approval signatures only they are `AppendOnly` incremental updates
- **Web vs desktop:** desktop uses system fonts (edge discovery, bytes into the core); web bundled or user-supplied fonts; touched-page reload measured
- **Server features:** none
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e6-adding-content.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`A.01`, `A.12`, `B.01`, `B.02`, `B.04`, `B.05`, `B.09`, `B.10`, `B.12`, `C.14`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaPdfId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `laminapdf` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E6"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E6b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E6"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Font sources on web: bundled set (which fonts, licence OFL) vs user-supplied only?
- Open question: Are added objects editable after save (re-selectable via a marked-content tag like `/LaminaPdfObj` in marked content properties) or only before save?
- Open question: Watermarks: content-stream form with OCG and `/Artifact`, or Watermark annotation, or both as options?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
