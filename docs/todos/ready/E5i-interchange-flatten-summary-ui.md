# E5i — Import, export, flatten and summary in the interface

| Field | Value |
| --- | --- |
| ID | `E5i` |
| Status | `ready` |
| Plan | [E5 plan §2.3](../../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E5a](../done/E5a-review-stamps-interchange-feature-plan.md), [E5d](./E5d-xfdf-fdf.md), [E5e](./E5e-flatten-and-summary.md), [E5f](./E5f-comment-summary.md), [E5g](./E5g-comments-panel.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0024](../../decisions/0024-app-shell-ux.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e5i-interchange-flatten-summary-ui` |
| PR title | `feat(ui): import and export comments, flatten, and the comment summary` |
| Size | M (200–400): three dialogs over finished functions |
| PR | — |

## Goal

The dialogs of C06 and C07: export as XFDF, FDF or Markdown with preview and Copy; import with its summary, modes and warnings; flatten with its scopes and signature notes; and the comment summary, opened in a new tab or window.

## Context

- **Design (Paper):** [C06](../../design/paper/comment-review.md#c06-import-and-export-comments), [C07](../../design/paper/comment-review.md#c07-flatten-and-comment-summary). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E5 plan §5](../../plans/editor/e5-review-stamps-interchange.md#5-saving-and-the-save-constraint): what flattening does on signed files

## Scope

**In scope**

- `comments.export`, `comments.export-markdown`, `comments.copy-markdown`, `comments.import`, `comments.flatten`, `comments.summary`, `comments.print-summary`, in the panel's More menu and the palette
- The import summary with Merge, Replace and Add; the mismatch warning; a malformed file's line
- The flatten dialog with All, This page, Shown and Selected; the approval-signature note; "Flatten in a copy…" on a certified file; "remove from history"
- The summary dialog with C07's options; the summary opened through `multiWindow`

**Out of scope** (and which todo covers it)

- The desktop → [E5j](E5j-desktop-review.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.13`, `D.14`, `D.15` (summary) in the interface
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`SaveConstraint`:** C07's notes and E1e's banner
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `D.13`, `D.14`, `D.15` → `round-trip` (web)

## Tasks

- [ ] Export and import dialogs
- [ ] The flatten dialog
- [ ] The summary dialog
- [ ] Playwright specs tagged with capability IDs

## Acceptance criteria

- [ ] Playwright in Chromium, Firefox and WebKit: export and import an XFDF, flatten a page and save, make a summary, each checked in the saved or new file
- [ ] On the certified-P3 fixture, Flatten offers only a copy
- [ ] axe reports no serious violation in the three dialogs

## Post-merge checks

None.

## Tests

- Playwright specs in `apps/web/e2e/`

## Docs to update

- User help: exchanging, flattening and summarising comments
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
