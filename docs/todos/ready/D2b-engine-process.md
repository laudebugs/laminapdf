# D2b — Engine child process: protocol, mmap byte source, restart on crash

| Field | Value |
| --- | --- |
| ID | `D2b` |
| Status | `ready` |
| Plan | [Desktop app §6 D2](../../plans/desktop-app.md#d2-shell-and-native-core) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [C4c](./C4c-remote-files-encryption-privacy.md), [C4d](./C4d-deploy-previews-and-production.md), [D2a](./D2a-adr-0008-and-desktop-scaffold.md) |
| Blocks | [D2c](./D2c-engine-sandbox.md), [D2d](./D2d-tile-transport-and-first-document.md) |
| Dependency notes | [D2a](D2a-adr-0008-and-desktop-scaffold.md); web plan **C2** (`crates/lamina-render` with the PDFium adapter, [web-app §C2](../../plans/web-app.md#c2-render-backend-and-adrs)) — satisfied transitively by C4 via D2a |
| ADRs | Implements ADR 0008 §process model (D2a). Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (`Limits` apply in the engine), [0001](../../decisions/0001-core-language-and-licence.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md), ADR 0004 (`RenderBackend`, web C2) |
| Branch | `feat/d2b-engine-process` |
| PR title | `feat(desktop): run parsing and rendering in a restartable engine process` |
| Size | L (600–1,500 lines) |
| PR | — |

## Goal

Move every byte of untrusted PDF processing out of the Tauri main process. This PR adds the engine binary, the framed binary protocol between main and engine, a memory-mapped `ByteSource` owned by the main process that serves range reads to the engine, an engine supervisor that spawns one engine per window and restarts it after a crash, and the Rust-side API the tile transport (D2d) will call. A hostile PDF that crashes PDFium or the lamina parser must take down only the engine, never the app — the same model as Chrome's out-of-process PDF viewer ([report](../../research/architecture/app-and-package-architecture.md#tauri-2-is-the-desktop-shell-with-electron-held-in-reserve-for-linux)).

## Context

- ADR 0008 (written in D2a): process model, engine binary location (`crates/lamina-desktop-engine` as a sidecar), ownership of file handles.
- [Desktop plan §4 req. 3](../../plans/desktop-app.md#functional): "Parse and render in a child process; a crash shows 'document process restarted' and reloads the document." [§5](../../plans/desktop-app.md#5-design-summary): main process does file I/O only (mmap, watching); tiles 256–512 px.
- [Architecture §7 Round D to-dos](../../plans/architecture.md#round-d--desktop-reader): main process "passes bytes and tiles across the boundary".
- [Report: seams](../../research/architecture/app-and-package-architecture.md#the-reference-architecture-is-one-engine-with-three-shells): `ByteSource` (range reads; mmap on desktop) and the handle-based façade with owned types and byte buffers.
- The `lamina` façade crate ([library plan B5](../../plans/pdf-library.md#b5-builder-and-the-lamina-crate)) and `lamina-render` (web C2): what the engine links.
- [ADR 0003](../../decisions/0003-reader-tolerance-and-limits.md): `Limits` defaults — the engine applies them; memory caps per engine are an extra OS-level guard (D2c).
- [Rust guidelines](../../guidelines/rust.md): `unsafe` only in edge/app crates with `// SAFETY:`; `memmap2::Mmap::map` is `unsafe` (file may change underneath — see risks).
- `memmap2` (MIT/Apache), `postcard` (MIT/Apache) or `bincode` 2 (MIT), `interprocess` (MIT/Apache) if pipes beyond stdio are needed.

## Scope

**In scope**

- `crates/lamina-desktop-engine/` (layer `app`, `publish = false`):
  - **lib**: protocol types and framing (`protocol.rs`), shared by `src-tauri` and the engine bin. Messages (versioned, `PROTOCOL_VERSION: u16`):
    - Main → engine: `Hello { version, limits }`, `OpenDocument { doc: DocId, len: u64, password: Option<Zeroizing<String>> }`, `RangeData { req: ReqId, offset: u64, bytes }`, `GetPageInfo { doc }`, `RenderTile { req, doc, page, scale_milli, x, y, w, h, format }`, `Cancel { req }`, `CloseDocument { doc }`, `Shutdown`.
    - Engine → main: `Ready { version, pid }`, `ReadRange { req, doc, offset, len }`, `Opened { doc, page_count, encrypted, repaired }`, `PageInfo { doc, sizes: Vec<(f32, f32, u16)> }`, `Tile { req, w, h, format, bytes }`, `Error { req, kind, message }`, `Heartbeat`.
    - Framing: `u32` little-endian length prefix + `postcard` payload, max frame size constant (e.g. 64 MiB) rejected before allocation; tile payloads carried as raw bytes after the header (no per-byte serialisation).
  - **bin** `lamina-desktop-engine`: reads frames from stdin, writes to stdout, logs to stderr (captured by main into `tauri-plugin-log`, never document content). Implements a `ByteSource` whose reads become `ReadRange` requests answered by the main process (synchronous from the parser's view, with a small read-ahead cache of e.g. 256 KiB blocks). Opens documents through the `lamina` façade and renders with `lamina-render`'s PDFium adapter; render jobs on a worker pool (`std::thread` — this is an app binary, so threads are fine) with cancellation by `ReqId`.
  - PDFium dynamic library located via an argument from main (`--pdfium <path>`), path from the app's resource dir.
- `apps/desktop/src-tauri/src/engine/`:
  - `supervisor.rs`: `EngineSupervisor` (Tauri managed state) spawning the sidecar with `std::process::Command` (path resolved next to the main executable as Tauri places `externalBin` sidecars; not through `tauri-plugin-shell`, which stays unregistered), piped stdio, one engine per window label; reader thread demultiplexing responses to `oneshot`/channel waiters; heartbeat watchdog; **restart policy**: on unexpected exit or missed heartbeats (e.g. 3 × 2 s), respawn with exponential backoff, at most 3 restarts in 60 s, then mark the window's engine failed.
  - `bytesource.rs`: `MmapSource` — opens the file read-only, `memmap2::Mmap` (with `// SAFETY:` explaining the truncation hazard and mitigation), serves `ReadRange` by copying the slice; falls back to positioned reads (`FileExt::read_at` / `seek_read`) for files on network shares or when mapping fails.
  - `documents.rs`: `DocumentRegistry` mapping `DocId` → (granted path, `MmapSource`, owning window, last view state). Paths enter the registry only from trusted sources (dialog opened from Rust, argv/Opened in D3a, recent list) — the webview never sends a path.
  - On engine restart: re-send `Hello`, re-open every document of that window, emit a Tauri event `engine://restarted` `{ window, docs }` to that window only (`emit_to`), so the UI shows "document process restarted" and re-requests visible tiles.
- Commands (each added to the app manifest and granted in `capabilities/main.json`): `open_dialog` (Rust-side `DialogExt::file().add_filter("PDF", &["pdf"]).blocking_pick_file()` on a background thread → registers the doc → returns `DocId` + metadata), `page_info`, `close_document`. Test-only command `debug_kill_engine` compiled only with the Cargo feature `e2e` (used by D2e).
- `bundle.externalBin: ["binaries/lamina-desktop-engine"]` in `tauri.conf.json` and an xtask step (`cargo xtask desktop-sidecar`) that builds the engine for the host/target triple and copies it to `src-tauri/binaries/lamina-desktop-engine-<target-triple>[.exe]` (Tauri's required naming), run from `beforeDevCommand`/`beforeBuildCommand`.
- PDFium shared library shipped via `bundle.resources` (per-OS prebuilt from a pinned, hash-checked release of bblanchon/pdfium-binaries — same pin as web C2's `@lamina/render-pdfium`).
- Measurements recorded in the PR: engine RSS idle and with a 1,000-page document; `ReadRange` throughput on the 1 GB fixture; time from `OpenDocument` to `Opened` and first `PageInfo` for the 1 GB fixture.

**Out of scope** (and which todo covers it)

- OS-level sandboxing of the engine: [D2c](D2c-engine-sandbox.md) (this PR runs it unsandboxed but already without any file paths).
- Delivering tiles to the webview and the viewer integration: [D2d](D2d-tile-transport-and-first-document.md).
- WebdriverIO e2e: [D2e](D2e-desktop-e2e-webdriverio.md).
- File watching (the registry exposes a "reload" entry point used by [D3b](D3b-recent-files-and-file-watching.md)).
- Crash reports for engine crashes: [D7a](D7a-crash-reporting.md) (this PR only logs exit status).
- Pooling engines across windows (ADR 0008 open question 3): revisit after measurements.

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2` and `rust-best-practices`.
- [ ] Create `crates/lamina-desktop-engine` (`Cargo.toml` with `[lib]` and `[[bin]]`, layer `app`, `publish = false`, licence `MIT OR Apache-2.0`); add to workspace deps.
- [ ] Protocol module with `Frame` encode/decode (`fn write_frame<W: Write>(w, msg: &Msg, payload: &[u8]) -> Result<(), ProtocolError>`; `fn read_frame<R: Read>(r, max: u32) -> Result<(Msg, Vec<u8>), ProtocolError>`), version check, `ProtocolError` via `thiserror`.
- [ ] Engine main loop: single reader thread, job queue with priorities (visible tiles > prefetch), cancel set, writer serialised through one `Mutex<StdoutLock>` or a channel to a writer thread.
- [ ] `RemoteByteSource` in the engine implementing the façade's `ByteSource` trait (blocking request/response with a timeout; `Error::Io` on timeout so the parser fails cleanly).
- [ ] Supervisor, `MmapSource`, `DocumentRegistry`, commands, event emission as above; all state behind `Mutex`/`RwLock` with no lock held across IPC waits.
- [ ] `debug_kill_engine` behind `#[cfg(feature = "e2e")]`, with the feature off in release builds (assert in CI that the release build doesn't contain the command name — `strings` check or a compile-time `cfg` test).
- [ ] xtask `desktop-sidecar` subcommand; wire into `tauri.conf.json` build commands and CI's `desktop-build` job.
- [ ] Add a 1 GB fixture **generator** (not a committed file): `cargo xtask fixtures large-pdf --pages 20000 --out target/fixtures/1gb.pdf` producing a valid PDF of ≥ 1 GiB (image-heavy pages) using the `lamina` builder; documented in `fixtures/README.md`.
- [ ] Record measurements in the PR description and in ADR 0008 (amend the "process model" section with numbers; status stays Accepted).

## Acceptance criteria

- [ ] Opening a PDF via `open_dialog` returns page count and page sizes computed **in the engine process** (verified: main process does not link `lamina-render`/PDFium — `cargo tree -p lamina-desktop -e normal` shows neither).
- [ ] Killing the engine (`kill -9` / Task Manager / `debug_kill_engine`) while a document is open results in a new engine PID, the document re-opened, and an `engine://restarted` event within 2 s; a fourth crash within 60 s marks the engine failed and the UI receives an error event instead of a restart loop.
- [ ] Malformed frames (oversized length, wrong version, truncated payload) are rejected with a `ProtocolError` and never panic (unit + fuzz tests below).
- [ ] The 1 GB generated fixture opens: `Opened` + `PageInfo` within **1.0 s** on the reference machine (leaving headroom for D2d's 1.5 s first-page target); engine RSS stays below 300 MB during open.
- [ ] No `unwrap`/`expect`/`panic!` outside tests; every `unsafe` block has `// SAFETY:`.

## Tests

- Unit tests (`crates/lamina-desktop-engine/src/protocol.rs`): round-trip every message; reject oversized frames; reject unknown version; one assertion per test, descriptive names.
- A `cargo fuzz` target `engine_protocol_decode` in the repo's fuzz layout (ADR 0018) feeding arbitrary bytes to `read_frame`; add to the full lane's 2-minute fuzz run.
- Integration test (`crates/lamina-desktop-engine/tests/engine_process.rs`): spawn the built engine binary, open `fixtures/` documents via an in-test `ByteSource` server, render a tile, compare its checksum with an in-process render; kill the child and assert the supervisor (exercised through a non-Tauri `Supervisor` core type) restarts it.
- Supervisor unit tests with a fake engine (a tiny test binary that crashes on command) for backoff and the 3-in-60 s limit.
- `MmapSource` tests: ranges at file boundaries, zero-length reads, reads past EOF return an error.

## Docs to update

- ADR 0008: measurements and any change to "how bytes reach the engine".
- `docs/plans/desktop-app.md` §9 D2: tick "Implement the engine child process with restart on crash" and the mmap half of "Implement the chosen tile transport and mmap-backed `ByteSource`".
- `CLAUDE.md` (new crate in the layout tree; `cargo xtask desktop-sidecar`, `cargo xtask fixtures large-pdf`); `fixtures/README.md`.

## Risks and open questions

- **mmap + external modification**: if another program truncates the file while mapped, reading the mapping raises `SIGBUS` (Unix) / `EXCEPTION_IN_PAGE_ERROR` (Windows) **in the main process**. Mitigations: copy ranges out promptly (never hand the mapping to the engine), detect size/mtime change via D3b's watcher and remap, and on Unix consider positioned reads instead of mmap for files on removable/network volumes. Decide in this PR and record in ADR 0008.
- **Range-over-pipe latency** may hurt parsing of files with scattered xref streams; measure; the fallback is passing a read-only file descriptor/handle to the engine (Unix `SCM_RIGHTS`/inherited fd, Windows `DuplicateHandle`) so it can mmap itself — compatible with sandboxing (the engine gets a handle, not a path).
- **PDFium is not thread-safe**: renders must be serialised per PDFium instance; the "worker pool" may be one render thread plus parsing threads. Check `pdfium-render`'s thread-safety feature flags.
- **Sidecar naming/signing**: `externalBin` requires the `-<target-triple>` suffix; universal macOS builds need a universal engine binary (`lipo`) — note for D6c.
- Engine-per-window memory cost (PDFium + core) vs a pool: record numbers; the plan's open question 3 is answered in ADR 0008 using them.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
