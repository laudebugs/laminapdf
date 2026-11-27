# E6e — Content insert tools and object handles

| Field | Value |
| --- | --- |
| ID | `E6e` |
| Status | `done` |
| Plan | [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6b](./E6b-place-images-and-shapes.md), [E6c](./E6c-add-text.md), [E6i](./E6i-bundled-fonts.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e6e-content-tools-ui` |
| PR title | `feat(ui): content-edit tools for adding text, images and shapes` |
| Size | L (600–1,500) |
| PR | [#840](https://github.com/laudebugs/laminapdf/pull/840) |

## Goal

`contentEdit` mode tools for adding text, images, shapes and stamps, with selection handles, snapping, alignment guides and a properties panel for the objects lamina added.

## Context

- **Design (Paper):** [A01 Edit mode: add text box](../../design/paper/add-edit-content.md#a01-edit-mode-add-text-box). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
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
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`
- **`SaveConstraint`:** as E6b–E6c
- **Web vs desktop:** identical in design; this PR builds the web reader. The desktop has its own engine client, so its Edit mode, the OS clipboard paste and the desktop columns of the rows below are [E6j](../ready/E6j-desktop-content-tools.md)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.01` → `round-trip` (web); `A.12` → `round-trip` (web); `B.01` → `write` (web); `B.02` → `write` (web); `B.04` → `round-trip` (web); `B.05` → `round-trip` (web); `B.09` → `round-trip` (web); the desktop columns are E6j's

## Tasks

- [x] Tools
- [x] Handles and snapping
- [x] Panel
- [x] e2e (the web reader; the desktop's are [E6j](../ready/E6j-desktop-content-tools.md))

## Acceptance criteria

- [x] e2e in the web reader for every tool (the desktop shell's is [E6j](../ready/E6j-desktop-content-tools.md))
- [x] axe zero serious violations (the inspector, the symbol picker and the toolbar, in the e2e)

## Post-merge checks

None.

## Tests

- Playwright in the web reader (`apps/web/e2e/content-edit.e2e.test.ts`); Node tests of the geometry, the ops and the inspector's reading of an object; WebdriverIO is E6j's

## Docs to update

- User help: adding content
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- A resized shape's outline scales with it, since only the matrix changes; a path change op would fix that. What the designs show and this PR leaves out is [#841](https://github.com/laudebugs/laminapdf/issues/841)

## Definition of done

- [x] Claimed: this work PR's first commit moved this file `ready/` → `ongoing/` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
