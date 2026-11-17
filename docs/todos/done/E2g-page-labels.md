# E2g — The page-setup dialogs: page labels, crop, resize, boxes and orientation

| Field | Value |
| --- | --- |
| ID | `E2g` |
| Status | `done` |
| Plan | [E2 page organisation §4](../../plans/editor/e2-page-organisation.md#4-crate-and-package-changes) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2h](./E2h-page-organiser-ui.md), [E2q](./E2q-page-labels-op.md), [E2s](./E2s-resize-and-orientation.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2g-page-labels` |
| PR title | `feat(ui): page-setup dialogs and the labels editor` |
| Size | L (400–800): five dialogs and their commands in `@laminapdf/ui`, the labels editor with its preview, and their Playwright tests; the ops are [E2f](../done/E2f-page-geometry.md)'s, [E2q](../done/E2q-page-labels-op.md)'s and [E2s](../done/E2s-resize-and-orientation.md)'s |
| PR | [#635](https://github.com/laudebugs/laminapdf/pull/635) |

## Goal

Give Organise's Page setup ▾ its dialogs: page labels (O10), and crop, resize, page boxes and orientation (O09), over [E2f](../done/E2f-page-geometry.md)'s, [E2s](../done/E2s-resize-and-orientation.md)'s and [E2q](../done/E2q-page-labels-op.md)'s ops, with each page's label shown under its thumbnail.

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

- The ops → [E2f](../done/E2f-page-geometry.md), [E2s](../done/E2s-resize-and-orientation.md), [E2q](../done/E2q-page-labels-op.md)
- The test that labels survive every page op, merge and extract → [E2i](../done/E2i-signed-gating-and-close.md)
- Headers, footers and page numbers stamped as content (the rest of `C.14`) → [E6d](../ready/E6d-watermarks-headers-footers.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.10` page labels, and `C.09`–`C.12` in the interface
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`Op` variants:** none of its own
- **`SaveConstraint`:** the dialogs show the strip and the confirmations of the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint)
- **Web vs desktop:** identical; the desktop column is [E2n](../done/E2n-desktop-organise.md)'s
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `I.10` → `round-trip` (web); `C.09`, `C.10`, `C.11`, `C.12` → `round-trip` (web). `C.14` is E6d's ([E2 plan §11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions))

## Tasks

- [x] The labels editor and the thumbnail captions
- [x] The O09 dialogs and their commands
- [x] Playwright tests tagged with capability IDs

## Acceptance criteria

- [x] Labels shown in pdf.js and PDFium match the editor's preview on 5 fixtures (roman, alpha, prefixed, restarts)
- [x] Playwright in Chromium, Firefox and WebKit, as a user: crop, resize, set boxes, change orientation and edit labels through the dialogs, save, reopen and check the page facts; the crop dialog shows that cropped content remains; axe reports no serious violations

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

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **Reads the dialogs fill in from.** The ops existed; reading the current values didn't. `laminapdf::Document::page_boxes(index)` returns `PageGeometry`: the five boxes in user space as readers use them, `OwnBoxes` (which of the bleed, trim and art boxes the page sets) and `user_unit`. `label_ranges()` returns the ranges as `PageLabelRange`s, which `SetPageLabels` takes back unchanged; `tests/page_setup.rs` writes each labels fixture's ranges back and finds nothing changed. WASM has `pageBoxes` and `labelRanges` in the extract and text modules only, so the base module stays as it was; the SDK has `doc.pageBoxes(index)` (`PageBoxesRead`) and `doc.labelRanges()`.
- **`@laminapdf/viewer`:** `client.pageBoxes(index)` and `client.labelRanges()`. They read from the worker's text-capable copy, which follows every edit, because the edited document may be open in the base module. An edit, undo, redo or replay that sets the labels returns every page with its new label (`EditResult.relabelled`), and the client and the reader's page list take it. No page is drawn again: a label draws nothing.
- **`@laminapdf/ui`** (`src/organise/`, in the `./editing` entry): `CropDialog`, `ResizeDialog`, `PageBoxesDialog` (`page-setup.tsx`, arithmetic in `geometry.ts`), `PageLabelsDialog` (`labels-dialog.tsx`, the model in `labels.ts`). There are six `pages.*` commands (`PAGE_SETUP_COMMANDS`) and the toolbar's Page setup ▾. `labels.ts` numbers pages as `laminapdf-model`'s `labels.rs` does, roman and letters included, and its tests check the preview against pdf.js and PDFium's labels for the five `fixtures/pages/set-labels/` files.
- **The reader** (`apps/web/src/page-setup.tsx`): the commands' handlers and the dialogs, over `OrganiseController.edit`, which asks `wouldViolate` first as the Organise commands do. The page menu has the six items too. Resize gives a page turned a quarter the paper with its sides swapped, so every page shows the size chosen.
- **Found in the browser and fixed:**
  - A length field's arrow-key handler swallowed Escape: React Aria stops a key a field handles, so the handler now passes every other key on.
  - The boxes dialog read the page's boxes again on every render, which would have wiped what the person typed. It now reads once each time the dialog opens.
  - The edited document can't read boxes in the base module, so the reads go to the text-capable copy.
- **Differences from O09 and O10** are under each screen's "As built (web, E2g)": crop is a dialog without drag handles, Apply to has no page range, and the crop notice leaves out Redact, which the reader doesn't have yet.
- **WebAssembly size:** the two reads add about 2.6 KB gzipped to the extract module, mostly reading code; crossing as flat values instead of records saved only 188 bytes and cost the SDK's JavaScript more. With main's comment threads (laudebugs/laminapdf#618) the module came to 1,227,634 bytes, over its 1,226,000 budget, and the owner raised the budget to 1,228,500 on 2026-11-15.
- **Startup JavaScript:** 282,534 bytes gzipped, under the 282,700 waiver. The dialogs load with the Organise mode; the growth is the viewer client's two reads and its handling of `relabelled`.
- **Tests:**
  - `apps/web/e2e/page-setup.e2e.test.ts` (Chromium here; Firefox and WebKit in CI). One test crops, resizes, sets a trim box, makes a page landscape and labels the pages, then saves and reads the file back with the SDK (C.09, C.10, C.11, C.12, I.10 `web round-trip`). The other runs axe in each dialog and checks the clash on one page.
  - `packages/ui/test/organise-labels.test.ts` covers the labels model.
  - The viewer's `edit.browser.test.ts` covers `relabelled` through an undo, and `pageBoxes` after a crop.
  - `crates/laminapdf/tests/page_setup.rs` covers the façade reads.
  - The gallery has the dialogs' states (`crop`, `crop-mixed`, `resize`, `page-boxes`, `page-labels`, `page-labels-clash`).
- **Not done here:** the desktop's Organise ([E2n](../done/E2n-desktop-organise.md)); dragging crop edges on the page; a page range in Apply to; the phone's sheets.

