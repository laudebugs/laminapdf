# E5i — Import, export, flatten and summary in the interface

| Field | Value |
| --- | --- |
| ID | `E5i` |
| Status | `done` |
| Plan | [E5 plan §2.3](../../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E5a](./E5a-review-stamps-interchange-feature-plan.md), [E5d](./E5d-xfdf-fdf.md), [E5e](./E5e-flatten-and-summary.md), [E5f](./E5f-comment-summary.md), [E5g](./E5g-comments-panel.md), [E5k](./E5k-summary-words.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0024](../../decisions/0024-app-shell-ux.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e5i-interchange-flatten-summary-ui` |
| PR title | `feat(ui): import and export comments, flatten, and the comment summary` |
| Size | M (200–400): three dialogs over finished functions |
| PR | [#820](https://github.com/laudebugs/laminapdf/pull/820) |

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
- The summary through the document worker: the worker client doesn't serve `doc.commentSummary`, since it returns a new document (as `extractPages` does), so the worker makes it and hands the new tab its saved bytes (E5f)
- The font the summary is set in: `commentSummary` needs an embeddable font (PDF/UA-1 refuses the standard 14), so the app supplies one with the coverage its comments need, and fallbacks for other scripts
- The pages the summary won't draw: those whose fonts the file doesn't embed, or whose content can't be read to its end within the document's limits, come back as `summary-page-not-drawn` warnings (data `page`, `reason`); the dialog names them before Create, or after, from the warnings
- Progress and cancel: `commentSummary` is one call, not a job (E5f's As built, the E5 plan §2.3), so C07's determinate progress bar is indeterminate, and Cancel drops the result in the worker
- The summary's own words in the UI's language: the dialog fills `words` (`SummaryWords`, [E5k](../done/E5k-summary-words.md)) from the messages catalogue, one template per phrase and the kind and state names

**Out of scope** (and which todo covers it)

- The desktop → [E5j](../ready/E5j-desktop-review.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.13`, `D.14`, `D.15` (summary) in the interface
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`SaveConstraint`:** C07's notes and E1e's banner
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `D.13`, `D.14`, `D.15` → `round-trip` (web)

## Tasks

- [x] Export and import dialogs
- [x] The flatten dialog
- [x] The summary dialog
- [x] Playwright specs tagged with capability IDs

## Acceptance criteria

- [x] Playwright in Chromium, Firefox and WebKit: export and import an XFDF, flatten a page and save, make a summary, each checked in the saved or new file
- [x] On the certified-P3 fixture, Flatten offers only a copy
- [x] axe reports no serious violation in the three dialogs

## As built

- **Worker.** The document worker serves four calls, in `packages/viewer/src/worker/interchange.ts` and `edit.ts`: `exportComments` and `previewImport` read the document as its edits leave them; `importComments` is one edit step (undo takes it whole) that a restarted worker imports again (`EditResult.imported`, `ReplayedStep.imported`) and a text-capable copy follows; `commentSummary` writes the document as a save would, opens that copy in the text module, lays the summary out, saves it and lets both go, so the edit session is untouched and the page holds only the bytes. Flatten is the `flattenAnnots` op with the chosen comments' IDs, so it needs no call of its own.
- **Where the summary opens.** With `multiWindow` the platform's `openWindow`; otherwise a new browser tab, which the opening tab opens at the press that asked (a browser allows it only then) with a one-time token in its address fragment, and hands the saved bytes over a same-origin `BroadcastChannel` (`apps/web/src/open/handoff.ts`). Nothing is stored, and a cancelled or failed summary closes the tab. A browser that blocks the tab gets the file as a download.
- **Remove from history** is `SaveFlow.rewriteNextSave`: the next save is a full rewrite, and a signed file asks first as any save that invalidates a signature does.
- **Summary font and words.** Lamina Sans, then Noto Sans Arabic as the fallback (`readSummaryFonts`). The summary's words are the later catalogue's `summary.words.*`, `summary.kind.*` and `summary.state.*`.
- **Left out, and why.** Dropping an `.xfdf` or `.fdf` on the window; Markdown headings with the outline's section titles (the export takes none); a live preview with numbered markers; the hidden-comments line and the signer's name in Flatten's notes (the threads and the save plan don't carry them). The area document records each under C06 and C07's "As built on the web (E5i)".
- **Checked.** Vitest for the dialogs' state (`packages/ui/test/interchange.test.ts`), the counts and requests (`apps/web/src/comment/interchange-data.test.ts`), the hand-off (`apps/web/src/open/handoff.test.ts`) and the edit session's import; `apps/web/e2e/interchange.e2e.test.ts` in Chromium, Firefox and WebKit, run here against the built reader. The gallery's new dialog states have no golden screenshots: `gallery --update` times out in Docker here and no workflow runs it.

## Post-merge checks

None.

## Tests

- Playwright specs in `apps/web/e2e/`

## Docs to update

- User help: exchanging, flattening and summarising comments
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

Left out of this todo, each with an issue:

- Encrypted files can't be exported or summarised, though the plan says they should work: [#823](https://github.com/laudebugs/laminapdf/issues/823).
- Summary fonts cover Latin and Arabic only: [#824](https://github.com/laudebugs/laminapdf/issues/824).
- Flatten's "Not counted: n hidden comments" line: [#825](https://github.com/laudebugs/laminapdf/issues/825).
- The summary dialog's live preview with numbered markers: [#826](https://github.com/laudebugs/laminapdf/issues/826).
- "Shown in the panel" exports whole pages: `ExportCommentsOptions` takes pages and a kind of comments, not comment IDs: [#827](https://github.com/laudebugs/laminapdf/issues/827).
- Markdown's section-title headings: [#828](https://github.com/laudebugs/laminapdf/issues/828).
- Dropping an `.xfdf` or `.fdf` on the window to open Import: [#829](https://github.com/laudebugs/laminapdf/issues/829).

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
