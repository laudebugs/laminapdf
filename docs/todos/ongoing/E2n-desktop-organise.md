# E2n — Desktop: the Organise mode over the engine's edit session

| Field | Value |
| --- | --- |
| ID | `E2n` |
| Status | `ongoing` |
| Plan | [E2 page organisation §7](../../plans/editor/e2-page-organisation.md#7-differences-between-the-shells) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [D3a](../done/D3a-open-from-os.md), [E1i](../done/E1i-desktop-save-in-place.md), [E2g](../done/E2g-page-labels.md), [E2m](../done/E2m-document-dialogs.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e2n-desktop-organise` |
| PR title | `feat(desktop): the Organise mode, page setup and labels over the engine` |
| Size | L (600–900): the Organise mode shared with the web, the engine's planner check, the page-setup dialogs |
| PR | [#683](https://github.com/laudebugs/laminapdf/pull/683) |

## Goal

The desktop part of round E2 ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): an editor round's desktop part is its own todo), first half. The desktop has no Organise mode yet: this todo brings it over the engine's edit session, with page setup, page labels and the signature checks that gate page edits. The second half, [E2t](../ready/E2t-desktop-sources-and-folders.md), adds what reads or writes other files (insert from a file, replace, merge, split to a folder, the multi-file open offer); the owner split the two on 2026-11-16, when the desktop turned out to need the whole Organise mode first.

## Context

- **Design (Paper):** [O06](../../design/paper/edit-session-organise.md#o06-organise-mode); O07 and O08 are [E2t](../ready/E2t-desktop-sources-and-folders.md)'s
- [E2 plan §5 and §7](../../plans/editor/e2-page-organisation.md#7-differences-between-the-shells)
- [E1h](../done/E1h-desktop-edit-session.md) and [E1i](../done/E1i-desktop-save-in-place.md): the engine's edit session and saving in place; [D3a](../done/D3a-open-from-os.md): open requests with several paths

## Scope

**In scope**

- The Organise mode on the desktop: the web's `OrganiseController`, toolbar, grid and dialogs over the engine's edit session, shared rather than copied
- The engine's edit session reports the new page order as the worker does ([E2l](../done/E2l-viewer-across-page-edits.md)), checked against the engine's `pageOrder`, so the shared viewer follows page-list edits on the desktop
- `wouldViolate` through the engine's save planner (a protocol message), so the signed-page question and the certified-document banner work on the desktop
- The page-setup dialogs (crop, resize, boxes, orientation) and the page labels editor (E2g) over the engine
- WebdriverIO suites on macOS, Windows and Linux

**Out of scope** (and which todo covers it)

- Insert from a file, replace, merge, split and extract to a folder, the multi-file open offer: [E2t](../ready/E2t-desktop-sources-and-folders.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): every E2 row's desktop column
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-engine`, `laminapdf-desktop-protocol`
- **`SaveConstraint`:** as the web; the banner and the named-signature dialog checked on the desktop here
- **Web vs desktop:** this todo is the desktop side
- **Server features:** none
- **Capability-matrix rows to flip:** `C.01`, `C.03`, `C.04`, `C.08`, `C.09`, `C.10`, `C.11`, `C.12`, `C.13`, `I.10` → `round-trip` (desktop); the rest of round E2's desktop column is [E2t](../ready/E2t-desktop-sources-and-folders.md)'s

## Tasks

- [ ] The Organise mode on the desktop, sharing the web's controller
- [ ] Page order from the engine's session
- [ ] `wouldViolate` through the engine's planner
- [ ] Page setup and page labels over the engine
- [ ] WebdriverIO specs `apps/desktop/e2e/specs/organise.e2e.ts`, tagged with capability IDs

## Acceptance criteria

- [ ] WebdriverIO on three systems: reorder, delete, insert blank pages, rotate, reverse, a crop and a page-label edit done through the interface, saved in place, reopened and checked
- [ ] On the approval-signed fixture, deleting the signature's page shows the named-signature dialog; on a certified one, a drag is refused with the banner

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

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
