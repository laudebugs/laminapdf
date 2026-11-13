# C8a — Every close asks about unsaved changes first

| Field | Value |
| --- | --- |
| ID | `C8a` |
| Status | `done` |
| Plan | [Close a document §4](../../plans/close-document.md#4-decisions) |
| Round / milestone | Round C (web reader): C8, closing a document |
| Depends on | None |
| ADRs | [ADR 0024](../../decisions/0024-app-shell-ux.md) (commands), [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (`#url=` stays in the fragment) |
| Branch | `fix/c8a-every-close-asks-first` |
| PR title | `fix(web): ask about unsaved changes before the address or Try another closes a document` |
| Size | S |
| PR | [laudebugs/laminapdf#555](https://github.com/laudebugs/laminapdf/pull/555) |

## Goal

Three routes close the web reader's document without asking about unsaved changes: removing `#url=` from the address (by editing it or pressing Back), changing it to an unusable `#url=` link (`showLinkProblem`), and the failure page's Try another. All three call `ReaderController.close()`, which skips the close guard, and a fragment change fires no `beforeunload`, so an edit session's changes are lost without a word. This todo sends every close through `requestClose()`, so O03 asks first, and puts the link back in the address when the reader answers Cancel. C8b's Close button builds on this one path.

## Context

- [Close a document plan](../../plans/close-document.md) §2 (what exists) and §4 decision 5.
- [Reader A R19](../../design/paper/reader-a.md#r19-close-a-document): Behaviour.
- `packages/ui/src/state/reader-controller.ts`: `requestClose()` asks the guard; `close()` doesn't.
- `apps/web/src/open/watch.ts` and `navigation.ts`: the fragment's `close` action; `forget()` removes `url` with `replaceState`.
- `apps/web/src/session.ts`: `showLinkProblem` closes before showing the link's problem; the backend switch (`#debug`) closes and reopens the same file, which needs no guard.
- `apps/web/src/app.tsx`: `onTryAnother={() => reader.close()}`.
- `apps/web/src/editing-runtime.tsx`: the guard and O03's dialog.

## Scope

**In scope**

- The fragment watcher's `close`, `showLinkProblem` and Try another through `requestClose()`; on Cancel, `showLinkProblem` shows no problem page.
- Cancel after a fragment change restores the `url` key (and any place keys) with `replaceState`, adding no history entry.
- A note in `ReaderController.close()`'s doc comment: it is for closes the reader didn't ask for (a backend switch); everything the reader asks for goes through `requestClose()`.

**Out of scope** (and which todo covers it)

- The Close button, `file.close`, More's items, Back to files: [C8b](C8b-close-button-and-command.md).

## Tasks

- [x] `watch.ts`: on `close`, call `requestClose()`; when it resolves `false`, `watch.ts` writes the fragment itself from what the reader holds (`remoteUrlOf(state.file)`) with `replaceState`, so the address names the open file again. `forget()` won't re-add it: it only removes keys, and a Cancel changes no state.
- [x] `session.ts`: `showLinkProblem` through `requestClose()`; on Cancel, restore the fragment the same way and show no problem page.
- [x] `app.tsx`: Try another through `requestClose()`. A failed document has no guard (the guard applies only while `open`), so behaviour stays the same; the one path matters for C8b.
- [x] Check no other caller closes for the reader: `git grep -E "reader\.close\(\)"` in `apps/web/src` and `packages/ui/src` (this matches `this.reader.close()` too), and record each kept one, such as the backend switch, with its reason.

## Acceptance criteria

- [x] With an unsaved edit on a `#url=` document, pressing Back to the address without `#url=` shows O03; Cancel keeps the document and the address shows its `#url=` again; Don't save closes to the home screen.
- [x] Without unsaved edits, the same Back closes at once, as today.

## Post-merge checks

None.

## Tests

- `apps/web/src/open/`: a unit test of the watcher with a fake engine whose guard answers `false`, then `true` (the fragment restored, then closed).
- `apps/web/e2e/remote.e2e.test.ts` or `edit-session.e2e.test.ts`: the acceptance case in Chromium, Firefox and WebKit.

## Docs to update

The plan's to-do; [Reader A R19](../../design/paper/reader-a.md#r19-close-a-document) only if behaviour differs from it.

## Risks and open questions

- `popstate` from the browser's Back has already moved the history entry when the guard asks; restoring the fragment with `replaceState` rewrites that entry. Check that Forward then still works as the reader expects.

## As built

- `watch.ts` sends the fragment's close and an unusable link through the guard; after a Cancel it writes the `url` key the reader holds back into the address (`writeUrl`), which `forget()` now uses too.
- `Engine.showLinkProblem` resolves whether it closed; on Cancel the reader shows no problem page.
- The audit of `reader.close()`: two calls are left, both closes the reader didn't ask for: `ReaderController.requestClose()` itself, and the `#debug` backend switch, which reopens the same file. `close()`'s doc comment now says so.
- Not checked: Forward after a Cancel that followed Back. The restored entry is the one Back landed on, rewritten by `replaceState`, so Forward goes where it went before.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
