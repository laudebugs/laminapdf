# Resolution: tabs couldn't be dragged on Windows

[Diagnosis](detail.md).

## The fix

The strip drives the drag with pointer events (`packages/ui/src/reader/tab-drag.ts`), which the webview's drop handler doesn't touch:

- A press on a tab that moves 4 pixels starts a drag; a press that doesn't is a click and shows the tab. Mouse and pen only, so a finger still scrolls the strip.
- The tab captures the pointer, so its moves and its release arrive even outside the window. The listeners sit on the document, so WebDriver's dispatched events reach them too.
- Over the strip an accent bar marks where the tab would go, and a release there reorders the tabs.
- A release anywhere else goes to the shell as before (`onDragOut`), and Rust moves the tab to the window under the cursor or to a new one (`src-tauri/src/tab_drag.rs`).
- Escape cancels the drag: the tab stays, and the new `tab_drag_cancel` command stops Rust following the cursor. Before, a cancelled HTML5 drag ending outside the window moved the tab anyway.
- The whole page shows the `grabbing` cursor while a tab is dragged, and a draggable tab shows `grab`.

The keyboard and the strip's menu move tabs as before.

## How it was checked

- `packages/ui/test/browser/document-tabs.browser.test.tsx`, in Chromium, Firefox and WebKit: a reorder with Playwright's mouse; with dispatched pointer events, the marker, the `grabbing` cursor, a release off the strip inside and outside the page, Escape, a click that doesn't move, and a drop on another strip.
- `packages/ui/test/tab-drag.test.ts`: where a dropped tab lands.
- A real mouse on macOS (CGEvent): the reorder, the marker, the dimmed tab, the `grabbing` cursor, and a drag out to a new window.
- `apps/desktop/e2e/specs/tab-drag.e2e.ts`, on Linux, macOS and Windows in CI: a reorder, a drag out to a new window and a drag onto another window, with WebDriver's pointer actions and the cursor Rust reads placed by `e2e_place_cursor`.

## Left for later

- Dragging with a real mouse on Windows and Linux, U7g's post-merge check.
- The `grabbing` cursor shows in the window the tab comes from. Over another window, or outside every window, the OS draws its ordinary cursor, which the page can't set; no "not allowed" cursor shows anywhere.
- A strip with more tabs than fit doesn't scroll while a tab is dragged along it, as it didn't with HTML5 drag and drop.
