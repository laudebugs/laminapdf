# E7k — Select and transform existing objects in the reader (A05)

| Field | Value |
| --- | --- |
| ID | `E7k` |
| Status | `ready` |
| Plan | [E7 plan](../../plans/editor/e7-editing-existing-content.md) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6e](./E6e-content-tools-ui.md), [E7d](./E7d-select-transform-existing.md) |
| ADRs | Follows ADR 0022, 0024 |
| Branch | `feat/e7k-select-transform-ui` |
| PR title | `feat(ui): select, move and restyle existing objects in the reader` |
| Size | L (600–1,500) |
| PR | — |

## Goal

A05 in the web and desktop readers. With E6e's Select tool, people select the images, paths and text blocks the file already had. Then they move, resize, rotate, crop, reorder or delete them through E7d's `EditContent`, with the inspectors, context menu and states A05 draws.

## Context

- **Design (Paper):** [A05 Select and transform existing objects](../../design/paper/add-edit-content.md#a05-select-and-transform-existing-objects). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E7 plan](../../plans/editor/e7-editing-existing-content.md) §2 and §4
- [E6 plan §4](../../plans/editor/e6-adding-content.md#4-crate-and-package-changes): Edit mode's tools and handles, which this reuses

## Scope

**In scope**

- Hover outline and selection of existing objects through `hit_test`, groups by marquee and Shift
- E6e's handles for move, resize and rotate on existing objects; Crop on images
- The context menu (Bring forward and the rest), and the inspectors "Image", "n shapes" and "Text block"
- A05's states: a shared image, an object Lamina PDF can't change safely (outlined, no handles, the reason), a signature's appearance (read-only)

**Out of scope** (and which todo covers it)

- Edit points, paint, clips and masks → [E7h](./E7h-vector-clipping-masks.md)
- Editing the text of a text block → [E7l](./E7l-inline-text-editor.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.07`, `B.01`, `B.02`, `B.08`, `B.12`
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`, `apps/web`, `apps/desktop`
- **`Op` variants:** none new (the reader drives the core todo's ops)
- **`SaveConstraint`:** as the core todo's: `Free` → incremental; a certified file becomes `Conflict` from the first edit, with the banner naming the certifier
- **Web vs desktop:** identical but for fonts on the computer (`systemFonts`, E6h)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.07`, `B.01`, `B.02`, `B.08`, `B.12` → `round-trip` (web, desktop)

## Tasks

- [ ] Selection and hover
- [ ] Handles and Crop
- [ ] Context menu and inspectors
- [ ] States
- [ ] e2e

## Acceptance criteria

- [ ] Every A05 control works with a pointer and from the keyboard, and the command palette reaches every action
- [ ] Checked in the built reader in a browser, as a user does it (CLAUDE.md), and on the desktop app
- [ ] The reader's startup scripts don't grow: everything loads with the editor's lazy chunk

## Post-merge checks

None.

## Tests

- Playwright: move, resize, rotate, crop, reorder and delete an existing image and shape on the fixtures, then save, reopen and check
- WebdriverIO: the same on each desktop OS

## Docs to update

- User help for the screens this adds
- [E7 plan](../../plans/editor/e7-editing-existing-content.md) §7: point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
