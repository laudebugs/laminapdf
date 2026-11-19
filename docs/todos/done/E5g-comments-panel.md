# E5g — The Comments panel

| Field | Value |
| --- | --- |
| ID | `E5g` |
| Status | `done` |
| Plan | [E5 plan §2.2](../../plans/editor/e5-review-stamps-interchange.md#22-reading-comments) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4f](./E4f-annotation-tools-ui.md), [E4k](./E4k-touch-and-stylus-annotating.md), [E5a](./E5a-review-stamps-interchange-feature-plan.md), [E5b](./E5b-comment-threads-and-review.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e5g-comments-panel` |
| PR title | `feat(ui): the Comments panel with threads, replies, status, filters and search` |
| Size | L (400–800): the panel, its filters and bulk actions, and the phone's comment sheet |
| PR | [#755](https://github.com/laudebugs/laminapdf/pull/755) |

## Goal

The Comments panel of C04 and the phone's comment sheet of C08: threads by page, replies, status, filters, search, sorting, jump to a comment, and bulk delete.

## Context

- **Design (Paper):** [C04 Comments panel](../../design/paper/comment-review.md#c04-comments-panel), [C08 Phone annotate](../../design/paper/comment-review.md#c08-phone-annotate). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E5 plan §2.2](../../plans/editor/e5-review-stamps-interchange.md#22-reading-comments)

## Scope

**In scope**

- The panel in the right dock, with its threads from `comment_threads`, replies, the Status menu and Mark as checked, the broken-thread notice and the empty states
- Filters by status, author, kind, date and page, with chips; search over note, replies, quoted text and author; sorting
- Bulk selection and "Delete n…" through `delete_annotations`
- The phone's comment sheet with replies and status
- Commands `comment.go-to`, `comment.reply`, `comment.set-status`, `comments.filter`, `comments.search`, `comments.sort`, `comments.delete-selected`, `comments.toggle-panel`
- The author name asked once and kept in Settings on this device
- From [E4f](../done/E4f-annotation-tools-ui.md), which asks for the name when the first tool is chosen and keeps it on the device (changed from the inspector or the palette): the name's entry in Settings; the inspector's Reply; Shift+F10's menu on a selected annotation; a selected annotation announced with its kind, border and colour ("Rectangle, cloudy border, blue, selected", Paper C02)

**Out of scope** (and which todo covers it)

- Import, export, flatten and summary → [E5i](../ready/E5i-interchange-flatten-summary-ui.md)
- The desktop → [E5j](../ready/E5j-desktop-review.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.10`, `D.11`, `D.12`, `D.15` (delete) in the interface
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`, `apps/web`
- **`SaveConstraint`:** the banner and gating from E1e
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `D.10`, `D.11` → `round-trip` (web); `D.12` → `read` (web)

## Tasks

- [x] The panel and threads
- [x] Filters, search and sorting
- [x] Bulk actions
- [x] The phone's comment sheet
- [x] Playwright specs tagged with capability IDs

## Acceptance criteria

- [x] Playwright in Chromium, Firefox and WebKit: reply, set a status, filter, search and delete three comments on `review.pdf`, save, and reopen in pdf.js with the replies and states kept
- [x] axe reports no serious violation in the panel and the sheet
- [x] `review.pdf`'s broken threads show with the notice

## Post-merge checks

None.

## Tests

- Playwright specs in `apps/web/e2e/`
- Vitest for filters and search

## Docs to update

- User help: reviewing comments
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
