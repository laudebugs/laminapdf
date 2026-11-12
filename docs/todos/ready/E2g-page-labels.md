# E2g — The page-setup dialogs: page labels, crop, resize, boxes and orientation

| Field | Value |
| --- | --- |
| ID | `E2g` |
| Status | `ready` |
| Plan | [E2 page organisation §4](../../plans/editor/e2-page-organisation.md#4-crate-and-package-changes) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2h](./E2h-page-organiser-ui.md), [E2q](./E2q-page-labels-op.md), [E2s](./E2s-resize-and-orientation.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2g-page-labels` |
| PR title | `feat(ui): page-setup dialogs and the labels editor` |
| Size | L (400–800): five dialogs and their commands in `@laminapdf/ui`, the labels editor with its preview, and their Playwright tests; the ops are [E2f](E2f-page-geometry.md)'s, [E2q](E2q-page-labels-op.md)'s and [E2s](E2s-resize-and-orientation.md)'s |
| PR | — |

## Goal

Give Organise's Page setup ▾ its dialogs: page labels (O10), and crop, resize, page boxes and orientation (O09), over [E2f](E2f-page-geometry.md)'s, [E2s](E2s-resize-and-orientation.md)'s and [E2q](E2q-page-labels-op.md)'s ops, with each page's label shown under its thumbnail.

## Context

- **Design (Paper):** [O09 Page geometry](../../design/paper/pages-navigation-extraction.md#o09-page-geometry), [O10 Page labels editor](../../design/paper/pages-navigation-extraction.md#o10-page-labels-editor). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E2 plan §4](../../plans/editor/e2-page-organisation.md#4-crate-and-package-changes): the interface's parts
- ISO 32000-2 §12.4.2 (page labels)

## Scope

**In scope**

- The labels editor dialog in `@laminapdf/ui` (a ranges table, a preview) and label captions on Organise's thumbnails
- The O09 dialogs: Crop… (margins, with the "content remains" warning from `CroppedContentRemains`), Resize…, Page boxes…, Make landscape and Make portrait
- Their commands: `pages.crop`, `pages.resize`, `pages.edit-boxes`, `pages.make-landscape`, `pages.make-portrait`, `pages.edit-labels`

**Out of scope** (and which todo covers it)

- The ops → [E2f](E2f-page-geometry.md), [E2s](E2s-resize-and-orientation.md), [E2q](E2q-page-labels-op.md)
- The test that labels survive every page op, merge and extract → [E2i](E2i-signed-gating-and-close.md)
- Headers, footers and page numbers stamped as content (the rest of `C.14`) → [E6d](E6d-watermarks-headers-footers.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.10` page labels, and `C.09`–`C.12` in the interface
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`Op` variants:** none of its own
- **`SaveConstraint`:** the dialogs show the strip and the confirmations of the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint)
- **Web vs desktop:** identical; the desktop column is [E2n](E2n-desktop-organise.md)'s
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `I.10` → `round-trip` (web); `C.09`, `C.10`, `C.11`, `C.12` → `round-trip` (web). `C.14` is E6d's ([E2 plan §11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions))

## Tasks

- [ ] The labels editor and the thumbnail captions
- [ ] The O09 dialogs and their commands
- [ ] Playwright tests tagged with capability IDs

## Acceptance criteria

- [ ] Labels shown in pdf.js and PDFium match the editor's preview on 5 fixtures (roman, alpha, prefixed, restarts)
- [ ] Playwright in Chromium, Firefox and WebKit, as a user: crop, resize, set boxes, change orientation and edit labels through the dialogs, save, reopen and check the page facts; the crop dialog shows that cropped content remains; axe reports no serious violations

## Post-merge checks

None.

## Tests

- Component tests for the dialogs
- Playwright `apps/web/e2e/page-setup.e2e.test.ts`

## Docs to update

- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
