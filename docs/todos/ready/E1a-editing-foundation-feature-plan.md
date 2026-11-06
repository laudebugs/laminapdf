# E1a — Editing foundation: feature plan

| Field | Value |
| --- | --- |
| ID | `E1a` |
| Status | `ready` |
| Plan | [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B3c](../done/B3c-save-planner-and-roundtrip.md), [B5c](../done/B5c-lamina-facade-and-examples.md), [C7c](./C7c-performance-gates-and-launch.md), [D7c](./D7c-desktop-beta-release.md), [X2](../done/X2-capability-matrix.md) |
| Dependency notes | `C7` ([web-app §C7](../../plans/web-app.md#c7-accessibility-offline-and-launch): web reader v0.1), `D7` ([desktop-app §D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta): desktop beta), `B3` ([pdf-library §B3](../../plans/pdf-library.md#b3-ops-and-save): ops and save planner), `B5` ([pdf-library §B5](../../plans/pdf-library.md#b5-builder-and-the-laminapdf-crate): builder and `laminapdf` crate), `X2` (capability-matrix generator: manifest, test tagging, `cargo xtask capabilities`, CI gate; [architecture §7](../../plans/architecture.md#cross-cutting), [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix)) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e1a-editing-foundation-feature-plan` |
| PR title | `docs(editor): E1 feature plan (editing foundation)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E1 ([full-editor §E1](../../plans/full-editor.md#e1-editing-foundation)): the edit session, undo/redo UI, Save / Save as / Download copy, the `SaveConstraint` banner, the in-app capability matrix page and the first write ops (metadata J.01–J.03, J.10, Info and XMP in sync). It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)
- [04 §5 Metadata](../../research/pdf/04-interactive-security.md#5-metadata): Info ↔ XMP mapping, `xmp:MetadataDate`, `xmpMM:InstanceID`
- [roadmap: sequence](../../plans/roadmap.md#sequence-and-dependencies): the editor starts after both readers ship v0.1

## Scope

**In scope**

- The feature plan `docs/plans/editor/e1-editing-foundation.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E1 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.01` title, author, subject, keywords; `J.02` creation/modification dates; `J.03` producer and creator; `J.10` remove selected metadata
- **Crates and packages:** (planned for the round) `laminapdf-ops`, `laminapdf-save`, `laminapdf-model` (XMP), `@laminapdf/ui`, platform adapters
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) `Free` → incremental by default; approval-signed files `AppendOnly` (metadata edits as incremental updates); under a certification signature metadata edits are a DocMDP violation at every level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) and gated
- **Web vs desktop:** desktop saves in place and watches for external changes; web saves in place only with the File System Access API, otherwise downloads a copy
- **Server features:** none
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e1-editing-foundation.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`J.01`, `J.02`, `J.03`, `J.10`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaPdfId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `laminapdf` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E1"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E1b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E1"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: How is the interaction-mode state machine typed so later rounds add modes (annotate, form-fill, content-edit, redact) without touching existing ones?
- Open question: Which XMP library, if any (own minimal RDF/XML reader/writer vs a crate): licence and wasm size?
- Open question: PDF 2.0: which Info keys are kept (only `CreationDate`/`ModDate` per ISO 32000-2 §14.3.3) and what the UI shows for deprecated ones?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
