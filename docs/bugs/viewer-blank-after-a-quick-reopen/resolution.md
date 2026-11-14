# The view stays blank after a quick reopen: resolution

## The fix

The viewer's scheduled frame now places a pending view once the view has a size (`packages/viewer/src/view/viewer.ts`, `#schedule`). Before, that frame only skipped the update while a view was pending, and only the `ResizeObserver` could place it. Now the two cover each other:

- **Hidden through a frame:** the observer saw the hidden size, so it reports the size the view comes back with, and places it as before.
- **Hidden and shown between two frames:** the observer reports nothing, and the frame the open scheduled finds the view sized and places it.

## The tests

- **The reproduction** (`packages/viewer/test/browser/navigation.browser.test.ts`, "draws the new document when it is hidden and shown again before the next frame"). It opens a document, then hides the view, opens another and shows the view again in one task, as a quick reopen does, and waits for the new document's first page to be drawn. Without the fix it fails every time in Chromium; with it, it passes, and so do the other 14 tests in the file.
- **The reader's close tests** pass with the fix, alone and in the whole Chromium e2e suite.
- **What a stuck wait shows:** the reader's e2e helpers (`apps/web/e2e/reader.ts`) now keep the page's console errors and warnings, uncaught errors, crashes and worker starts, and `stuck()` turns a timed-out wait into an error with the step, the window title, the view box's state, any dialog, what the reader last announced and the last console lines. That report found this cause, where a bare timeout had only said a canvas was hidden.

## Left for later

- **The `#url=` dialog failure** ([detail](detail.md#not-yet-explained)) isn't explained yet; `stuck()` wraps its wait, so its next failure reports what the reader showed.
