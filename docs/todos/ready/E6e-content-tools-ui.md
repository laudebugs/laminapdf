# E6e — Content insert tools and object handles

| Field | Value |
| --- | --- |
| ID | `E6e` |
| Status | `ready` |
| Plan | [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6b](./E6b-place-images-and-shapes.md), [E6c](./E6c-add-text.md) |
| Blocks | [E7d](./E7d-select-transform-existing.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e6e-content-tools-ui` |
| PR title | `feat(ui): content-edit tools for adding text, images and shapes` |
| Size | L (600–1,500) |
| PR | — |

## Goal

`contentEdit` mode tools for adding text, images, shapes and stamps, with selection handles, snapping, alignment guides and a properties panel for the objects lamina added.

## Context

- [full-editor §6](../../plans/full-editor.md#6-design-summary) interaction modes

## Scope

**In scope**

- Tools and handles
- Image insertion from picker/drop/paste
- Properties panel (fill, stroke, opacity, font)
- Keyboard nudging

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.01` add text anywhere on a page; `A.12` insert special characters and symbols; `B.01` insert, delete, replace images; `B.02` resize, rotate, crop, reposition; `B.04` adjust transparency/opacity; `B.05` draw lines, rectangles, circles, polygons; `B.09` add logos, icons, stamps
- **Crates and packages:** `@lamina/ui`, `@lamina/viewer`
- **`SaveConstraint`:** as E6b–E6c
- **Web vs desktop:** identical; desktop paste of images from the OS clipboard
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.01` → `round-trip` (web, desktop); `A.12` → `round-trip` (web, desktop); `B.01` → `write` (web, desktop); `B.02` → `write` (web, desktop); `B.04` → `round-trip` (web, desktop); `B.05` → `round-trip` (web, desktop); `B.09` → `round-trip` (web, desktop)

## Tasks

- [ ] Tools
- [ ] Handles and snapping
- [ ] Panel
- [ ] e2e

## Acceptance criteria

- [ ] e2e on both shells for every tool
- [ ] axe zero serious violations

## Post-merge checks

None.

## Tests

- Playwright, WebdriverIO

## Docs to update

- User help: adding content
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
