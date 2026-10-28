# E3a — Navigation, attachments, inspection and extraction: feature plan

| Field | Value |
| --- | --- |
| ID | `E3a` |
| Status | `ready` |
| Plan | [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2a](./E2a-page-organisation-feature-plan.md) |
| Blocks | [E3b](./E3b-bookmarks-and-destinations.md), [E3c](./E3c-links.md), [E3d](./E3d-initial-view-and-threads.md), [E3e](./E3e-xmp-editor.md), [E3f](./E3f-attachments-and-associated-files.md), [E3g](./E3g-inspectors.md), [E3h](./E3h-images-to-pdf-and-page-export.md), [E3i](./E3i-extraction.md), [E4a](./E4a-markup-and-drawing-annotations-feature-plan.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e3a-navigation-attachments-inspection-feature-plan` |
| PR title | `docs(editor): E3 feature plan (navigation, attachments, inspection and extraction)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E3 ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)): bookmark/link/destination editing, TOC generation, initial view, XMP editor, attachments and associated files, inspectors and object browser, and the extraction/export features (images → PDF, page images, text/images/links/attachments). It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e3-navigation-attachments-inspection.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E3 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.01` create/edit bookmarks; nested trees; `I.02` internal page links; `I.03` external hyperlinks; `I.04` named destinations; `I.05` clickable table of contents; `I.06` initial view and zoom; page display mode; `I.09` article threads; `I.11` outline and navigation preferences; `J.04` manage XMP; `J.05` attach / extract / delete embedded files; `J.06` portfolios / collections; `J.07` inspect version and conformance; `J.08` identify fonts and colour profiles; `J.09` inspect internal objects, streams, references; `A.09` add hyperlinks to text; `B.03` extract original embedded images; `H.02` images → PDF; `H.06` pages → PNG/JPEG/TIFF; `H.07` extract text and images; `H.08` extract hyperlinks and bookmarks; `H.09` extract attachments; `H.11` render pages to images; `M.09` associated files and structured metadata
- **Crates and packages:** (planned for the round) `lamina-model` (outline, names, embedded files, XMP schemas), `lamina-build` (images → PDF), `lamina-render` (export), `lamina-text` (extraction)
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) `Free`; on signed files, outline, destination, attachment and XMP changes (class `Document`/`Metadata`) are DocMDP violations under any certification and gated, links (`Annotation`) are allowed under P3; with approval signatures only, all are `AppendOnly` incremental updates
- **Web vs desktop:** object inspector on both; desktop exports to a folder, web zips
- **Server features:** none
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e3-navigation-attachments-inspection.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`I.01`, `I.02`, `I.03`, `I.04`, `I.05`, `I.06`, `I.09`, `I.11`, `J.04`, `J.05`, `J.06`, `J.07`, `J.08`, `J.09`, `A.09`, `B.03`, `H.02`, `H.06`, `H.07`, `H.08`, `H.09`, `H.11`, `M.09`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `lamina` and `@lamina/pdf`), and the UI tools/panels in `@lamina/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E3"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E3b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E3"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: TOC generation: from bookmarks only, or also from headings in the struct tree / font-size heuristics?
- Open question: TIFF export: which encoder (licence, wasm size), multi-page TIFF on web?
- Open question: Object inspector: read-only only, or raw object editing for power users (dangerous; would need its own op class)?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
