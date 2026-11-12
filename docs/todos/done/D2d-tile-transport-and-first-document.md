# D2d — Tile transport and the desktop engine client: open and read one document

| Field | Value |
| --- | --- |
| ID | `D2d` |
| Status | `done` |
| Plan | [Desktop app §6 D2](../../plans/desktop-app.md#d2-shell-and-native-core) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C3b](./C3b-viewer-virtualiser-and-zoom.md), [C4f](./C4f-reader-home-repairs-and-demo.md), [D2a](./D2a-adr-0008-and-desktop-scaffold.md), [D2b](./D2b-engine-process.md) |
| Dependency notes | [D2b](../done/D2b-engine-process.md); web plan **C3** (`@laminapdf/viewer` and its engine RPC interface, [web-app §C3](../../plans/web-app.md#c3-viewer-core)) and **C4** (`@laminapdf/ui`), both already required by D2a. Can run in parallel with [D2c](../done/D2c-engine-sandbox.md). |
| ADRs | Implements ADR 0008 §tile transport. Follows ADR 0009 (typed viewer RPC, web C2) and ADR 0010 (Platform adapter) |
| Branch | `feat/d2d-tile-transport` |
| PR title | `feat(desktop): stream engine tiles to the viewer and open a document` |
| Size | L (600–1,200 lines) |
| PR | [#421](https://github.com/laudebugs/laminapdf/pull/421) |

## Goal

Make the desktop app a working single-window reader: File ▸ Open (via a button for now) picks a PDF, the engine parses it, and `@laminapdf/viewer` scrolls and zooms it with tiles delivered over the transport ADR 0008 chose from D1a. The web reader's viewer code runs unchanged; only the engine client differs (`DesktopEngineClient` instead of the Web Worker RPC client). This proves the "~90 % shared UI" claim ([report](../../research/architecture/app-and-package-architecture.md#tauri-2-and-a-framework-light-web-client-share-90-of-the-ui)) and meets the 1 GB first-page target.

## Context

- ADR 0008 §tile transport and D1a's README (`spikes/desktop-ipc/`): primary/fallback transport, tile size, compression, in-flight limit.
- ADR 0009 / web C3: the viewer's engine interface (request IDs, priorities, `AbortSignal` cancellation, `ImageBitmap` painting). The desktop client must implement exactly that interface.
- D2b: `EngineSupervisor`, `DocumentRegistry`, `SessionPassword`, and the protocol (crate `laminapdf-desktop-protocol`): `RenderTile`/`Cancel`/`Tile`, `UnlockDocument`/`Unlocked`/`NeedsPassword`.
- [Desktop plan §4 non-functional](../../plans/desktop-app.md#non-functional-targets-and-release-gates): first page of a 1 GB PDF ≤ 1.5 s after open; idle memory ≤ 150 MB with one 10-page document.
- [Desktop notes Q1](../../research/architecture/notes/desktop-shell.md#q1-shell-options-versions-size-memory-startup-webview-differences-licence-maturity-production-users): Canvas2D only on Linux; no WebGL dependency.
- Tauri: `tauri::ipc::Channel`, `tauri::ipc::Response`, `InvokeResponseBody::Raw`, `register_asynchronous_uri_scheme_protocol` (per D1a's choice).

## Scope

**In scope**

- Rust: the transport chosen in ADR 0008, e.g. (adapt to the decision):
  - **Channel**: `#[tauri::command] async fn render_tiles(window: tauri::WebviewWindow, state: State<'_, EngineSupervisor>, doc: DocId, tiles: Vec<TileSpec>, on_tile: Channel<InvokeResponseBody>) -> Result<BatchId, DesktopError>`; tiles forwarded from the engine as `Raw(header ++ bytes)` without copying more than once; `cancel_tiles(batch | req ids)`.
  - **Custom scheme**: `lamina://localhost/tile/{window}/{doc}/{page}/{scale}/{x}/{y}/{gen}` (Windows `http://lamina.localhost/...`), handler validates the requesting webview's label matches `{window}` (no cross-window document access), returns `200` with the tile or `404`/`409` for stale generations; `Cache-Control: no-store` (the viewer has its own cache); cancellation via `AbortController` on `fetch` maps to dropping the engine job when the responder is dropped.
  - The fallback transport behind a runtime setting for diagnosis.
- `apps/desktop/src/engine/desktop-engine-client.ts`: implements the `@laminapdf/viewer` engine interface on top of `invoke` + the transport; maps `AbortSignal` to `cancel_tiles`; converts tiles to `ImageBitmap` (`createImageBitmap(new ImageData(...))` for raw RGBA, `createImageBitmap(blob)` for compressed); priority hints (visible > prefetch) forwarded to the engine.
- Desktop `Platform.openFile()` → `invoke('open_dialog')` → `DocId` + metadata; the viewer's document handle wraps the `DocId`.
- Password prompt: `@laminapdf/ui`'s prompt (from C4) → `invoke('unlock_document', { doc, password })` → main sends the protocol message `UnlockDocument { doc, password }` to the window's engine. On `Unlocked` the main process keeps the password as the document's `SessionPassword` (D2b: locked, zeroised on close, never logged or persisted) so an engine restart re-unlocks without a prompt; on `Error { kind: WrongPassword }` the copy is zeroised and the UI shows the retry prompt. The command's `String` argument is moved into `Zeroizing` immediately; the JS side drops its reference after the call.
- Engine restart UX: listen to `engine://restarted`, show the shared toast "Document process restarted", re-request visible tiles; on `engine://failed` show the error state with "Reopen".
- Error mapping: `DesktopError { kind }` → the UI's existing error notices (repair notice when `Opened.repaired`).
- Performance check: time-to-first-page on the 1 GB fixture and the C3 1,000-page fixture, idle memory (main + engine + webview) with a 10-page document; numbers in the PR and ADR 0008.

**Out of scope** (and which todo covers it)

- e2e automation of these flows: [D2e](../ready/D2e-desktop-e2e-webdriverio.md) (this PR includes a manual checklist in the PR).
- Text layer, search, thumbnails: arrive through shared web rounds C5/C6 and need engine messages for text extraction — a follow-up desktop todo once C5 lands (note in Risks).
- Opening from argv/OS/drag-drop: [D3a](../ready/D3a-open-from-os.md). Multiple documents/tabs: [D4a](../ready/D4a-windows-tabs-and-session-restore.md).

## Tasks

- [x] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2` and `rust-best-practices`.
- [x] Implement the transport (Rust) and register commands in `generate_handler!`, the app manifest in `build.rs`, and `capabilities/main.json` (`allow-render-tiles`, `allow-cancel-tiles`, `allow-open-dialog`, `allow-unlock-document`, `allow-page-info`, `allow-close-document`).
- [x] If a custom scheme is used: update the CSP `img-src`/`connect-src` for both URL forms; add `Cross-Origin-Resource-Policy: same-origin` if COOP/COEP are on.
- [x] Implement `DesktopEngineClient` (TS) with strict types shared with Rust via `tsify`-free hand-written types or generated with `ts-rs`/`specta` (decide; record in the PR; licence check), plus Vitest tests using `mockIPC`.
- [x] Wire `@laminapdf/ui`'s open button and password prompt through the desktop `Platform`.
- [x] Engine restart and failure events in the UI.
- [x] Measure: time-to-first-page (1 GB), scroll smoothness on the 1,000-page fixture using C3's perf harness adapted to the desktop build (dev-only page), idle memory; record in ADR 0008 and the PR. *Done on macOS, measured again under D2c's sandbox. Scroll smoothness isn't measured: D2e's scroll scenario covers the 1,000-page fixture on all three OSes.*
- [x] Manual checklist in the PR for macOS, Windows, Linux (X11 and Wayland): open, scroll, zoom 25–800 %, password file, repaired file, kill engine mid-scroll. *macOS by hand; the owner moved Windows and Linux to [D2e](../ready/D2e-desktop-e2e-webdriverio.md)'s scenarios, which run the same flows on all three OSes in CI.*

## Acceptance criteria

- [x] On all three OSes, opening a fixture through the open button renders the first page; scrolling and zooming paint sharp tiles; cancelled tile requests never paint (stale-generation check, unit-tested). *Checked by hand on macOS; CI builds the app on all three; D2e checks the rest on Windows and Linux.*
- [x] Cold start to an interactive window (empty state accepts input; the engine process may still be starting) is measured on the reference machine on all three OSes and recorded against the **< 300 ms** release gate ([desktop-app §4](../../plans/desktop-app.md#non-functional-targets-and-release-gates)); a miss opens a follow-up todo; from D2e on, the reference-runner assertion fails PRs that regress it. *Measured on macOS: 0.51–0.54 s, a miss; follow-up [D2g](../ready/D2g-desktop-cold-start.md). Windows and Linux are measured by D2e.*
- [x] The generated 1 GB fixture shows its first page within **1.5 s** after the open dialog closes on the reference machine (macOS and Windows; Linux recorded). *macOS: 0.53–0.74 s; Windows and Linux by D2e's 1 GB scenario.*
- [x] Idle memory with one 10-page document ≤ **150 MB** across main + engine (webview recorded separately; if over target, note it in ADR 0008 with a plan).
- [x] Encrypted fixtures open after the password prompt; a wrong password shows the retry prompt; the password never appears in logs (grep of log output in the test run).
- [x] Killing the engine mid-scroll shows "Document process restarted" and the same page re-renders without user action — **including for an encrypted document already unlocked** (no second password prompt; D2b's re-unlock).
- [x] A webview cannot fetch tiles of a document opened by another window (custom-scheme label check or channel ownership check; unit test).

## Post-merge checks

None.

## Tests

- Vitest: `DesktopEngineClient` — cancellation maps to `cancel_tiles`; stale generation discarded; restart event triggers re-request; errors mapped to UI notices (mock IPC via `@tauri-apps/api/mocks`).
- Rust unit tests: URL parsing/validation for the custom scheme (malformed, wrong window, out-of-range page), batch cancellation bookkeeping.
- Rust integration test: supervisor + engine render a tile for each `fixtures/` sample; checksum equals the in-process render.
- The e2e versions of the manual checklist are added in D2e.

## Docs to update

- ADR 0008: measurements (first page, memory) and the final transport details.
- `docs/plans/desktop-app.md` §9 D2: tick "Implement the chosen tile transport and mmap-backed `ByteSource`".
- `apps/desktop/README.md`: architecture diagram (webview ↔ main ↔ engine).

## Risks and open questions

- **WebKitGTK slow paths**: if Linux scrolling misses budgets, check the DMABUF renderer and `createImageBitmap` behaviour; the renderer switch arrives in D7b, but document env workarounds now.
- **Main-thread blocking**: Tauri runs non-`async` commands on the main thread — every command here must be `async` or offload to a thread.
- **Text layer parity** needs an engine-side text extraction message once web C5 lands; raise a follow-up todo (`D2f` or within the editor plan) when C5 is done — record in the PR.
- **Type sharing** between Rust and TS (hand-written vs generated): generated types reduce drift but add a dependency; decide here.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
