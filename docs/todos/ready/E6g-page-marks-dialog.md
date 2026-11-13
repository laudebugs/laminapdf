# E6g — The Page marks dialog and Manage page marks

| Field | Value |
| --- | --- |
| ID | `E6g` |
| Status | `ready` |
| Plan | [E6 plan §2.3](../../plans/editor/e6-adding-content.md#23-page-marks) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6d](./E6d-watermarks-headers-footers.md), [E6e](./E6e-content-tools-ui.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e6g-page-marks-dialog` |
| PR title | `feat(ui): the Page marks dialog for watermarks, headers, footers and Bates numbers` |
| Size | L (600–1,500) |
| PR | — |

## Goal

A03's Page marks dialog in the reader: the Watermark, Header & footer and Bates numbers tabs with a live preview, the overlap notice, and Manage page marks to edit or remove the marks Lamina PDF added. Split from [E6d](E6d-watermarks-headers-footers.md) by the [E6 plan](../../plans/editor/e6-adding-content.md#7-todos), so the ops and the dialog are one reviewable PR each.

## Context

- **Design (Paper):** [A03 Watermark, header and footer dialog](../../design/paper/add-edit-content.md#a03-watermark-header-and-footer-dialog). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E6 plan §2.3](../../plans/editor/e6-adding-content.md#23-page-marks): tokens, the overlap check, hideable marks

## Scope

**In scope**

- The dialog's three tabs, the preview with its page stepper, the page range with page labels, the invalid-range and empty states, progress on many pages, the certified-file footer note
- Tokens shown as `[Page]`, `[Pages]`, `[Page label]`, `[Date]`, `[File name]` and written as E6d's braces
- Manage page marks: the list of added marks, edit (title "Edit watermark", Update, Remove) and remove
- Commands `edit.add-watermark`, `edit.add-header-footer`, `edit.add-bates`, `edit.manage-page-marks`, in the Page marks menu and the palette

**Out of scope** (and which todo covers it)

- Bates numbering across files → [E14d](E14d-batch-runner.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `B.10` background images and watermarks; `C.14` labels, page numbers, headers/footers
- **Crates and packages:** `@laminapdf/ui`, `apps/web`, `apps/desktop`
- **`SaveConstraint`:** as E6d
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `B.10` → `round-trip` (web, desktop); `C.14` → `round-trip` (web, desktop)

## Tasks

- [ ] The dialog and its tabs
- [ ] The preview, drawn by the dialog over the page's rendered picture from the mark's settings (text in the chosen font through the browser, the image, colour, opacity and angle), so nothing is applied until the primary button; the dialog notes that the saved mark may differ slightly
- [ ] The overlap notice from E6d's check
- [ ] Manage page marks
- [ ] e2e

## Acceptance criteria

- [ ] Each tab adds its mark as one undo step, on web and desktop
- [ ] Editing and removing a mark from Manage page marks works after a save and reopen
- [ ] axe finds no serious violations in the dialog

## Post-merge checks

None.

## Tests

- Playwright (reader), WebdriverIO (desktop)

## Docs to update

- User help: page marks
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and its §10 to-dos
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- The browser's text rendering can differ from the embedded font's; the preview is close, not exact

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
