# E16a — Optimisation and print production: feature plan

| Field | Value |
| --- | --- |
| ID | `E16a` |
| Status | `ready` |
| Plan | [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E15a](./E15a-accessibility-feature-plan.md) |
| Blocks | [E16b](./E16b-structural-optimiser.md), [E16d](./E16d-colour.md), [E16f](./E16f-imposition.md), [E17a](./E17a-advanced-interactions-feature-plan.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e16a-optimisation-print-feature-plan` |
| PR title | `docs(editor): E16 feature plan (optimisation and print production)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E16 ([full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production)): the optimiser (compression, images, fonts, GC, dedupe, linearisation), colour management, output intents and overprint/spot preview, preflight rules as data, PDF/A and PDF/X conversion and validation, imposition/booklet/N-up, and optional server batch optimisation. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)
- [05 §3.4 Optimisation](../../research/pdf/05-creating-pdfs-from-code.md#34-optimisation-and-compression)
- [05 §3.5 Linearisation](../../research/pdf/05-creating-pdfs-from-code.md#35-linearisation-fast-web-view)
- [05 §3.7 Conversion to PDF/A](../../research/pdf/05-creating-pdfs-from-code.md#37-conversion-to-pdfa)
- [02 §6 Color](../../research/pdf/02-imaging-model.md#6-color)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e16-optimisation-print.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E16 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `K.01` compress / reduce size; `K.02` downsample and recompress images; `K.03` embed or subset fonts; `K.04` remove unused objects; `K.05` deduplicate shared resources; `K.06` linearise for fast web view; `K.07` flatten transparency; `K.08` convert colour spaces; `K.09` ICC profiles and output intents; `K.10` bleed, trim, crop; `K.11` print-ready PDFs / preflight; `K.12` validate PDF/A or PDF/X; `K.13` imposition and booklet; `K.14` spot colours and overprint; `C.15` booklet / N-up layouts; `H.12` convert to PDF/A, PDF/X, PDF/UA
- **Crates and packages:** (planned for the round) `lamina-save` (object streams, linearisation, GC), image codecs, colour module, preflight rules as data
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) `RequiresRewrite` for optimisation; signed files warned
- **Web vs desktop:** desktop recommended for large print files; web enforces `Limits`
- **Server features:** optional batch optimisation (ADR 0013)
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e16-optimisation-print.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`K.01`, `K.02`, `K.03`, `K.04`, `K.05`, `K.06`, `K.07`, `K.08`, `K.09`, `K.10`, `K.11`, `K.12`, `K.13`, `K.14`, `C.15`, `H.12`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `lamina` and `@laminapdf/pdf`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E16"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E16b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E16"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Colour management engine: lcms2 bindings (MIT, C) vs a pure-Rust CMS (qcms/moxcms) — wasm size and accuracy
- Open question: Transparency flattening: implement (hard) or rasterise regions with a warning?
- Open question: PDF/X validation without commercial tools: which rule set?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
