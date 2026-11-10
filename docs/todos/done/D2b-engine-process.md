# D2b — Engine child process: protocol, file byte source, restart on crash

| Field | Value |
| --- | --- |
| ID | `D2b` |
| Status | `done` |
| Plan | [Desktop app §6 D2](../../plans/desktop-app.md#d2-shell-and-native-core) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [C4c](./C4c-remote-files-encryption-privacy.md), [C4d](./C4d-deploy-previews-and-production.md), [D2a](./D2a-adr-0008-and-desktop-scaffold.md) |
| Dependency notes | [D2a](../done/D2a-adr-0008-and-desktop-scaffold.md); web plan **C2** (`crates/laminapdf-render` with the PDFium adapter, [web-app §C2](../../plans/web-app.md#c2-render-backend-and-adrs)) — satisfied transitively by C4 via D2a |
| ADRs | Implements ADR 0008 §process model (D2a). Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (`Limits` apply in the engine), [0001](../../decisions/0001-core-language-and-licence.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md), ADR 0004 (`RenderBackend`, web C2) |
| Branch | `feat/d2b-engine-process` |
| PR title | `feat(desktop): run parsing and rendering in a restartable engine process` |
| Size | L (600–1,500 lines) |
| PR | [#394](https://github.com/laudebugs/laminapdf/pull/394) |

## Goal

Move every byte of untrusted PDF processing out of the Tauri main process. This PR adds the engine binary, the framed binary protocol between main and engine (in its own dependency-light crate so the main process never links the render stack), a positioned-read `ByteSource` owned by the main process that serves range reads to the engine (no mmap in the main process — see risks), an engine supervisor that spawns one engine per window and restarts it after a crash, and the Rust-side API the tile transport (D2d) will call. A hostile PDF that crashes PDFium or the lamina parser must take down only the engine, never the app — the same model as Chrome's out-of-process PDF viewer ([report](../../research/architecture/app-and-package-architecture.md#tauri-2-is-the-desktop-shell-with-electron-held-in-reserve-for-linux)).

## Context

- ADR 0008 (written in D2a): process model, engine binary location (`crates/laminapdf-desktop-engine` as a sidecar), the separate protocol crate `crates/laminapdf-desktop-protocol`, ownership of file handles.
- [Desktop plan §4 req. 3](../../plans/desktop-app.md#functional): "Parse and render in a child process; a crash shows 'document process restarted' and reloads the document." [§5](../../plans/desktop-app.md#5-design-summary): main process does file I/O only (mmap, watching); tiles 256–512 px.
- [Architecture §7 Round D to-dos](../../plans/architecture.md#round-d--desktop-reader): main process "passes bytes and tiles across the boundary".
- [Report: seams](../../research/architecture/app-and-package-architecture.md#the-reference-architecture-is-one-engine-with-three-shells): `ByteSource` (range reads; mmap on desktop) and the handle-based façade with owned types and byte buffers.
- The `laminapdf` façade crate ([library plan B5](../../plans/pdf-library.md#b5-builder-and-the-laminapdf-crate)) and `laminapdf-render` (web C2): what the engine links.
- [ADR 0003](../../decisions/0003-reader-tolerance-and-limits.md): `Limits` defaults — the engine applies them; memory caps per engine are an extra OS-level guard (D2c).
- [Rust guidelines](../../guidelines/rust.md): `unsafe` only in edge/app crates with `// SAFETY:`. The main process does **not** use `memmap2` (a mapping of a file truncated underneath raises `SIGBUS` in the main process — see risks).
- `postcard` (MIT/Apache) or `bincode` 2 (MIT), `interprocess` (MIT/Apache) if pipes beyond stdio are needed.

## Scope

**In scope**

- `crates/laminapdf-desktop-protocol/` (layer `app`, `publish = false`, lib only): protocol types and framing, shared by `src-tauri` and the engine. Dependencies limited to `serde`, `postcard`, `thiserror`, `zeroize` — **never** `laminapdf`, `laminapdf-render`, `pdfium-render` or anything that pulls them in. Messages (versioned, `PROTOCOL_VERSION: u16`):
  - Main → engine: `Hello { version, limits }`, `OpenDocument { doc: DocId, len: u64 }`, `UnlockDocument { doc, password: Zeroizing<String> }`, `RangeData { req: ReqId, offset: u64, bytes }`, `GetPageInfo { doc }`, `RenderTile { req, doc, page, scale_milli, x, y, w, h, format }`, `Cancel { req }`, `CloseDocument { doc }`, `Shutdown`.
  - Engine → main: `Ready { version, pid }`, `ReadRange { req, doc, offset, len }`, `Opened { doc, page_count, encrypted, repaired }` (for an encrypted document that needs a user password, `Opened { encrypted: true, page_count: 0 }` arrives first and nothing else works until it is unlocked), `NeedsPassword { doc }`, `Unlocked { doc, page_count }`, `PageInfo { doc, sizes: Vec<(f32, f32, u16)> }`, `Tile { req, w, h, format, bytes }`, `Error { req, kind, message }` (`kind` includes `WrongPassword`), `Heartbeat`.
  - Passwords travel only in `UnlockDocument`; the engine zeroises its copy after the handler has applied it (`Zeroizing` drop) and never logs it; `Debug` for `UnlockDocument` is hand-written to redact the field.
  - Framing: `u32` little-endian length prefix + `postcard` payload, max frame size constant (e.g. 64 MiB) rejected before allocation; tile payloads carried as raw bytes after the header (no per-byte serialisation).
- `crates/laminapdf-desktop-engine/` (layer `app`, `publish = false`, depends on `laminapdf-desktop-protocol`, `laminapdf`, `laminapdf-render`):
  - **bin** `laminapdf-desktop-engine`: reads frames from stdin, writes to stdout, logs to stderr (captured by main into `tauri-plugin-log`, never document content). The stdin reader thread **only decodes and routes frames**: `RangeData` goes straight to the waiting `RemoteByteSource` request (by `ReqId`), `Cancel` updates the cancel set, and every other request (`OpenDocument`, `UnlockDocument`, `GetPageInfo`, `RenderTile`) is handed to a worker thread. The reader thread never parses or renders, because a parse blocked on `ReadRange` waits for a `RangeData` that only the reader thread can deliver (deadlock). Implements a `ByteSource` whose reads become `ReadRange` requests answered by the main process (synchronous from the parser's view, with a small read-ahead cache of e.g. 256 KiB blocks). Opens documents through the `laminapdf` façade and renders with `laminapdf-render`'s PDFium adapter; render jobs on a worker pool (`std::thread` — this is an app binary, so threads are fine) with cancellation by `ReqId`.
  - PDFium dynamic library located via an argument from main (`--pdfium <path>`), path from the app's resource dir.
- `apps/desktop/src-tauri/src/engine/`:
  - `supervisor.rs`: `EngineSupervisor` (Tauri managed state) spawning the sidecar with `std::process::Command` (path resolved next to the main executable as Tauri places `externalBin` sidecars; not through `tauri-plugin-shell`, which stays unregistered), piped stdio, one engine per window label; reader thread demultiplexing responses to `oneshot`/channel waiters; heartbeat watchdog; **restart policy**: on unexpected exit or missed heartbeats (e.g. 3 × 2 s), respawn with exponential backoff, at most 3 restarts in 60 s, then mark the window's engine failed.
  - `bytesource.rs`: `FileSource` — opens the file read-only once and serves `ReadRange` with positioned reads (`std::os::unix::fs::FileExt::read_at` / `std::os::windows::fs::FileExt::seek_read`) into a buffer, no `unsafe`. On Windows the handle is opened with `OpenOptionsExt::share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)` so an open document never blocks other apps from saving, renaming or deleting it (D3b relies on this). A short read (file truncated underneath) becomes an `Error { kind: Io }` to the engine, never a crash. No `memmap2` in the main process.
  - `documents.rs`: `DocumentRegistry` mapping `DocId` → (granted path, `FileSource`, owning window, last view state, `Option<SessionPassword>`). Paths enter the registry only from trusted sources (dialog opened from Rust, argv/Opened in D3a, recent list, session restore in D4a) — the webview never sends a path.
  - `secret.rs`: `SessionPassword` — the password of an unlocked encrypted document, held by the main process **for as long as that document is open** so an engine restart can unlock it again without asking the user. Stored in a `Zeroizing<Vec<u8>>` whose buffer is locked into RAM (`mlock` on Unix, `VirtualLock` on Windows, via a small audited crate such as `memsec` or `region` — licence check — or a 20-line `unsafe` wrapper with `// SAFETY:`); locking is best effort (failure logged once at `warn`, never fatal); zeroised and unlocked on `CloseDocument`, window close, app exit and a failed unlock; never serialised, logged, persisted, put in session files (D4a) or sent to the webview; `Debug` redacts it.
  - On engine restart: re-send `Hello`, then for every document of that window `OpenDocument` and, if it holds a `SessionPassword`, `UnlockDocument` with it; emit a Tauri event `engine://restarted` `{ window, docs }` to that window only (`emit_to`), so the UI shows "document process restarted" and re-requests visible tiles. The user is **not** re-prompted. If the stored password is rejected (should not happen; the file is unchanged), the password is discarded and the document is reported as locked, so the UI shows the normal password prompt.
- Commands (each added to the app manifest and granted in `capabilities/main.json`): `open_dialog` (Rust-side `DialogExt::file().add_filter("PDF", &["pdf"]).blocking_pick_file()` on a background thread → registers the doc → returns `DocId` + metadata), `page_info`, `close_document`. (`unlock_document`, which the UI calls with the user's password and which stores the `SessionPassword` on success, is added with the password prompt in [D2d](../ready/D2d-tile-transport-and-first-document.md); this PR implements the protocol messages, `SessionPassword` and the restart re-unlock, exercised by tests.) Test-only command `debug_kill_engine` compiled only with the Cargo feature `e2e` (used by D2e).
- An xtask step (`cargo xtask desktop-sidecar`) that builds the engine for the host/target triple, copies it to `src-tauri/binaries/laminapdf-desktop-engine-<target-triple>[.exe]` (Tauri's required naming) and fetches the PDFium shared library (per-OS prebuilt from a pinned, hash-checked release of bblanchon/pdfium-binaries — same pin as web C2's `@laminapdf/render-pdfium`) into `src-tauri/resources/pdfium/` (git-ignored).
- **Keeping plain `cargo` builds working.** `tauri-build`'s `build.rs` copies `bundle.externalBin` and `bundle.resources` at compile time and fails if they are missing, and `laminapdf-desktop` is part of `cargo xtask check rust` (clippy + tests on a fresh clone, no sidecar built). So the base `tauri.conf.json` has **no** `externalBin`/`resources`; they live in `src-tauri/tauri.bundle.conf.json`, merged only by the Tauri CLI (`tauri dev --config src-tauri/tauri.bundle.conf.json`, `tauri build --config …`, wired into the `desktop:dev`/`desktop:build` scripts, whose `beforeDevCommand`/`beforeBuildCommand` run `cargo xtask desktop-sidecar` first). Verify in this PR that the CLI hands the merged config to `build.rs`; if it does not, the fallback is that `src-tauri/build.rs` writes zero-byte placeholders at the expected paths when the real files are absent and the `desktop-build` job asserts the bundled sidecar and PDFium are non-empty before bundling. Either way, `cargo xtask check rust` stays sidecar-free, and the CI `desktop-build` job (its own job, not the fast lane) runs `desktop-sidecar` then `tauri build`.
- Measurements recorded in the PR: engine RSS idle and with a 1,000-page document; `ReadRange` throughput on the 1 GB fixture; time from `OpenDocument` to `Opened` and first `PageInfo` for the 1 GB fixture.

**Out of scope** (and which todo covers it)

- OS-level sandboxing of the engine: [D2c](../ready/D2c-engine-sandbox.md) (this PR runs it unsandboxed but already without any file paths).
- Delivering tiles to the webview and the viewer integration: [D2d](../ready/D2d-tile-transport-and-first-document.md).
- WebdriverIO e2e: [D2e](../ready/D2e-desktop-e2e-webdriverio.md).
- File watching (the registry exposes a "reload" entry point used by [D3b](../ready/D3b-recent-files-and-file-watching.md)).
- Crash reports for engine crashes: [D7a](../ready/D7a-crash-reporting.md) (this PR only logs exit status).
- Pooling engines across windows (ADR 0008 open question 3): revisit after measurements.

## Tasks

- [x] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2` and `rust-best-practices`.
- [x] Create `crates/laminapdf-desktop-protocol` (lib only) and `crates/laminapdf-desktop-engine` (bin, plus a small lib target for its testable internals) — both layer `app`, `publish = false`, licence `MIT OR Apache-2.0`; add both to `[workspace.dependencies]` (`crates/*` is already a workspace member glob). `src-tauri` depends on `laminapdf-desktop-protocol` only; it never depends on `laminapdf-desktop-engine`.
- [x] Protocol crate (`src/lib.rs`, `src/frame.rs`, `src/msg.rs`) with `Frame` encode/decode (`fn write_frame<W: Write>(w, msg: &Msg, payload: &[u8]) -> Result<(), ProtocolError>`; `fn read_frame<R: Read>(r, max: u32) -> Result<(Msg, Vec<u8>), ProtocolError>`), version check, `ProtocolError` via `thiserror`.
- [x] Engine main loop: single reader thread that only decodes and routes frames (`RangeData` → the waiting `RemoteByteSource`; everything else → workers), job queue with priorities (visible tiles > prefetch) served by worker threads that run open/unlock/page-info/render, cancel set, writer serialised through one `Mutex<StdoutLock>` or a channel to a writer thread. Test: open a document whose parse needs many `ReadRange` round trips while a render is queued — no deadlock.
- [x] `RemoteByteSource` in the engine implementing the façade's `ByteSource` trait (blocking request/response with a timeout; `Error::Io` on timeout so the parser fails cleanly).
- [x] `SessionPassword` (`secret.rs`) and the re-unlock step in the restart path.
- [x] Supervisor, `FileSource`, `DocumentRegistry`, commands, event emission as above; all state behind `Mutex`/`RwLock` with no lock held across IPC waits.
- [x] `debug_kill_engine` behind `#[cfg(feature = "e2e")]`, with the feature off in release builds (assert in CI that the release build doesn't contain the command name — `strings` check or a compile-time `cfg` test).
- [x] xtask `desktop-sidecar` subcommand (engine build + PDFium fetch); `tauri.bundle.conf.json` with `externalBin`/`resources`; wire into the `desktop:dev`/`desktop:build` scripts and CI's `desktop-build` job. Check on a fresh clone (no `src-tauri/binaries/`, no PDFium) that `cargo xtask check rust` passes.
- [x] Add a 1 GB fixture **generator** (not a committed file): `cargo xtask fixtures large-pdf --pages 20000 --out target/fixtures/1gb.pdf` producing a valid PDF of ≥ 1 GiB (image-heavy pages) using the `laminapdf` builder; documented in `fixtures/README.md`.
- [x] Record measurements in the PR description and in ADR 0008 (amend the "process model" section with numbers; status stays Accepted).

## Acceptance criteria

- [x] Opening a PDF via `open_dialog` returns page count and page sizes computed **in the engine process** (verified: main process does not link `laminapdf-render`/PDFium — `cargo tree -p laminapdf-desktop -e normal` shows none of `laminapdf-desktop-engine`, `laminapdf-render`, `pdfium-render`, `laminapdf`; run as a CI step that matches **exact package names**, not substrings — `laminapdf-desktop` and `laminapdf-desktop-protocol` are expected — e.g. `cargo tree -p laminapdf-desktop -e normal --prefix none --format '{p}' | awk '{print $1}' | sort -u | grep -Fx -e lamina -e laminapdf-render -e laminapdf-desktop-engine -e pdfium-render` failing when it prints anything).
- [x] On a fresh clone without the sidecar or PDFium, `cargo xtask check rust` (clippy + tests including `laminapdf-desktop`) passes.
- [x] Killing the engine (`kill -9` / Task Manager / `debug_kill_engine`) while a document is open results in a new engine PID, the document re-opened, and an `engine://restarted` event within 2 s; a fourth crash within 60 s marks the engine failed and the UI receives an error event instead of a restart loop.
- [x] Killing the engine while an **encrypted** document is open (unlocked with its password) re-opens and re-unlocks it in the new engine without a prompt; after `CloseDocument` the registry holds no password (unit test on the registry; integration test with an encrypted fixture).
- [x] Malformed frames (oversized length, wrong version, truncated payload) are rejected with a `ProtocolError` and never panic (unit + fuzz tests below).
- [x] The 1 GB generated fixture opens: `Opened` + `PageInfo` within **1.0 s** on the reference machine (leaving headroom for D2d's 1.5 s first-page target); engine RSS stays below 300 MB during open.
- [x] No `unwrap`/`expect`/`panic!` outside tests; every `unsafe` block has `// SAFETY:`.

## As built

Where the PR differs from the tasks above, and why:

- **Every request carries a `ReqId`**, and its reply repeats it, so one table matches every reply (the todo gave ids only to ranges and tiles). An encrypted document's `Opened` says `needs_password` itself; `NeedsPassword` answers any other request on a locked document. `RangeFailed` (main → engine) carries a failed read, and `RenderTile` a `priority`. `Hello`'s limits mirror the parser's and renderer's, each `None` for the engine's default, so the main process knows none of them.
- **The supervisor's core lives in the protocol crate** (`supervisor`, `std` only), with an in-process fake engine behind its `test-support` feature, instead of `src-tauri/src/engine/supervisor.rs`. The engine's integration test drives the real binary through it without Tauri, and the restart-policy tests use the fake rather than a second test binary. `src-tauri/src/engine/mod.rs` (`Engines`) adds the Tauri side: one engine per window, reopening and events.
- **One worker thread, not a pool:** PDFium serialises every call behind one lock and the façade's `Document` stays on one thread, so a pool would add nothing until a second renderer exists. Jobs still queue by priority (documents, visible tiles, prefetched tiles), and a cancel reaches both queued and running jobs.
- **The fake engine "crashes on command"** through its process handle, not a separate test binary.
- **No "last view state" in the registry yet:** nothing records one until the viewer runs (D2d) and session restore (D4a) reads it; the registry keeps the path for D3b.
- **`Engines::unlock` has no command yet:** D2d's `unlock_document` calls it with the user's password, as the todo says; the restart path and the tests use it now.
- **No core-dump switch:** `prctl(PR_SET_DUMPABLE, 0)` also blocks the user's debugger and crash reporter; ADR 0008 §2 records the trade-off.
- **Merged config:** `tauri build --config src-tauri/tauri.bundle.conf.json` hands the merged config to `build.rs` (Tauri copied `resources/pdfium/` into `target/debug/pdfium`), so no placeholder files are needed.
- **The fixture generator** is the engine crate's `large_pdf` example, which `cargo xtask fixtures large-pdf` runs, so xtask doesn't link the core. 20,000 pages make 1,082 MiB.
- **Measurements** (M1 Pro, macOS 27, release build; ADR 0008 §2): engine idle 11 MiB; the 1 GB fixture's `Opened` and `PageInfo` in 478–494 ms with 115–126 MiB peak; 4,335 reads of 256 KiB (the whole file, since each page dictionary sits beside its image) at about 2.2 GiB/s; a 1,000-page document in 74 ms, 41 MiB after opening, 106 MiB peak over 1,000 tiles.
- **Added:** `build.rs` deletes the e2e command's permission file in a non-e2e build, since tauri-build never removes one and a stale file put `debug_kill_engine` in the next release build. A main-process read is capped at 16 MiB, so a compromised engine can't make the main process allocate gigabytes, and a window's engine reads only that window's documents. `cargo xtask render-golden` also runs the engine's PDFium test, a tile compared byte for byte with an in-process render.

## Post-merge checks

None.

## Tests

- Unit tests (`crates/laminapdf-desktop-protocol`): round-trip every message (including `UnlockDocument`/`Unlocked`/`NeedsPassword`); reject oversized frames; reject unknown version; `format!("{:?}", UnlockDocument { .. })` does not contain the password; one assertion per test, descriptive names.
- `SessionPassword` unit tests: dropped/cleared values are zeroised (inspect through a test-only hook), `Debug` redacts, the registry drops it on `CloseDocument` and on a failed unlock.
- A `cargo fuzz` target `engine_protocol_decode` in the repo's fuzz layout (ADR 0018) feeding arbitrary bytes to `read_frame`; add to the full lane's 2-minute fuzz run.
- Integration test (`crates/laminapdf-desktop-engine/tests/engine_process.rs`): spawn the built engine binary, open `fixtures/` documents via an in-test `ByteSource` server, render a tile, compare its checksum with an in-process render; kill the child and assert the supervisor (exercised through a non-Tauri `Supervisor` core type) restarts it.
- Supervisor unit tests with a fake engine (a tiny test binary that crashes on command) for backoff and the 3-in-60 s limit.
- `FileSource` tests: ranges at file boundaries, zero-length reads, reads past EOF return an error, a file truncated after open returns an error (no panic, no signal).
- Engine routing test: a fake main process that answers `ReadRange` slowly while `OpenDocument` and `RenderTile` are in flight — the engine completes both (guards the reader-thread deadlock).

## Docs to update

- ADR 0008: measurements, "how bytes reach the engine", and the decision **positioned reads, not mmap, in the main process** (the plan says "mmap"; ADR 0008 records why this differs).
- `docs/plans/desktop-app.md` §9 D2: tick "Implement the engine child process with restart on crash" and the `ByteSource` half of "Implement the chosen tile transport and mmap-backed `ByteSource`" (note: positioned reads, see ADR 0008).
- `CLAUDE.md` (new crates `laminapdf-desktop-protocol` and `laminapdf-desktop-engine` in the layout tree; `cargo xtask desktop-sidecar`, `cargo xtask fixtures large-pdf`); `fixtures/README.md`.

## Risks and open questions

- **Decision: no mmap in the main process.** If another program truncates a mapped file, touching the mapping raises `SIGBUS` (Unix) / `EXCEPTION_IN_PAGE_ERROR` (Windows) **in the main process**, taking down every window; a Windows mapping also stops other apps from replacing the file. Positioned reads turn truncation into an ordinary I/O error and cost one copy per range, which the engine pipe already needs. If the measured `ReadRange` throughput misses the 1 GB targets, revisit by mmapping **in the engine** (via a handed-over handle, see next point), where a `SIGBUS` only kills the restartable engine. Recorded in ADR 0008.
- **Range-over-pipe latency** may hurt parsing of files with scattered xref streams; measure; the fallback is passing a read-only file descriptor/handle to the engine (Unix `SCM_RIGHTS`/inherited fd, Windows `DuplicateHandle`) so it can mmap itself — compatible with sandboxing (the engine gets a handle, not a path).
- **Holding passwords for the session**: chosen over re-prompting after a crash (a crash is not the user's fault and a prompt per crash per document is hostile). Locked memory keeps the password out of swap but not out of a core dump or a debugger attached to the main process; disable core dumps for the main process where cheap (`prctl(PR_SET_DUMPABLE, 0)` is too broad — record what is done) and record the trade-off in ADR 0008.
- **PDFium is not thread-safe**: renders must be serialised per PDFium instance; the "worker pool" may be one render thread plus parsing threads. Check `pdfium-render`'s thread-safety feature flags.
- **Sidecar naming/signing**: `externalBin` requires the `-<target-triple>` suffix; universal macOS builds need a universal engine binary (`lipo`) — note for D6c.
- Engine-per-window memory cost (PDFium + core) vs a pool: record numbers; the plan's open question 3 is answered in ADR 0008 using them.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
