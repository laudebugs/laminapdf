# D9c — Desktop: find in document

| Field | Value |
| --- | --- |
| ID | `D9c` |
| Status | `ready` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | None |
| ADRs | Follows [ADR 0008](../../decisions/0008-desktop-shell.md) (the engine runs sandboxed and reads untrusted PDFs); may add an amendment for the new engine message |
| Branch | `feat/d9c-desktop-find` |
| PR title | `feat(desktop): find in document` |
| Size | L (400–800: the engine's text read and search, the protocol, the client, the find bar and the highlights; split the engine half from the interface half if you can, the engine PR first) |
| PR | — |

## Goal

Let a person press Cmd+F or Ctrl+F in the desktop, type, and see every match highlighted on the page with a count, then step through them with Enter and Shift+Enter or the Find Next and Find Previous menu items. These three items, and the Edit ▸ Find shortcut, are disabled today.

## Context

- [#793](https://github.com/laudebugs/laminapdf/issues/793) and its root-cause comment.
- `packages/ui/src/commands/reader-commands.ts` (`findCommands`): `find.open`, `find.next` and `find.previous` are `delegated()`.
- The web handlers: `useFind` in `apps/web/src/find.tsx` shows `FindBar` from `@laminapdf/ui` and calls `viewer.find` (open, next, previous, close). The bar and the viewer's find controller (`packages/viewer/src/find`) are shared; the web's results rail tab (`apps/web/src/rail.tsx`, "Results") is not, and belongs to [D9d](./D9d-desktop-sidebar-and-outline.md).
- **What the desktop engine lacks.** `apps/desktop/src/engine/desktop-engine-client.ts` (header, "Text, links and search aren't served by the engine yet"): `pageText` returns no text and `find` returns `noText: true` for every page.
- The engine (`apps/desktop/src-tauri/src/engine/`, the protocol in `crates/laminapdf-desktop-protocol/src/msg.rs`) has `Query` reads for the Organise mode and none for text.
- So the engine gets a read that returns a page's text runs with positions (what `PageTextData` holds) and a search over them.
- The source is the work's choice: PDFium's text page through `pdfium-render` (`FPDFText_*`, already loaded for drawing) or the text extraction in the Rust library. The work measures on the large fixtures and records the choice.
- The read stays inside the sandbox's rules and adds no descriptor or request.
- [Desktop plan](../../plans/desktop-app.md) §5 (design summary) and ADR 0008: the engine process owns the document; the webview never sees the PDF's bytes beyond tiles.
- `apps/desktop/src/menus/use-native-menus.test.ts` line 32 asserts `find.open` is disabled. That assertion must change: with the desktop's handler present, `find.open` is enabled (and still disabled with no document open).

## Scope

**In scope**

- The engine's text read per page and a search over the pages (case and whole-word options as the web's find offers), through a new protocol message, with the sandbox and size limits the other reads have.
- `DesktopEngineClient.pageText` and `find` answering from the engine, so `viewer.find` works unchanged.
- The find bar, the match highlights, the count, next and previous, in the desktop, through the web's `FindBar` and viewer controller.
- A shared `useFind` if the web's can move to `@laminapdf/ui` without change (one home per concept), else the desktop calls the web's logic through the shared package.
- Menu items enabled and working: `find.open`, `find.next`, `find.previous`, and the Cmd+F shortcut.
- A scanned page (no text) reports "no text on this page" as the viewer's `noText` flag does.

**Out of scope**

- The Results rail tab and the outline: [D9d](./D9d-desktop-sidebar-and-outline.md).
- Search and replace: the editor (A.08).
- Text selection and copy, if the desktop lacks them; this todo only adds what find needs, and says if selection should follow.

## Tasks

- [ ] Fresh worktree; claim in the work PR.
- [ ] Decide and record the engine's text source; measure the cost on a long and a large fixture (first match time, memory).
- [ ] Add the `Query` (or message) for page text and search in `laminapdf-desktop-protocol`, the engine side and the main process's relay; add a fuzz or limits test as the other reads have, and run the sandbox selftest.
- [ ] Answer `pageText` and `find` in `desktop-engine-client.ts`; update its header comment.
- [ ] Mount the find bar and register the three handlers in `apps/desktop/src/app.tsx`; clear the bar when the tab or document changes.
- [ ] Update `use-native-menus.test.ts` line 32 and add the tests below.
- [ ] Try it in the built app on each OS and in a browser from the user's side.
- [ ] Docs below.

## Acceptance criteria

- [ ] In the real app on macOS, Windows and Linux, Edit ▸ Find, Find Next and Find Previous are enabled with a document open, and Cmd+F or Ctrl+F opens the bar.
- [ ] Typing a word highlights every match on the visible pages, shows "n of m", and Enter and Shift+Enter move between matches, scrolling the page to each.
- [ ] Closing the bar or switching tab clears the highlights; a document with no text says so rather than showing zero matches.
- [ ] The existing assertion at `use-native-menus.test.ts:32` is changed to expect `find.open` enabled when a handler and a document are present, and a new case keeps it disabled with no document; a test shows `find.next` and `find.previous` are available.
- [ ] Engine tests: search finds a word across a page, honours case and whole-word, handles ligatures and right-to-left text as the web's find does on the same fixtures, and returns within the limit on the 1,000-page fixture.
- [ ] A browser check from the user's side with real PDFs (a text one, a scanned one, one with CJK): search, step through, close, and judge the highlights sit on the words. Record what was seen.
- [ ] The engine sandbox selftest still passes, and the store build's engine needs no new descriptor.

## Post-merge checks

- [ ] None.

## Tests

- Rust: the text read and search on `fixtures/` PDFs, including a page with no text; a limits and malformed-input test.
- TypeScript: `desktop-engine-client.test.ts` for `pageText` and `find`; `use-native-menus.test.ts` as above.
- Desktop e2e (`apps/desktop/e2e/specs/`): open a fixture, press the shortcut, type a word, read the count.

## Docs to update

- `docs/plans/desktop-app.md`: tick the to-do under D9; update §5 if the engine gains a read.
- `docs/decisions/0008-desktop-shell.md`: a line for the new engine read if the protocol gains a message.
- `docs/capabilities/README.md`: no row covers reading-side search; if the matrix gets one, set the desktop cell with the e2e as evidence. Otherwise no change.
- The header comment of `desktop-engine-client.ts`.

## Risks and open questions

- **Size.** The engine read is most of the work. If the PR passes 800 lines, split it: the engine read and client first, the bar and menu second.
- **Text quality.** PDFium's text order and the library's can differ; the match positions must come from the same source as the highlights.
- **Memory on huge files.** Search runs page by page with a cancel, as the web's does; confirm the engine does not hold the whole text.
- **Reader mode needs the same text.** [D9e](./D9e-desktop-reader-mode.md) depends on this read.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
