# D1a — `desktop-ipc` spike: tile transport and COOP/COEP in the three webviews

| Field | Value |
| --- | --- |
| ID | `D1a` |
| Status | `ready` |
| Plan | [Desktop app §6 D1](../../plans/desktop-app.md#d1-ipc-and-print-spikes) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C2c](../done/C2c-render-packages.md), [C3b](./C3b-viewer-virtualiser-and-zoom.md) |
| Blocks | [D1b](./D1b-desktop-print-spike.md), [D2a](./D2a-adr-0008-and-desktop-scaffold.md) |
| Dependency notes | Web plan **C3** (viewer core, [web-app §C3](../../plans/web-app.md#c3-viewer-core)): the spike's webview half reuses the `@laminapdf/viewer` canvas pool / `ImageBitmap` paint path so the numbers match what the reader will do. C3 brings C2's `lamina-render` with it. |
| ADRs | Informs ADR 0008 (desktop shell, written in D2a). Follows [0001](../../decisions/0001-core-language-and-licence.md), [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (spikes live outside both workspaces), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `spike/d1a-desktop-ipc` |
| PR title | `spike(desktop): measure Tauri tile transports and cross-origin isolation` |
| Size | M (200–600 lines of spike code plus the README with results) |
| PR | — |

## Goal

Answer, with numbers from all three desktop webviews (WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux), the two questions ADR 0008 can't be written without. First, **which transport moves rendered page tiles from Rust into the webview fastest and most smoothly**: a `tauri::ipc::Channel` carrying raw RGBA, a command returning `tauri::ipc::Response` (raw body, no JSON), or a custom URI scheme `lamina://tile/...` that the webview fetches like an image (raw RGBA or compressed). Second, **does each webview report `crossOriginIsolated === true`** when Tauri sets COOP/COEP through `app.security.headers`, which decides whether the desktop app can use the threaded WASM variant for parity testing and `SharedArrayBuffer` in the UI. The research could not find any benchmark of these on all three webviews ([desktop notes Q3](../../research/architecture/notes/desktop-shell.md#q3-performance-native-rust-over-ipc-vs-wasm-in-webview-bitmaps-1-gb-files-mmap-file-watching), [report: open questions](../../research/architecture/app-and-package-architecture.md#open-questions-the-research-could-not-close)), and getting this wrong means a reader that stutters on zoom.

## Context

- [Desktop plan §4 req. 4 and §6 D1](../../plans/desktop-app.md#d1-ipc-and-print-spikes): the three candidate transports and the acceptance bar ("READMEs report numbers per OS and recommend a tile transport").
- [Architecture §4 spikes](../../plans/architecture.md#4-spikes-to-close-open-questions): `desktop-ipc` feeds ADR 0008.
- [Desktop notes Q3](../../research/architecture/notes/desktop-shell.md#q3-performance-native-rust-over-ipc-vs-wasm-in-webview-bitmaps-1-gb-files-mmap-file-watching): serde returns are JSON (one number per byte); `ipc::Response` is raw; Channels are Tauri's recommended streaming mechanism; v2 IPC still goes through `fetch`; the third-party `tauri-conduit` figure (64 KB: ~202 µs binary vs 2.27 ms invoke) is self-reported and must not be cited as our result.
- [Report: Tauri section](../../research/architecture/app-and-package-architecture.md#tauri-2-is-the-desktop-shell-with-electron-held-in-reserve-for-linux): 256–512 px tiles; A4 at 2× DPR is ~15 MB of RGBA, so only the viewport is transferred.
- [Desktop notes Q1](../../research/architecture/notes/desktop-shell.md#q1-shell-options-versions-size-memory-startup-webview-differences-licence-maturity-production-users): WebKitGTK canvas can silently fall back to a slow path; DMABUF/NVIDIA issues and their env workarounds. The Linux run must record which renderer was active.
- Tauri docs: [calling Rust](https://v2.tauri.app/develop/calling-rust/) (Channels, `ipc::Response`, `InvokeBody::Raw`), [HTTP headers](https://v2.tauri.app/security/http-headers/) (`app.security.headers`, Tauri ≥ 2.1), custom protocols via `tauri::Builder::register_asynchronous_uri_scheme_protocol`.
- [Code organisation](../../guidelines/code-organization.md): `spikes/` sit outside both workspaces; throwaway quality, but the README is the deliverable. Existing spikes to copy the README shape from: `spikes/krilla-fit/README.md`.
- The `tauri-v2` and `rust-best-practices` skills (CLAUDE.md requires both for this work).

## Scope

**In scope**

- `spikes/desktop-ipc/`: a minimal Tauri 2 app (own `Cargo.toml` + `package.json`, not a workspace member) whose Rust side generates tiles and whose webview paints them on a `<canvas>` with Canvas2D (`drawImage` of an `ImageBitmap`), exactly the reader's baseline paint path. No WebGL/WebGPU (Linux constraint).
- Tile sources: (a) synthetic tiles (deterministic gradient + noise so compression is realistic but not trivial) and (b) real pages rendered with `lamina-render`'s PDFium adapter from 3 `fixtures/` files (text-heavy, image-heavy, vector-heavy), so compressed-size numbers are honest.
- Transports, each behind a URL switch in the spike UI:
  1. **Channel / raw RGBA**: `#[tauri::command] async fn stream_tiles(req: TileBatch, on_tile: tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>)`, sending `InvokeResponseBody::Raw(header ++ rgba)`; JS side `new Channel<ArrayBuffer>()` → `new ImageData(new Uint8ClampedArray(buf, 16), w, h)` → `createImageBitmap`.
  2. **`ipc::Response` / raw RGBA**: one `invoke('tile', {...})` per tile returning `tauri::ipc::Response::new(bytes)`; JS receives an `ArrayBuffer`.
  3. **Custom scheme, raw RGBA**: `lamina://localhost/tile/{doc}/{page}/{scale}/{x}/{y}` (Windows/WebView2: `http://lamina.localhost/...`) fetched with `fetch()`; body is RGBA with `Content-Type: application/octet-stream`.
  4. **Custom scheme, compressed**: same URL with `?fmt=png|webp|qoi`, decoded by `createImageBitmap(blob)` off the main thread (or `<img>` decode). Encoders: `png` (fast settings), `image-webp` lossless, `qoi`; record encode time in Rust separately.
- Tile sizes 256 and 512 px; DPR 1 and 2.
- A scripted scroll + zoom benchmark (fixed sequence: 40 pages scrolled at a constant velocity, then zoom 100 % → 200 % → 400 % with anchor) driven from JS with `requestAnimationFrame` timestamps.
- Measurements per OS × transport × tile size: tiles/s and MB/s sustained; p50/p95/p99 latency request → painted; dropped frames and long tasks (> 50 ms, `PerformanceObserver('longtask')` where supported, rAF gap histogram elsewhere); main-process and webview-process peak RSS; CPU % during the run.
- COOP/COEP check: `app.security.headers` with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`; report `self.crossOriginIsolated`, whether `new SharedArrayBuffer(1)` works, and whether the custom-scheme tile responses still load under COEP (they need `Cross-Origin-Resource-Policy: same-origin` or same origin).
- Linux runs on both X11 and Wayland, recording `WEBKIT_DISABLE_DMABUF_RENDERER` on/off and GPU (Intel/AMD vs NVIDIA if available).
- README with method, raw numbers (CSV committed under `spikes/desktop-ipc/results/`), charts optional, and a clear **recommendation** for ADR 0008 (primary transport, fallback transport, tile size, compression yes/no, and whether COOP/COEP is usable per OS).

**Out of scope** (and which todo covers it)

- The production transport, `Platform` adapter and engine process: [D2b](D2b-engine-process.md), [D2d](D2d-tile-transport-and-first-document.md).
- Writing ADR 0008: [D2a](D2a-adr-0008-and-desktop-scaffold.md).
- Printing: [D1b](D1b-desktop-print-spike.md).
- Engine-process ↔ main-process IPC (pipes/shared memory): measured in D2b, not here (this spike measures the Rust → webview hop only).
- CI for the spike: spikes are not in CI (like `krilla-fit`).

## Tasks

- [ ] Create a fresh worktree off `main` (`git worktree add -b spike/d1a-desktop-ipc spike-d1a-desktop-ipc main`); this todo is already claimed via the claim PR (see DoD), so do not move it here.
- [ ] Load the `tauri-v2` and `rust-best-practices` skills.
- [ ] Scaffold `spikes/desktop-ipc/` with `pnpm create tauri-app` (vanilla TS + Vite), then trim: `src-tauri/src/main.rs` thin (`fn main() { desktop_ipc::run() }`), all logic in `src-tauri/src/lib.rs`; `[lib] crate-type = ["staticlib", "cdylib", "rlib"]`; pin `tauri = "2.x.y"` and `@tauri-apps/api`/`@tauri-apps/cli` to the same minor (record versions in the README).
- [ ] Add `spikes/desktop-ipc` to the list of excluded paths so neither the Cargo workspace nor `pnpm-workspace.yaml` picks it up (check `Cargo.toml` `exclude` and the spike convention in `spikes/krilla-fit`).
- [ ] `tauri.conf.json`: `identifier: "dev.lamina.spike.ipc"`, one window `main`, `app.security.csp` restricting `img-src`/`connect-src` to `'self' lamina: http://lamina.localhost ipc: http://ipc.localhost`, and a second config variant (`tauri.coi.conf.json`, merged with `--config`) adding `app.security.headers` for COOP/COEP/CORP.
- [ ] `src-tauri/capabilities/default.json`: `core:default` only; app commands exposed through an app manifest in `build.rs` (`tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&["tile", "stream_tiles", "metrics"])))`) and granted as `allow-tile`, `allow-stream-tiles`, `allow-metrics` — this also rehearses the permission model D2a adopts.
- [ ] `src-tauri/src/tiles.rs`: `TileRequest { doc: u32, page: u32, scale_milli: u32, x: u32, y: u32, size: u16 }`; a `TileSource` trait with `Synthetic` and `Pdfium` impls (PDFium via `pdfium-render`, dynamic library path from an env var; render a page once per scale and cut tiles from it, cache in an LRU so the spike measures transport, not rendering — report render time separately).
- [ ] Implement the four transports (see Scope). For the custom scheme use `register_asynchronous_uri_scheme_protocol("lamina", |_ctx, request, responder| { spawn(async move { ... responder.respond(response) }) })` so encoding never blocks the main thread; set `Cache-Control: no-store` for the benchmark and `Cross-Origin-Resource-Policy: same-origin`.
- [ ] Webview side (`src/bench.ts`): the fixed scroll/zoom script, a tile scheduler limited to N in-flight requests (N = 4, 8, 16 — report the best per transport), paint via `createImageBitmap` + `drawImage`, and metrics collection (`performance.now()`, `PerformanceObserver`, rAF gaps). Results posted back with a `metrics` command and written by Rust to `results/{os}-{transport}-{size}-{dpr}.csv`.
- [ ] `src/coi.ts`: report `crossOriginIsolated`, `SharedArrayBuffer` availability, and whether a `lamina://` fetch and an `ipc` invoke still work under COEP.
- [ ] `scripts/run-all.{sh,ps1}`: build release (`pnpm tauri build --debug=false --no-bundle`), run each combination 5 times, collect CSVs. Record machine specs (CPU, GPU, RAM, display scale, OS build, webview version: `WKWebView` via macOS version, WebView2 runtime version, `webkit2gtk-4.1` package version).
- [ ] Run on: macOS (Apple Silicon, macOS 13+), Windows 11 x64, Ubuntu 22.04/24.04 x64 (X11 and Wayland). Note anything that differed from the docs (e.g. scheme URL form on Windows, `fetch` of custom schemes on WebKitGTK).
- [ ] `spikes/desktop-ipc/README.md`: question, method, environment table, results tables (median of 5 runs), observations per OS, **Recommendation** section phrased as inputs to ADR 0008, and "Limits of this spike".
- [ ] Update `docs/plans/desktop-app.md` (tick the D1 IPC to-dos, link the README), the architecture §4 spike row (mark done, link result), `spikes/` mention in `CLAUDE.md` layout tree.

## Acceptance criteria

- [ ] `spikes/desktop-ipc/README.md` contains, for **each of macOS, Windows and Linux**, a table with throughput (tiles/s, MB/s), p50/p95 latency and dropped frames for every transport at 256 px and 512 px, DPR 1 and 2.
- [ ] The README states `crossOriginIsolated` (true/false) per OS with and without the headers, and whether custom-scheme tiles load under COEP.
- [ ] The Linux section reports X11 vs Wayland and DMABUF renderer on/off.
- [ ] A **Recommendation** names one primary transport, one fallback, the tile size and the compression choice, each justified by a number in the tables, and lists what ADR 0008 must record.
- [ ] Raw CSVs committed under `spikes/desktop-ipc/results/` and reproducible with `scripts/run-all.*`.
- [ ] The spike builds with `cargo build --release` inside `spikes/desktop-ipc/src-tauri` and `pnpm tauri build --no-bundle`; no change to the root workspaces' CI.

## Post-merge checks

None.

## Tests

Spike code: no test suite required. Sanity checks inside the benchmark: every received tile's dimensions and a checksum of the first row are compared to what Rust sent (mismatch aborts the run), so a transport that corrupts bytes cannot "win".

## Docs to update

- `docs/plans/desktop-app.md` §9 D1: tick the three IPC to-dos; link the README from §6 D1.
- `docs/plans/architecture.md` §4: `desktop-ipc` row → **Done** with a link to the result (one line).
- `CLAUDE.md`: add `desktop-ipc/` to the `spikes/` tree with a one-line description.
- `docs/todos/README.md` board (move the entry) as part of the move to `done/`.

## Risks and open questions

- **Machine variance.** Results from one laptop per OS are indicative only; record specs and run 5×. If two transports are within 15 %, prefer the simpler one (decided by the PR author, noted in the README).
- **Windows custom-scheme URL form** (`http://lamina.localhost`) and WebView2's handling of large bodies may change with the Evergreen runtime; record the runtime version.
- **WebKitGTK** may not fire `longtask` entries; fall back to rAF gap analysis and say so.
- **NVIDIA hardware** may be unavailable; if so, mark the gap and list it as a follow-up for D2e's CI matrix.
- **COOP/COEP on custom protocols**: if any webview ignores `app.security.headers`, ADR 0008 must record "single-threaded only on that OS" — not a blocker, because the native core does not need WASM threads.
- Licence check: `pdfium-render` (MIT OR Apache-2.0), PDFium binaries (BSD-3/Apache-2.0), `qoi`/`image-webp`/`png` (MIT/Apache) — confirm with `cargo deny check licenses` run locally inside the spike.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
