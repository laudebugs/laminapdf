# E16d — Colour conversion, ICC and output intents, spot colours and overprint preview, page boxes for print

| Field | Value |
| --- | --- |
| ID | `E16d` |
| Status | `ready` |
| Plan | [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E2f](./E2f-page-geometry.md), [E7c](./E7c-content-ir.md), [E16a](./E16a-optimisation-print-feature-plan.md) |
| Blocks | [E16e](./E16e-preflight-and-conversion.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e16d-colour` |
| PR title | `feat(colour): colour conversion, output intents and overprint preview` |
| Size | L (600–1,500) |
| PR | — |

## Goal

A colour module: convert content and images between colour spaces with ICC transforms (engine per E16a), set/replace output intents, inspect Separation/DeviceN spot colours, overprint preview in the renderer, and print-oriented bleed/trim editing (UI over E2f's boxes).

## Context

- [02 §6 Color](../../research/pdf/02-imaging-model.md#6-color)

## Scope

**In scope**

- CMS integration
- `ConvertColours`, `SetOutputIntent` ops
- Separations panel and overprint preview
- Bleed/trim UI

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `K.08` convert colour spaces; `K.09` ICC profiles and output intents; `K.10` bleed, trim, crop; `K.14` spot colours and overprint
- **Crates and packages:** new colour module (crate decided in E16a), `lamina-content`, `lamina-render`
- **`SaveConstraint`:** `RequiresRewrite` for optimisation (full rewrite); signed files → `Conflict`, warned and overridden only explicitly; output intents alone are incremental
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `K.08` → `round-trip` (core, pdf, web, desktop); `K.09` → `round-trip` (core, pdf, web, desktop); `K.10` → `round-trip` (core, pdf, web, desktop); `K.14` → `read` (core, pdf, web, desktop)

## Tasks

- [ ] CMS
- [ ] Ops
- [ ] Preview
- [ ] UI

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Converted fixtures match reference conversions within ΔE thresholds

## Post-merge checks

None.

## Tests

- Colour fixtures

## Docs to update

- User help
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
