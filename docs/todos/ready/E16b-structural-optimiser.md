# E16b — Optimiser: unused objects, dedupe, object streams, linearisation

| Field | Value |
| --- | --- |
| ID | `E16b` |
| Status | `ready` |
| Plan | [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E16a](./E16a-optimisation-print-feature-plan.md) |
| Blocks | [E16c](./E16c-images-and-fonts.md), [E16g](./E16g-server-batch-optimise.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e16b-structural-optimiser` |
| PR title | `feat(save): structural optimisation and linearisation` |
| Size | L (600–1,500) |
| PR | — |

## Goal

"Save as optimised" options: garbage-collect unreachable objects, deduplicate identical streams/dicts, recompress streams with Flate, pack into object streams with xref streams, and linearise (hint tables) for fast web view.

## Context

- **Design (Paper):** [X09 Optimise](../../design/paper/convert-compare-produce.md#x09-optimise). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [01 §8 Linearization](../../research/pdf/01-file-structure.md#8-linearization-fast-web-view-annex-f)
- [ADR 0007](../../decisions/0007-save-planner.md) §5 full rewrite

## Scope

**In scope**

- `Optimise { options }` op (requires rewrite)
- Linearisation writer
- Size report

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `K.01` compress / reduce size; `K.04` remove unused objects; `K.05` deduplicate shared resources; `K.06` linearise for fast web view
- **Crates and packages:** `lamina-save`
- **`Op` variants:** `Optimise`
- **`SaveConstraint`:** `RequiresRewrite` for optimisation (full rewrite); signed files → `Conflict`, warned and overridden only explicitly
- **Web vs desktop:** identical; web enforces size limits from `Limits`
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `K.01` → `round-trip` (core, pdf, web, desktop); `K.04` → `round-trip` (core, pdf, web, desktop); `K.05` → `round-trip` (core, pdf, web, desktop); `K.06` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] GC
- [ ] Dedupe
- [ ] Object streams
- [ ] Linearisation
- [ ] Report

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] qpdf `--check-linearization` passes
- [ ] Corpus size reduction reported; renders unchanged (PDFium diff zero)

## Post-merge checks

None.

## Tests

- Corpus
- qpdf

## Docs to update

- User help: optimising
- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
