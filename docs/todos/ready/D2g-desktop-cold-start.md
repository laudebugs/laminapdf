# D2g — Desktop cold start under 300 ms

| Field | Value |
| --- | --- |
| ID | `D2g` |
| Status | `ready` |
| Plan | [Desktop app §4 release gates](../../plans/desktop-app.md#non-functional-targets-and-release-gates) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D2d](../ongoing/D2d-tile-transport-and-first-document.md) |
| ADRs | Records its numbers in [0008](../../decisions/0008-desktop-shell.md) §4 |
| Branch | `perf/d2g-desktop-cold-start` |
| PR title | `perf(desktop): reach an interactive window within 300 ms of launch` |
| Size | S–M (measure first; the fix decides) |
| PR | — |

## Goal

Bring the desktop reader's cold start, from launch to an empty window that accepts input, under the plan's **300 ms** release gate. D2d measured 0.49–0.68 s on an M1 Pro (macOS 27, release build, five runs), so the gate blocks the beta ([D7c](./D7c-desktop-beta-release.md)) until this todo lands.

## Context

- D2d's split, from the same runs: the main process logs "Lamina PDF started" (Tauri's `setup`) 0.25–0.4 s after launch (0.25 s in four runs of five), and the window's first command arrives 0.24–0.28 s after that. The second part is the webview loading, parsing and running the frontend: 600 KB of JavaScript (187 KB gzipped) and 97 KB of CSS before React's first render.
- The build also copies 7 MB of WebAssembly (`lamina_pdf_*.wasm`) into `dist/`, which Vite emits from the viewer's dependencies. No startup code loads it, but it grows the bundle and the installer.
- The engine starts on the first document, not at launch, so it isn't part of cold start (it takes 10–300 ms to become ready).

## Tasks

- [ ] Measure where the time goes on all three OSes: launch to `setup`, `setup` to the window's first paint, first paint to the first command. Add a dev-only timing log (not in release builds) or an `e2e` command that D2e's reference runner can assert.
- [ ] Shrink the main process's time to `setup`: plugin initialisation, logging setup, and anything else run before the window exists.
- [ ] Shrink the frontend's startup: split code the empty state doesn't need (the viewer, the password dialog) out of the first chunk, and keep the WebAssembly out of the desktop build.
- [ ] Record the numbers in ADR 0008 §4 and the PR; from D2e on, the reference runner fails a PR that regresses them.

## Acceptance criteria

- [ ] Launch to an interactive empty window is under 300 ms on the reference machine on macOS, Windows and Linux.
- [ ] The desktop `dist/` contains no `.wasm` file.
