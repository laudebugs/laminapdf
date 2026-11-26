# E2n — Desktop: the Organise mode over the engine's edit session

| Field | Value |
| --- | --- |
| ID | `E2n` |
| Status | `done` |
| Plan | [E2 page organisation §7](../../plans/editor/e2-page-organisation.md#7-differences-between-the-shells) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [D3a](./D3a-open-from-os.md), [E1i](./E1i-desktop-save-in-place.md), [E2g](./E2g-page-labels.md), [E2m](./E2m-document-dialogs.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e2n-desktop-organise` |
| PR title | `feat(desktop): the Organise mode, page setup and labels over the engine` |
| Size | L (600–900): the Organise mode shared with the web, the engine's planner check, the page-setup dialogs |
| PR | [#683](https://github.com/laudebugs/laminapdf/pull/683) |

## Goal

The first half of round E2's desktop part ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): an editor round's desktop part is its own todo). The desktop has no Organise mode yet: this todo brings it over the engine's edit session, with page setup, page labels and the signature checks that gate page edits. The second half, [E2u](../ongoing/E2u-desktop-sources-and-folders.md), adds what reads or writes other files (insert from a file, replace, merge, split to a folder, the multi-file open offer); the owner split the two on 2026-11-16, when the desktop turned out to need the whole Organise mode first.

## Context

- **Design (Paper):** [O06](../../design/paper/edit-session-organise.md#o06-organise-mode); O07 and O08 are [E2u](../ongoing/E2u-desktop-sources-and-folders.md)'s
- [E2 plan §5 and §7](../../plans/editor/e2-page-organisation.md#7-differences-between-the-shells)
- [E1h](../done/E1h-desktop-edit-session.md) and [E1i](../done/E1i-desktop-save-in-place.md): the engine's edit session and saving in place; [D3a](../done/D3a-open-from-os.md): open requests with several paths

## Scope

**In scope**

- The Organise mode on the desktop: the web's `OrganiseController`, toolbar, grid and dialogs over the engine's edit session, shared rather than copied
- The engine's edit session reports the new page order as the worker does ([E2l](../done/E2l-viewer-across-page-edits.md)), so the shared viewer follows page-list edits on the desktop
- `wouldViolate` through the engine's save planner (a protocol message), so the signed-page question and the certified-document banner work on the desktop
- The page-setup dialogs (crop, resize, boxes, orientation) and the page labels editor (E2g) over the engine
- The certification banner's Details from [E2t](../done/E2t-signed-file-choices.md) on the desktop
- WebdriverIO suites on macOS, Windows and Linux

**Out of scope** (and which todo covers it)

- Insert from a file, replace, merge, split and extract to a folder, the multi-file open offer: [E2u](../ongoing/E2u-desktop-sources-and-folders.md)
- The banner's Save a copy and Remove signature… on the desktop, which write a copy with a refused change: [E2u](../ongoing/E2u-desktop-sources-and-folders.md), with the rest of the desktop's file writing

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): the desktop column of `C.01`, `C.03`, `C.04`, `C.08`–`C.13` and `I.10`; the rest is [E2u](../ongoing/E2u-desktop-sources-and-folders.md)'s
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-engine`, `laminapdf-desktop-protocol`
- **`SaveConstraint`:** as the web; the banner and the named-signature dialog checked on the desktop here
- **Web vs desktop:** this todo is the desktop side
- **Server features:** none
- **Capability-matrix rows to flip:** `C.01`, `C.03`, `C.04`, `C.08`, `C.09`, `C.10`, `C.11`, `C.12`, `C.13`, `I.10` → `round-trip` (desktop); the rest of round E2's desktop column is [E2u](../ongoing/E2u-desktop-sources-and-folders.md)'s

## Tasks

- [x] The Organise mode on the desktop, sharing the web's controller
- [x] Page order from the engine's session
- [x] `wouldViolate` through the engine's planner
- [x] Page setup and page labels over the engine
- [x] WebdriverIO specs `apps/desktop/e2e/specs/organise.e2e.ts`, tagged with capability IDs

## Acceptance criteria

- [x] WebdriverIO on three systems: reorder, delete, insert blank pages, rotate, reverse, a crop and a page-label edit done through the interface, saved in place, reopened and checked
- [x] On the approval-signed fixture, turning the signature's page asks first; on a certified one, the change is refused with the banner (both fixtures have one page, which can't be deleted, so the scenarios turn it)

## As built

- **Engine reads (protocol 10):** `ToEngine::Query { WouldViolate { log } | PageBoxes { index } | LabelRanges }`, answered by `FromEngine::Answer` with JSON in `@laminapdf/sdk`'s shapes (`laminapdf-desktop-engine/src/query.rs`), behind the commands `would_violate`, `page_boxes` and `label_ranges`, each with its own permission.
- **One Organise mode:** the controller, the mode's components and Page setup's actions moved from `apps/web` into `@laminapdf/ui/editing` (`organise/mode.tsx`, `organise/setup-actions.tsx`), behind the `OrganiseEngine`, `OrganiseViewer` and `OrganiseCopies` shapes. The web passes its dialogs across files through `dialogs`, and saving's copies through `copies`; the desktop passes no copies, so its banner has no Save a copy.
- **Labels on the desktop:** the engine reports pages without labels, so `DesktopOrganiseEngine` works them out from the label ranges (`pageLabels`) and reads them again after each edit.
- **The window's modes:** the desktop now makes its shell's layout and modes (`useShell`), with Read and Organise in the top bar while a document shows.
- **Page order:** the engine already reported each edit's page order (`edit.rs`, `EditReport.page_order`), and the desktop's edit session already passed it to the viewer, so this todo needed no change there. The first scenario shows the viewer and grid following moves, inserts, deletes and the reversal. Unlike the web worker (`pagesAfter`), the desktop doesn't check that order against the page list. The engine reads both from the one edited document in the same call; the worker checks because its text copy follows the edits separately. So the scope's "checked against the engine's `pageOrder`" was dropped.
- **Class names on the web:** the mode's classes moved with it, from `web-organise*` to `paper-organise*` (the grid's `paper-organise-mode`); the web's e2e selector follows.
- **The inspector on narrow windows:** below the rail's docked width (1024 px), the inspector is now removed from the page rather than hidden by a CSS query, since the package keeps its width breakpoints in code. The threshold is the same on the web.

## Post-merge checks

None.

## Tests

- `apps/desktop/e2e/specs/organise.e2e.ts`

## Docs to update

- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and the [E2 plan §8](../../plans/editor/e2-page-organisation.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
