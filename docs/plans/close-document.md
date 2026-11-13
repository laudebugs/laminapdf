# Plan: close a document and go back to the home screen

- **Status:** Accepted (owner, 2026-11-13: the × beside the file name, no Reopen toast)
- **Date:** 2026-11-13
- **Asked for by:** the project owner (2026-11-13): "how to implement going back to the empty start screen through a close button".
- **Changes:** [Reader A](../design/paper/reader-a.md)'s top bar and More menu, the screen inventory (a new screen, R19), and the command list in [design language §8.8](../design/paper/design-language.md#88-keyboard-and-shortcuts)
- **Builds on:** the web reader's shell ([C4b](../todos/done/C4b-web-app-shell.md)), the edit session's close confirmation (O03, [edit session §O03](../design/paper/edit-session-organise.md)), the command registry ([ADR 0024](../decisions/0024-app-shell-ux.md))
- **Todos:** [§6](#6-todos)

## 1. Goal

The web reader can only leave a document by a route nobody would guess. More's "Choose a file" item closes the document and shows the home screen, though its label says it opens a file. Opening another file replaces the document, and removing `#url=` from the address closes it. This plan gives the reader a visible Close button that goes back to the home screen, a command behind it, and one close path that always asks about unsaved changes first.

## 2. What the reader does today (2026-11-13)

The pieces of a close exist; the reader lacks a button and a command for them.

- **The controller.** `ReaderController.requestClose()` asks the close guard, then calls `close()`, which drops any open still in flight, releases the document and returns the state to `empty` (`packages/ui/src/state/reader-controller.ts`).
- **The guard.** While an edit session has unsaved changes, the guard shows O03's dialog: Save, Don't save or Cancel (`apps/web/src/editing-runtime.tsx`). A `beforeunload` handler covers the browser tab itself.
- **The clean-up.** The session saves the reading position, clears the title, pages, outline and threads, tells the network panel, and closes the document in the worker, which stays ready for the next open (`apps/web/src/session.ts`, `packages/viewer/src/client.ts`). A remote file's `url` key leaves the fragment by `replaceState`.
- **The routes in.** More's "Choose a file" calls `requestClose()`. The fragment watcher and the failure page's "Try another" call `close()`, which skips the guard: removing `#url=` from the address throws away unsaved edits without asking, and no `beforeunload` fires for a fragment change.
- **The design.** O03 already names an in-app "Close document" on the web. The phone's and tablet's top bars start with **Back to files** (Reader B R17, R18), which Reader A's home screen lists as a way in; neither is built. The desktop-width top bar has no way out. The desktop app closes a document with its tab (`tab.close`, `Ctrl W`), a key the web leaves to the browser (design language §8.8).

## 3. What other readers do

Two patterns recur. Viewers inside a product, such as SharePoint's PDF viewer and PSPDFKit's mobile viewers, put a close or back control at the top left of the toolbar, and users judge it by where it takes them: SharePoint's sends readers to the document library instead of the page they came from, which its users report as a fault ([Microsoft Tech Community](https://techcommunity.microsoft.com/t5/sharepoint/redirecting-to-sharepoint-home-page-after-clicking-quot-x-quot/td-p/2278567)), and PSPDFKit hides its close button by default, leaving it to the app ([PSPDFKit guide](https://pspdfkit.com/guides/react-native/user-interface/close-button)). Tabbed apps, the desktop reader among them, put the close on the tab beside the file's name. Readers that lack any exit leave users stuck ([Adobe community](https://community.adobe.com/questions-567/after-opening-a-pdf-in-a-captivate-9-mobile-app-on-my-iphone-i-can-t-get-out-of-the-pdf-without-hitting-the-home-button-how-do-i-go-back-to-the-app-215350)). For lamina the destination is never in doubt, because the web reader has one place without a document, its home screen.

## 4. Decisions

1. **A close button beside the file name.** A 24 px icon button with a 14 px cross follows the file name in the top bar, named "Close Q3-board-report.pdf". The file name and its close read as one unit, like the desktop app's tab, so the web and desktop readers close a document from the same place. It shows whenever a document is open, opening, locked or failed, and never on the home screen.
2. **A command, `file.close`, titled "Close document".** The button, the palette and More's item all run it. It has no shortcut on the web, because `Ctrl W` and `⌘W` belong to the browser; the desktop app keeps `tab.close` on `Ctrl W`.
3. **More's items say what they do.** "Choose a file" becomes "Open another file…" (`file.open`, `Ctrl O`), which opens the picker as its shortcut chip always said; a new "Close document" item goes last.
4. **Phone and tablet build Back to files.** Their designed Back to files runs `file.close`, so touch readers get the platform's back gesture of a control, top left, and their More holds no Close item.
5. **Every close asks first.** The button, the command, the fragment watcher and "Try another" all go through `requestClose()`. When the address loses its `#url=` and the reader answers Cancel, the reader puts the fragment back with `replaceState`, so the address keeps naming the open file.
6. **After closing, focus and words.** Focus moves to the home screen's heading, and a polite live region says "Closed Q3-board-report.pdf". The window title goes back to "lamina". While "Remember recent files" is on, the default, Recent holds a local file with its reading position, which is how a reader reopens it; a linked file reopens from its link (ADR 0011). There is no undo.

## 5. Design

[Reader A R19, "Close a document"](../design/paper/reader-a.md#r19-close-a-document) holds the design, with three boards in `docs/design/paper/reader-a/project/`:

- `R19-close-button`: the top bar with the Close button under keyboard focus and its tooltip;
- `R19-close-more`: the web More menu with "Open another file…" and "Close document";
- `R19-closed-home`: the home screen right after a close, the file first in Recent.

O03 over the reader and the phone's and tablet's Back to files are drawn already (Edit session O03, Reader B R17 and R18).

## 6. Todos

- [ ] Every close asks about unsaved changes: the fragment watcher and "Try another" through `requestClose()`, with the fragment restored on Cancel ([C8a](../todos/ready/C8a-every-close-asks-first.md))
- [ ] `file.close`, the Close button, More's two items, Back to files on phone and tablet, focus and announcement ([C8b](../todos/ready/C8b-close-button-and-command.md))

## 7. Risks and open questions

- **A misplaced click.** The Close button sits in the file area, near the sidebar toggle; a stray click on a clean document closes it at once. Recent reopens the file at its page, so the cost is a click, not lost work. The owner decided against a toast with Reopen (2026-11-13).
- **Remote files.** A file opened by `#url=` reopens only through its link, since Recent never keeps a linked file (ADR 0011). Closing it removes the link from the address, as today.
- **An out-of-date board.** `R01-home` still draws a linked file in Recent, which ADR 0011 forbids; R19's home board leaves it out, and R01's board waits for its own fix.
