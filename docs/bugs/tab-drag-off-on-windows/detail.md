# Tabs couldn't be dragged on Windows

[Resolution](resolution.md).

## What happened

The desktop app's tab strip used HTML5 drag and drop to reorder tabs and move them to other windows. Each tab was `draggable`, and its `dragstart`, `dragover`, `drop` and `dragend` events drove the reorder and told Rust where the drag ended. By Tauri's own account, HTML5 drag and drop can't work on Windows, and on macOS and Linux the page could show only a "not allowed" cursor over windows that took the tab.

## Cause

Tauri replaces WebView2's own drag-and-drop handler so that it can report dropped files, and its source says what that costs: `WebviewWindowBuilder::disable_drag_drop_handler` "is required to use HTML5 drag and drop APIs on the frontend on Windows since we replace the drag drop handler of WebView2" (`tauri` 2.12.5, `src/webview/webview_window.rs`, line 1096). The app keeps that handler on, because opening a PDF dropped on a window needs it (`src-tauri/src/open.rs`). So on Windows, Tauri's source says a tab's drag never reaches a drop.

On macOS and Linux the drags ran, but an HTML5 drag shows the cursor for the drop effect of whatever is under it. Another window's strip refuses this one's tabs, and the desktop outside every window takes no drop, so the cursor would read "not allowed" exactly where the tab would go.

## How it was checked

From Tauri's source and the app's window setup, not by hand on Windows. No end-to-end scenario dragged tabs: WebDriver can't start an HTML5 drag, and D4e's checks used a real mouse on macOS and Linux. The fix's scenario runs on Windows in CI; dragging with a real mouse on each OS is a post-merge check of U7g.
