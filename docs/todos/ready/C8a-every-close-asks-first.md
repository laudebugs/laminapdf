# C8a — Every close asks about unsaved changes first

| Field | Value |
| --- | --- |
| ID | `C8a` |
| Status | `ready` |
| Plan | [Close a document §4](../../plans/close-document.md#4-decisions) |
| Round / milestone | Round C (web reader): C8, closing a document |
| Depends on | None |
| ADRs | [ADR 0024](../../decisions/0024-app-shell-ux.md) (commands), [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (`#url=` stays in the fragment) |
| Branch | `fix/c8a-every-close-asks-first` |
| PR title | `fix(web): ask about unsaved changes before the address or Try another closes a document` |
| Size | S |
| PR | — |

## Goal

Two routes close the web reader's document without asking about unsaved changes: removing `#url=` from the address (by editing it or pressing Back) and the failure page's Try another. Both call `ReaderController.close()`, which skips the close guard, and a fragment change fires no `beforeunload`, so an edit session's changes are lost without a word. This todo sends every close through `requestClose()`, so O03 asks first, and puts the link back in the address when the reader answers Cancel. C8b's Close button builds on this one path.

## Context

- [Close a document plan](../../plans/close-document.md) §2 (what exists) and §4 decision 5.
- [Reader A R19](../../design/paper/reader-a.md#r19-close-a-document): Behaviour.
- `packages/ui/src/state/reader-controller.ts`: `requestClose()` asks the guard; `close()` doesn't.
- `apps/web/src/open/watch.ts` and `navigation.ts`: the fragment's `close` action; `forget()` removes `url` with `replaceState`.
- `apps/web/src/app.tsx`: `onTryAnother={() => reader.close()}`.
- `apps/web/src/editing-runtime.tsx`: the guard and O03's dialog.

## Scope

**In scope**

- The fragment watcher's `close` and Try another through `requestClose()`.
- Cancel after a fragment change restores the `url` key (and any place keys) with `replaceState`, adding no history entry.
- A note in `ReaderController.close()`'s doc comment: it is for closes the reader didn't ask for (a backend switch); everything the reader asks for goes through `requestClose()`.

**Out of scope** (and which todo covers it)

- The Close button, `file.close`, More's items, Back to files: [C8b](C8b-close-button-and-command.md).

## Tasks

- [ ] `watch.ts`: on `close`, call `requestClose()`; when it resolves `false`, rebuild the fragment from what the reader holds (`remoteUrlOf(state.file)`) with `replaceState`, so the address names the open file again.
- [ ] `app.tsx`: Try another through `requestClose()`. A failed document has no guard (the guard applies only while `open`), so behaviour stays the same; the one path matters for C8b.
- [ ] Check no other caller of `reader.close()` closes for the reader: `git grep "reader.close()"` in `apps/web/src` and `packages/ui/src`, and record each kept one with its reason.

## Acceptance criteria

- [ ] With an unsaved edit on a `#url=` document, pressing Back to the address without `#url=` shows O03; Cancel keeps the document and the address shows its `#url=` again; Don't save closes to the home screen.
- [ ] Without unsaved edits, the same Back closes at once, as today.

## Post-merge checks

None.

## Tests

- `apps/web/src/open/`: a unit test of the watcher with a fake engine whose guard answers `false`, then `true` (the fragment restored, then closed).
- `apps/web/e2e/remote.e2e.test.ts` or `edit-session.e2e.test.ts`: the acceptance case in Chromium, Firefox and WebKit.

## Docs to update

The plan's to-do; [Reader A R19](../../design/paper/reader-a.md#r19-close-a-document) only if behaviour differs from it.

## Risks and open questions

- `popstate` from the browser's Back has already moved the history entry when the guard asks; restoring the fragment with `replaceState` rewrites that entry. Check that Forward then still works as the reader expects.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
