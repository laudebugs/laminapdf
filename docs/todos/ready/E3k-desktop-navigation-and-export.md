# E3k — Desktop: navigation, attachments, inspection and export

| Field | Value |
| --- | --- |
| ID | `E3k` |
| Status | `ready` |
| Plan | [E3 plan §7](../../plans/editor/e3-navigation-attachments-inspection.md#7-differences-between-the-shells) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1i](./E1i-desktop-save-in-place.md), [E2i](./E2i-signed-gating-and-close.md), [E2n](./E2n-desktop-organise.md), [E3b](../done/E3b-bookmarks-and-destinations.md), [E3c](./E3c-links.md), [E3d](../done/E3d-initial-view-and-threads.md), [E3e](../done/E3e-xmp-editor.md), [E3f](./E3f-attachments-and-associated-files.md), [E3g](./E3g-inspectors.md), [E3h](../done/E3h-images-to-pdf-and-page-export.md), [E3i](../ongoing/E3i-extraction.md), [E3j](./E3j-export-dialog.md), [E3l](./E3l-contents-page.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e3k-desktop-navigation-and-export` |
| PR title | `feat(desktop): bookmarks, links, attachments, inspector and exports` |
| Size | M (200–400): the round's panels over the engine, native encoding into a folder, and the WebdriverIO suites |
| PR | — |

## Goal

The desktop part of round E3 ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): an editor round's desktop part is its own todo), and the round's close. Every E3 panel and dialog runs over the engine's edit session; exports are encoded natively, pages in parallel, into a folder; attachments save to disk through a dialog. The signed-document table of the [E3 plan §6](../../plans/editor/e3-navigation-attachments-inspection.md#6-saving-and-the-save-constraint) runs as one suite. With this todo merged, every E3 row's desktop column is complete.

## Context

- **Design (Paper):** [O11](../../design/paper/pages-navigation-extraction.md#o11-bookmarks-editor) to [O16](../../design/paper/pages-navigation-extraction.md#o16-export-and-extraction), desktop artboards
- [E3 plan §6 and §7](../../plans/editor/e3-navigation-attachments-inspection.md#7-differences-between-the-shells)
- [E1h](E1h-desktop-edit-session.md), [E1i](E1i-desktop-save-in-place.md) and [E2n](E2n-desktop-organise.md): the engine's edit session, saving in place, folder output

## Scope

**In scope**

- The engine runs the round's ops, its inspection and its exports; PNG and JPEG through `png` and `jpeg-encoder`, TIFF through `tiff`
- Exports into the chosen folder with Replace / Keep both and Show in Finder or Explorer; attachments saved through the save dialog, with O14's warning for programs
- The initial view applied when the desktop opens a file
- The signed-document suite: `would_violate` and the banner for every E3 op on each `fixtures/signed/` file
- WebdriverIO suites on macOS, Windows and Linux

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): every E3 row's desktop column
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-engine`, `laminapdf-desktop-protocol`
- **`SaveConstraint`:** as the web; the banner checked on the desktop here
- **Web vs desktop:** this todo is the desktop side
- **Server features:** none
- **Capability-matrix rows to flip:** `I.01`, `I.02`, `I.03`, `I.04`, `I.05`, `I.06`, `I.11`, `A.09`, `J.04`, `J.05`, `M.09` → `round-trip` (desktop); `H.02` → `write` (desktop); `I.09`, `J.06`, `J.07`, `J.08`, `J.09`, `H.06`, `H.07`, `H.08`, `H.09`, `H.11`, `B.03`, `A.10` → `read` (desktop)

## Tasks

- [ ] The engine's calls for the round's ops, inspection and exports
- [ ] Native encoding into a folder
- [ ] The signed-document suite
- [ ] WebdriverIO specs `apps/desktop/e2e/specs/navigation.e2e.ts` and `export.e2e.ts`, tagged with capability IDs

## Acceptance criteria

- [ ] WebdriverIO on three systems: a bookmark, a link, an attachment and an initial view made through the interface, saved in place, reopened and checked; 50 pages exported as TIFF into a folder
- [ ] On the certified-P3 fixture a link saves incrementally and a bookmark is refused with the banner

## Post-merge checks

None.

## Tests

- `apps/desktop/e2e/specs/navigation.e2e.ts`, `apps/desktop/e2e/specs/export.e2e.ts`
- The signed-document suite in `crates/laminapdf-save/tests/`

## Docs to update

- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
