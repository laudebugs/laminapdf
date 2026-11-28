# E6k — The Page marks dialog on the desktop

| Field | Value |
| --- | --- |
| ID | `E6k` |
| Status | `ready` |
| Plan | [E6 plan §2.3](../../plans/editor/e6-adding-content.md#23-page-marks) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6g](../done/E6g-page-marks-dialog.md), [E6j](./E6j-desktop-content-tools.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e6k-desktop-page-marks` |
| PR title | `feat(desktop): the Page marks dialog` |
| Size | M (200–400): the protocol, the engine, the client, the mount and WebdriverIO on three OSes |
| PR | — |

## Goal

The desktop reader gets the Page marks dialog and Manage page marks that [E6g](../done/E6g-page-marks-dialog.md) built for the web. E6g's controller and dialogs are in `apps/web/src/page-marks/` and `@laminapdf/ui`; the desktop has its own engine client, so the two calls the web's viewer client gained, `pageMarks()` and `textRuns(index)`, have to cross the engine protocol first. E6g left the desktop out, as [E6e](../done/E6e-content-tools-ui.md) did, because that work is another agent's.

## Context

- [E6g](../done/E6g-page-marks-dialog.md) and [A03](../../design/paper/add-edit-content.md#a03-watermark-header-and-footer-dialog)
- `apps/desktop/src/engine/desktop-engine-client.ts`: the desktop's implementation of the viewer client, which has neither call yet
- [E6j](E6j-desktop-content-tools.md): Edit mode on the desktop, which the dialog's commands switch to

## Scope

**In scope**

- `pageMarks()` and `textRuns(index)` in the engine protocol and the desktop engine client, answered by the engine's document
- Mounting the dialogs beside Edit mode in the desktop app, from the code the web reader shares; fonts on the computer in its font select come with [E6h](E6h-desktop-system-fonts.md)
- The desktop columns of the capability rows E6g flips on the web
- WebdriverIO scenarios: a watermark, a header and footer and Bates numbers added, saved, found again in Manage page marks, edited and removed

**Out of scope** (and which todo covers it)

- Bates numbering across files → [E14d](E14d-batch-runner.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `B.10`, `C.14`
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-protocol`, `laminapdf-desktop-engine`
- **`SaveConstraint`:** as E6d
- **Web vs desktop:** desktop only
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `B.10` → `round-trip` (desktop); `C.14` → `round-trip` (desktop)

## Tasks

- [ ] `pageMarks` and `textRuns` in the protocol, the engine and the client
- [ ] The dialogs in the desktop app
- [ ] WebdriverIO scenarios

## Acceptance criteria

- [ ] Each tab adds its mark as one undo step on the desktop
- [ ] Editing and removing a mark from Manage page marks works after a save and reopen
- [ ] e2e on macOS, Windows and Linux

## Post-merge checks

None.

## Tests

- WebdriverIO; a protocol round-trip test for `pageMarks` and `textRuns`

## Docs to update

- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [E6 plan](../../plans/editor/e6-adding-content.md) as-built notes

## Risks and open questions

- The desktop engine is built without the builder's appearance writer ([#834](https://github.com/laudebugs/laminapdf/issues/834)), which the page mark ops need as `addContentObject` does

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
