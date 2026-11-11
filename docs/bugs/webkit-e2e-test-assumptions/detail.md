# Two WebKit e2e tests checked something that can't happen

## What happened

Two of the reader's e2e tests failed in WebKit while the reader behaved correctly:

- `devices.e2e.test.ts`, "iPad Pro 11: zooms on a double tap and keeps the point under a pinch within 2 px", failed about one run in two, on main and on pull requests, locally and in CI. The pinch ended at 800 % instead of 175 % of where it started.
- `selection.e2e.test.ts`, "lists the selection's commands in the palette", failed every time in WebKit on Linux: the palette never opened.

## Causes

The pinch test sent its first finger as `pointerId` 1, which is the mouse's id in WebKit. When the page moved under the resting mouse, WebKit sent the mouse a `pointermove` with that id, and the viewer took it for the finger jumping 150 px. Logging every `setZoom` call showed the jump: 270 % to 769 % from a `pointermove` the test never dispatched. Pointer ids are unique among active pointers, so a real touch never shares the mouse's id.

The palette test pressed Meta+K whenever the browser was WebKit, which is the shortcut only on a Mac. On Linux the reader opens the palette with Control+K.
