# D2g — Desktop cold start under 300 ms

| Field | Value |
| --- | --- |
| ID | `D2g` |
| Status | `done` |
| Plan | [Desktop app §4 release gates](../../plans/desktop-app.md#non-functional-targets-and-release-gates) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D2d](./D2d-tile-transport-and-first-document.md) |
| ADRs | Records its numbers in [0008](../../decisions/0008-desktop-shell.md) §4 |
| Branch | `perf/d2g-desktop-cold-start` |
| PR title | `perf(desktop): reach an interactive window within 300 ms of launch` |
| Size | S–M (measure first; the fix decides) |
| PR | [#456](https://github.com/laudebugs/laminapdf/pull/456) |

## Goal

Bring the desktop reader's cold start, from launch to an empty window that accepts input, under the plan's **300 ms** release gate. D2d measured 0.51–0.54 s on an M1 Pro (macOS 27, release build, five runs, the engine sandboxed), so the gate blocks the beta ([D7c](../ready/D7c-desktop-beta-release.md)) until this todo lands.

## Context

- D2d's split, from the same runs: the main process logs "Lamina PDF started" (Tauri's `setup`) 0.25–0.4 s after launch (0.25 s in four runs of five), and the window's first command arrives 0.24–0.28 s after that. The second part is the webview loading, parsing and running the frontend: 600 KB of JavaScript (187 KB gzipped) and 97 KB of CSS before React's first render.
- The build also copies 7 MB of WebAssembly (`lamina_pdf_*.wasm`) into `dist/`, which Vite emits from the viewer's dependencies. No startup code loads it, but it grows the bundle and the installer.
- The engine starts on the first document, not at launch, so it isn't part of cold start (it takes 10–300 ms to become ready).

## Tasks

- [x] Measure where the time goes on all three OSes: launch to `setup`, `setup` to the window's first paint, first paint to the first command. Add a dev-only timing log (not in release builds) or an `e2e` command that D2e's reference runner can assert. *`src-tauri/src/startup.rs`: in `e2e` builds the first command logs `run`, `setup`, the window and the command; `tools/desktop/cold_start.py` adds launch → `run` and compares two builds in turn. Measured on macOS; Windows and Linux are D2e's, whose runners read the same log.*
- [x] Shrink the main process's time to `setup`: plugin initialisation, logging setup, and anything else run before the window exists. *Nothing to shrink: the app's code before `setup` takes about 1 ms and the log plugin made no measurable difference; the rest is Tauri building the app and starting its event loop, as long in the floor build.*
- [x] Shrink the frontend's startup: split code the empty state doesn't need (the viewer, the password dialog) out of the first chunk, and keep the WebAssembly out of the desktop build. *WebAssembly dropped from the build (`vite.config.ts`'s `dropUnusedWasm`). Not split: a build without the viewer at all was 13 ms faster to the first command, which doesn't pay for loading it on open.*
- [x] Record the numbers in ADR 0008 §4 and the PR; from D2e on, the reference runner fails a PR that regresses them. *ADR 0008 §4, "Cold start (D2g)"; the assertion lands with D2e.*

## Acceptance criteria

- [x] ~~Launch to an interactive empty window is under 300 ms on the reference machine on macOS, Windows and Linux.~~ *Replaced by the owner on 2026-11-11 (ADR 0008 §4): Tauri and the webview alone take 431 ms (the floor build) on the M1 Pro. The gates are now launch → window under 300 ms (241 ms on macOS: met) and launch → first command within 50 ms of the floor (55 ms on macOS: missed, see ADR 0008 §4); Windows and Linux are measured by D2e's runners.*
- [x] The desktop `dist/` contains no `.wasm` file. *Checked by the full lane's `desktop` job.*
