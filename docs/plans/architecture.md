# Plan: lamina architecture

- **Status:** Draft, for review
- **Date:** 2026-10-28
- **Research:** [docs/research/architecture/app-and-package-architecture.md](../research/architecture/app-and-package-architecture.md) (and its [notes](../research/architecture/notes/)), plus PDF research [05 §8](../research/pdf/05-creating-pdfs-from-code.md), [06](../research/pdf/06-feature-inventory.md) and [07](../research/pdf/07-spike-rust-and-typescript.md)
- **Scope:** the shared architecture behind every lamina product. The product plans (library, web app, desktop app) build on this one.

## 1. Goal

One architecture that lets lamina ship, in order:

1. **Packages:** a Rust crate family (`laminapdf`) and a TypeScript package (`@laminapdf/sdk`) that generate PDFs, and later read and edit them.
2. **A web app** on Cloudflare: a reader first, growing into a full editor.
3. **A desktop app** (macOS, Windows, Linux) with the same features, with downloads and updates served from Cloudflare.

All three share one engine, so each of the ~180 operations in the [feature inventory](../research/pdf/06-feature-inventory.md) is built once.

## 2. The architecture in one picture

```
   apps/web (Cloudflare)        apps/desktop (Tauri 2)          packages
   React chrome +               same UI packages +              @laminapdf/sdk (npm)
   framework-free viewer        native menus, updater           laminapdf (crates.io)
        │ typed RPC                  │ Channel / lamina://            │ direct calls
        ▼ (Web Worker)               ▼ (native, sandboxed process)    ▼ (WASM or native)
   ┌──────────────────────── lamina core (Rust, bytes in / bytes out) ───────────────────────┐
   │ crates: laminapdf-syntax · -cos · -model · -ops · -save · -build · -text (façade: laminapdf)   │
   └───────────────────────────────┬─────────────────────────────────────────────────────────┘
                                   ▼
        laminapdf-render: RenderBackend trait → PDFium (default) · hayro (CI oracle → future default)

   Hosted service tier (Cloudflare; login and cloud features opt-in): Workers · Durable Objects (sync) · R2 · D1 · Workflows · Containers (OCR, Office)
```

### Principles (each becomes part of an ADR)

1. **Bytes in, bytes out.** The core never touches the file system, network or clocks, and never spawns threads (shells may run it on several threads or workers). Input arrives through a `ByteSource` trait, output leaves through a `WriteSink` trait. This was proven in the spike, and it is what makes one crate run in a browser worker, a Cloudflare Worker, Tauri and, later, mobile.
2. **Every change is a typed `Op`.** Undo, dirty tracking, collaboration sync and the public API all consume the same op stream.
3. **The save planner owns correctness.** `SaveConstraint` (Free / AppendOnly / RequiresRewrite, or Conflict when they overlap; [ADR 0007](../decisions/0007-save-planner.md)) makes signature-breaking or redaction-leaking saves impossible by default. Fixed order: redact → full rewrite → sign → append-only saves.
4. **Rendering is swappable.** PDFium is the default renderer today. hayro (pure Rust) is diffed against it in CI and becomes the default once it reaches parity on our corpus. MuPDF is excluded (AGPL).
5. **The UI framework stays off the hot path.** Page virtualisation, the canvas pool, the text layer and the annotation layer are plain TypeScript, so the framework choice is reversible and the viewer can be embedded by others.
6. **Privacy first.** Documents are processed on the device by default. lamina is also a hosted service with user login and cloud features (§8 Q2), but signing in and every server feature stay opt-in, and the app works fully without an account.

## 3. Decisions to record (ADRs)

ADRs go in `docs/decisions/`, one file per decision (`NNNN-title.md`). Each round writes the ADRs assigned to it below (see §5). The numbering follows the report.

| # | ADR | When | Status |
| --- | --- | --- | --- |
| [0001](../decisions/0001-core-language-and-licence.md) | Core language (Rust), licence (MIT OR Apache-2.0), bytes-in/bytes-out | Round A | Accepted |
| [0002](../decisions/0002-binding-strategy.md) | Binding strategy: wasm-bindgen + tsify; UniFFI later; napi-rs and Component Model deferred | Round A | Accepted |
| [0015](../decisions/0015-monorepo-layout-and-tooling.md) | Monorepo layout and tooling (Cargo + pnpm + xtask) | Round A | Accepted |
| [0019](../decisions/0019-supply-chain-and-licence-hygiene.md) | Supply-chain and licence hygiene (cargo-deny, npm licence checks, SBOM) | Round A | Accepted |
| [0003](../decisions/0003-reader-tolerance-and-limits.md) | Reader tolerance policy and `Limits` defaults | Round B | Accepted |
| [0005](../decisions/0005-document-model.md) | Document model: lazy COS overlay, typed views, stable `LaminaPdfId`s | Round B | Accepted (generation strategy Proposed) |
| [0006](../decisions/0006-op-log-and-undo.md) | Op log, transactions, undo semantics | Round B | Accepted |
| [0007](../decisions/0007-save-planner.md) | Save planner and `SaveConstraint` | Round B | Accepted |
| [0016](../decisions/0016-package-design.md) | Package design: ESM-only, export conditions, split WASM modules, size budgets | Round B | Accepted |
| [0017](../decisions/0017-versioning-and-release.md) | Versioning and release: lockstep core, Changesets + Cargo version sync, release lane; OIDC trusted publishing, attestations (P1) | Round B | Accepted (versioning; publishing part Proposed, P1) |
| [0018](../decisions/0018-test-strategy.md) | Test strategy: goldens, differential corpus, fuzzing, veraPDF | Round B | Accepted |
| [0023](../decisions/0023-content-stream-reading.md) | Content-stream reading: `laminapdf-content` (tokenizer, visitor), tolerance rules, operator and XObject-depth limits | Round B (B9a) | Accepted |
| [0004](../decisions/0004-render-backend.md) | `RenderBackend`; PDFium default; hayro oracle; MuPDF prohibited | Round C | Accepted |
| [0009](../decisions/0009-web-client-architecture.md) | Web client: React chrome + framework-free viewer core; typed worker RPC | Round C | Accepted |
| [0010](../decisions/0010-platform-adapter.md) | Platform adapter and capability flags | Round C | Accepted |
| [0011](../decisions/0011-cloudflare-topology-and-privacy.md) | Cloudflare topology, privacy/retention rules and the hosted service (accounts, cloud storage, sync) | Round C | Accepted (login stack by amendment after H1) |
| [0008](../decisions/0008-desktop-shell.md) | Desktop shell: Tauri 2, process model, tile transport, Linux fallback | Round D (after IPC spike) | Accepted |
| [0014](../decisions/0014-desktop-distribution.md) | Desktop distribution: update Worker + R2, channels, signing (the owner as publisher), key custody | Round D | Accepted |
| [0024](../decisions/0024-app-shell-ux.md) | App shell UX: inverted-L layout, modes, one command registry, selection surfaces, page appearance modes, phone layout, token layers ([UI/UX plan](ui-ux.md)) | Round U (U1a) | Accepted |
| 0025 | Mobile shell: Tauri 2 mobile vs UniFFI with native UI (after the `mobile-shell` spike) | Round U (U3b) | Defer |
| 0012 | Collaboration: one Durable Object per document, last-writer-wins records | Later (with collaboration) | Defer |
| 0013 | Heavy jobs: Workflows + Containers; OCR/conversion engines and licences | Later (with OCR) | Defer |
| [0022](../decisions/0022-content-stream-editing.md) | Content-stream editing: an IR with object identity and a span-preserving encoder (after the `content-edit` spike) | [Full editor](full-editor.md#e7-editing-existing-content) E7 (E7a) | Accepted |
| [0026](../decisions/0026-error-and-diagnostic-contract.md) | Error and diagnostic contract | [Shareable libraries](shareable-libraries.md#6-decisions-to-record-adrs) L1 | Accepted |
| [0027](../decisions/0027-coordinates-units-and-page-indexing.md) | Coordinates, units and page indexing in public APIs | Shareable libraries L1 | Accepted |
| [0028](../decisions/0028-facade-rules-jobs-and-cancellation.md) | Façade binding rules, jobs, cancellation and progress | Shareable libraries L1 | Accepted |
| [0029](../decisions/0029-mobile-bindings-and-packaging.md) | Mobile bindings and packaging (UniFFI pin, SPM, AARs) | Shareable libraries L6 | Accepted (L6a spike) |
| [0030](../decisions/0030-extraction-output-and-model-plugins.md) | Extraction output (`laminapdf.extract/1`) and model plug-ins | Shareable libraries L3 | Accepted |
| 0031 | CLI and MCP contract | Shareable libraries L4 | Propose |
| [0032](../decisions/0032-flowing-layout-and-document-inputs.md) | Flowing layout and document inputs: our own engine over the builder's `Surface` (not Typst), tagged by default, Markdown and `laminapdf.document/1` | Trust and proof T4a | Accepted |

## 4. Spikes to close open questions

The research couldn't answer these. Each is a small, time-boxed spike under `spikes/` with a written result.

| Spike | Question | Blocks |
| --- | --- | --- |
| `desktop-ipc` | Throughput of Tauri Channel vs `ipc::Response` vs a `lamina://` scheme for page tiles on macOS, Windows and Linux; do WKWebView and WebKitGTK honour COOP/COEP? | ADR 0008. **Done** ([result](../../spikes/desktop-ipc/README.md)): lossless WebP over the `lamina://` scheme moved 2.5–2.7× more 512 px tiles per second than any raw transport on Windows and Linux, 1.2× on macOS 15, and tied on an M1 Pro (raw RGBA as the fallback); only WebView2 becomes cross-origin isolated with `SharedArrayBuffer`, so the desktop app stays single-threaded |
| `pdfium-wasm` | Load PDFium-WASM in a worker next to the lamina core: bundle size, memory, render time; reload after an incremental save | ADR 0004, 0009. **Done** ([result](../../spikes/pdfium-wasm/README.md)): both engines share one worker in Chromium, Firefox and WebKit (25–60 ms to instantiate, page 1 in 15–39 ms cold); PDFium is 2.1 MB gzip; reopening PDFium after an incremental save is milliseconds, and lamina's save is the cost (it reads the whole base for `/ID`; Chromium's first whole-output save is ≈ 10× slower until its code tiers up); custom file access halves PDFium's heap and is how WebKit opens a 500 MB file |
| `krilla-fit` | Can krilla write into an existing object graph, or do we build our own builder over `laminapdf-cos`? | Library plan, Round B. **Done** ([result](../../spikes/krilla-fit/README.md)): no, krilla only emits finished bytes; recommends an own builder over `laminapdf-cos` reusing `pdf-writer`/`subsetter`/`skrifa`/HarfRust (ADR 0005 §6) |
| `lamina-ffi` | Does the façade bind to Swift and Kotlin through UniFFI 0.32 without changes? Cancellation, panics, sizes, packaging ([L6a](../todos/done/L6a-lamina-ffi-spike.md)) | ADR 0029. **Done** ([README](../../spikes/lamina-ffi/README.md)): binds without façade changes; Swift (macOS, iOS simulator) and Kotlin pass with byte-equal output; Swift async cancellation needs a Rust-side token (the generated Swift never drops the future); every Android `.so` 16 KB-aligned; about 2.0 MB on iOS and 2.4 MB on Android `arm64-v8a` with `text`; cargo-swift 0.11.1 can't build UniFFI 0.32; UniFFI is MPL-2.0 (allowed as an exception in `deny.toml`, 2026-10-30) |
| `hayro-parity` | hayro vs PDFium pass rate on the pdf.js and PDFBox corpora: the baseline for switching the default | ADR 0004. **Done** ([result](../../spikes/hayro-parity/README.md)): 81.8 % of 2,196 pages pass and 90.8 % pass or differ slightly at 96 DPI; where hayro is clearly wrong, it is mostly fonts, annotations without appearance streams and transparency groups (CalRGB/CalGray differences still need a reference rendering to judge). Recommends PDFium as the default everywhere and hayro as the CI oracle, with switch criteria for ADR 0004 |
| `flow-layout` | Our own flowing-layout engine over `Surface`, Typst embedded, or Typst's layout drawn through `Surface`: size, speed, tagging, shaping, the object graph, licences ([T4a](../todos/done/T4a-adr-0032-layout-engine.md)) | ADR 0032. **Done** ([result](../../spikes/flow-layout/README.md)): Typst adds 7.4 MB gzip and 147 crates, returns finished bytes, draws missing glyphs as blank boxes without a warning, and pdf.js reads its Arabic and Devanagari back scrambled; a paginator of our own added 19 KB and wrote identical bytes natively and in WebAssembly. Recommends our own engine, with four builder gaps first (elements across pages, measuring, artifact subtypes, marks under PDF/UA-1) |
| `desktop-print` | How to print faithfully from Tauri on each OS (native print of rendered pages vs webview print), given the `hasNativePrint` capability | Round D. **Done** ([result](../../spikes/desktop-print/README.md)): print Lamina PDF's own rasters through each OS's print API (every page at its own size on Windows and Linux, SSIM ≥ 0.95); no webview print path ships |
| `mobile-shell` | Tauri 2 mobile vs UniFFI with native UI (SwiftUI, Compose): native selection-menu conflicts, Apple Pencil latency, share sheet and Files integration, Vision / ML Kit OCR access ([U3b](../todos/ready/U3b-mobile-shell-spike.md)) | ADR 0025, §8 Q4 |
| `auth` | Better Auth / Auth.js on D1 vs hosted vendors, for the hosted service's user login and accounts (in scope since 2026-10-28, §8 Q2); run before the first round that needs accounts ([H1](../todos/ongoing/H1-auth-spike.md)) | ADR 0011. **Result, waiting for the owner's decision** ([README](../../spikes/auth/README.md)): Better Auth 1.7 on a Worker with D1 passed all 19 checks (magic link, passkeys, Strict `/api` cookies, a loopback + PKCE desktop sign-in with an approval page, export, deletion); Auth.js can't store passkeys on its D1 adapter and now points new projects to Better Auth; Clerk's non-HttpOnly session cookie and CSP needs break ADR 0011. Every library fails OAuth with a Strict state cookie, so that cookie alone is Lax. Recommends Better Auth; the ADR 0011 amendment waits for the owner |

## 5. Rounds

Each round is split into todos, one pull request each, every one in its own worktree and reviewed before merge (see [Todos](#todos) below).

- **Round A — Foundations** (done: [A1](../todos/done/A1-foundation-adrs.md), [A2](../todos/done/A2-monorepo-skeleton.md), [A3](../todos/done/A3-port-spike-core.md)). ADRs 0001, 0002, 0015, 0019. Monorepo skeleton (`crates/`, `packages/`, `apps/`, `fixtures/`, `tools/xtask`, `deny.toml`, `rust-toolchain.toml`, pnpm workspace). CI fast lane. Port the spike's core into `crates/laminapdf-syntax` and `laminapdf-cos`, keeping its tests.
- **Round B — Core model and the first package** (todos B1a–B9c, listed in the [library plan §7](pdf-library.md#7-rounds)). ADRs 0003, 0005–0007, 0016–0018. `laminapdf-model`, `laminapdf-ops`, `laminapdf-save`, `laminapdf-build`; the `laminapdf` façade crate; `laminapdf-wasm` + `@laminapdf/sdk` with generation; the `krilla-fit` spike. CI full lane with golden tests ([X4](../todos/done/X4-full-lane-gate.md)). The first `0.1.0-alpha` is versioned and tagged but **not published**: crates and npm packages stay local until the owner decides to publish (see [Publishing (later)](#publishing-later)).
- **Round C — Web reader** (todos C1a–C7c, listed in the [web-app plan §6](web-app.md#6-rounds)). ADRs 0004, 0009–0011. `pdfium-wasm` and `hayro-parity` spikes; `@laminapdf/viewer`, `@laminapdf/ui`, `apps/web` on Workers static assets; Playwright tests; preview deploys per PR.
- **Round D — Desktop reader** (todos D1a–D7c, listed in the [desktop-app plan §6](desktop-app.md#6-rounds)). `desktop-ipc` spike; ADRs 0008, 0014; `apps/desktop` (Tauri 2); `apps/updates` Worker + R2; signing and notarisation in CI.
- **Round U — UI/UX** (todos U1a–U5c, listed in the [UI/UX plan §6](ui-ux.md#6-rounds)). ADR 0024; `mobile-shell` spike (ADR 0025). Runs alongside C, D and E: command registry and palette, inverted-L shell and modes, selection surfaces and copy modes, reading-order corpus, page appearance modes, phone layout, network indicator, reader mode, pricing and promises page, benchmark, opt-in telemetry. Its v0.1 parts gate C7a and C7c.
- **Later rounds** (todos E1a–E18h and H1–H4, listed in the [full-editor plan §7](full-editor.md#7-phases-and-rounds)). Editing features by phase ([06 §9](../research/pdf/06-feature-inventory.md)), collaboration (ADR 0012), heavy jobs (ADR 0013). Each feature group gets its own plan and pull requests.

Product-level detail (features, milestones, acceptance criteria) lives in the product plans ([library](pdf-library.md), [web app](web-app.md), [desktop app](desktop-app.md), [full-featured editor](full-editor.md)).

### Todos

Every round is broken into todos, one per pull request, in [`docs/todos/`](../todos/README.md): `ready/` → `ongoing/` → `done/`, each with its dependencies. The to-dos in §7 link to them; Round A's are in `done/`.

## 6. Risks

| Risk | Mitigation | Owner |
| --- | --- | --- |
| **Two parsers** (lamina core + PDFium) disagree, or re-rendering after content edits is slow | Annotations, fields and page operations render as overlays from lamina's model; incremental re-render of touched pages; nightly differential corpus; push hayro to parity | Core |
| **Linux WebKitGTK** canvas/compositing failures in Tauri | No GPU dependency; documented workarounds; Electron-on-Linux fallback | Desktop |
| **4 GB WASM memory cap**, since Safari lacks Memory64 | Streaming `ByteSource`, OPFS cache, tile caps; desktop handles giant files natively | Core / Web |
| **PDFium supply chain** (prebuilt WASM, single-maintainer `pdfium-render`) | Pin and reproduce builds; isolate in a worker or child process; keep the trait so it can be swapped | Render |
| **Signing identities and keys** (Windows eligibility, Apple notarisation, losing the updater key) | The owner signs personally as the publisher on all three OSes (§8 Q5). **Check for the owner:** Windows Artifact Signing's public-trust individual eligibility is US/Canada only ([desktop notes, Q6](../research/architecture/notes/desktop-shell.md#q6-code-signing--notarization-linux-packaging-ci)); outside those, use an OV certificate held in a cloud HSM. Secret-manager or HSM custody; key-rotation runbook (ADR 0014) | Release |
| **Scope**: ~180 operations across three products | Capability matrix as the roadmap; packages ship value before the editor | Maintainers |

## 7. To-dos

### Round A — Foundations
Todos: [A1](../todos/done/A1-foundation-adrs.md) (ADRs), [A2](../todos/done/A2-monorepo-skeleton.md) (skeleton, xtask, CI), [A3](../todos/done/A3-port-spike-core.md) (port), all done.

- [x] Write ADR 0001 (core language, licence, bytes in / bytes out)
- [x] Write ADR 0002 (binding strategy)
- [x] Write ADR 0015 (monorepo layout and tooling)
- [x] Write ADR 0019 (supply-chain and licence hygiene)
- [x] Create the root Cargo workspace (`[workspace.lints]`, `rust-toolchain.toml`, edition 2024, MSRV policy)
- [x] Create the pnpm workspace (`pnpm-workspace.yaml`, shared `tsconfig.base.json`, Biome config)
- [x] Add `tools/xtask` with `cargo xtask check`, `test`, `wasm`, `fmt` (the single task entry point, per the code-organization guide)
- [x] Add `deny.toml` (licence allow-list without AGPL/GPL; advisories) and an npm licence check
- [x] CI fast lane: fmt, clippy `-D warnings`, nextest, wasm32 build, TS typecheck/lint/test, cargo-deny (the wasm size budget moves to Round B, with the first `.wasm` artefact from `laminapdf-wasm`)
- [x] Port the spike reader/writer into `crates/laminapdf-syntax` + `crates/laminapdf-cos`, with its fixtures moved to `fixtures/`
- [x] Reserve package names ([P1](../todos/done/P1-reserve-names-and-trusted-publishing.md); deferred on 2026-10-28, done 2026-11-01); see [Publishing (later)](#publishing-later)
- [ ] Update `CLAUDE.md` (layout, commands, decided architecture)

### Round B — Core model and the first package
- [x] Write ADRs 0003, 0005, 0006, 0007, 0018 ([B1a](../todos/done/B1a-round-b-adrs.md); 0005 §6, the generation strategy, settled by `krilla-fit`)
- [x] Write ADRs 0016 ([B6a](../todos/done/B6a-lamina-wasm-and-adr-0016.md): [done](../decisions/0016-package-design.md)), 0017 ([B7b](../todos/done/B7b-versioning-and-0-1-0-alpha.md): [versioning part](../decisions/0017-versioning-and-release.md); the publishing part is [P1](../todos/done/P1-reserve-names-and-trusted-publishing.md))
- [x] `krilla-fit` spike ([B1b](../todos/done/B1b-krilla-fit-spike.md), [result](../../spikes/krilla-fit/README.md)), then decide the generation strategy (Option B, recorded in [ADR 0005 §6](../decisions/0005-document-model.md))
- [x] `laminapdf-model`: typed pages, annotations, fields, outline; stable IDs (pages, outline, Info, name trees and page labels with their IDs done; annotations and fields come later. [B2a](../todos/done/B2a-limits-owned-document.md) owned `Document` and `Limits`, [B2b](../todos/done/B2b-standard-security-decryption.md) decryption, [B2c](../todos/done/B2c-lamina-model-corpus-fuzz.md) views and IDs; annotations and fields in the editor rounds)
- [x] `laminapdf-ops`: `Op` enum, `transact()`, inverses, undo/redo ([B3a](../todos/done/B3a-overlay-and-ops.md))
- [x] `laminapdf-save`: save planner, `SaveConstraint`, streaming `WriteSink` (B3b: the crate, writers and `WriteSink`; B3c: planner, `SaveConstraint`, round-trip) ([B3b](../todos/done/B3b-lamina-save-writers.md), [B3c](../todos/done/B3c-save-planner-and-roundtrip.md))
- [x] Move the writer (`Writer`, `incremental_update`, `XrefEol`) from `laminapdf-cos` into `laminapdf-save`, keeping the determinism and golden tests ([B3b](../todos/done/B3b-lamina-save-writers.md))
- [x] `laminapdf-build`: builder API (`Document → page → surface → finish() → Document`) ([B5a](../todos/done/B5a-lamina-build-core.md), [B5b](../todos/done/B5b-builder-text-and-document-features.md))
- [x] Promote the spike's content builder and golden samples (`hello`, `imaging_demo`), until then test support in `crates/laminapdf-cos/tests/common/`, into `laminapdf-build` (API and examples) ([B5a](../todos/done/B5a-lamina-build-core.md): the builder uses `pdf_writer::Content`; the spike's builder and the two frozen goldens are `laminapdf-build`'s test support)
- [x] `laminapdf` façade crate: re-exports the public API of the core crates; this is what goes to crates.io ([B5c](../todos/done/B5c-lamina-facade-and-examples.md))
- [x] `laminapdf-text`: font embedding and subsetting for generation (see [03](../research/pdf/03-text-and-fonts.md)) (B4a done: loading, TTC, variable instances, subsetting, Type 0 with `/W` and ToUnicode; B4b done: HarfRust shaping, ICU4X bidi and line breaking, greedy layout; B4c done: the standard 14 with AFM metrics, simple-font encodings, the AGL) ([B4a](../todos/done/B4a-font-loading-and-subsetting.md), [B4b](../todos/done/B4b-shaping-bidi-line-breaking.md), [B4c](../todos/done/B4c-standard-14-and-simple-fonts.md))
- [x] `laminapdf-wasm` (wasm-bindgen + tsify) and `@laminapdf/sdk` (ESM-only, `init()`, export conditions; publint + attw) (`laminapdf-wasm` and `@laminapdf/wasm` done in [B6a](../todos/done/B6a-lamina-wasm-and-adr-0016.md), `@laminapdf/sdk` in [B6b](../todos/done/B6b-lamina-pdf-package.md); the runtime matrix is [B6c](../todos/done/B6c-runtime-matrix.md))
- [x] Fuzz targets under `fuzz/`, with a scheduled 10-minute run, and the golden, corpus-facts and differential harness in the fast lane ([B2c](../todos/done/B2c-lamina-model-corpus-fuzz.md))
- [x] Golden-file and corpus test harness in CI (full lane) ([X4](../todos/done/X4-full-lane-gate.md) full lane)
- [x] Linux CI on a self-hosted runner in a container on the Mac ([X5](../todos/done/X5-linux-ci-container.md))
- [x] Windows CI coverage: the full check on hosted Windows ([X6](../todos/done/X6-windows-ci.md))
- [x] A required check for the fast lane ([X7](../todos/done/X7-fast-lane-gate.md))
- [ ] Compiled Rust shared between CI runs, through sccache on Cloudflare R2 ([CI caching plan](ci-caching.md): [X8a](../todos/done/X8a-rust-compile-cache-trial.md), [X8b](../todos/ready/X8b-rust-compile-cache-everywhere.md))
- [x] Changesets + Cargo version sync; tag `0.1.0-alpha` locally (no registry publish yet) ([B7b](../todos/done/B7b-versioning-and-0-1-0-alpha.md): every crate and both packages at `0.1.0-alpha.0`, `cargo xtask version sync`, the release lane and `verify-install`; the `v0.1.0-alpha.0` tag is the owner's post-merge step; docs site [B7a](../todos/done/B7a-docs-site.md))

### Round C — Web reader

Todos: see the [web-app plan §6](web-app.md#6-rounds) (C1a–C7c).

- [x] `pdfium-wasm` and `hayro-parity` spikes, with written results (`hayro-parity` done: [C1a](../todos/done/C1a-hayro-parity-spike.md), [result](../../spikes/hayro-parity/README.md); `pdfium-wasm` done: [C1b](../todos/done/C1b-pdfium-wasm-spike.md), [result](../../spikes/pdfium-wasm/README.md))
- [x] Write ADRs 0004, 0009, 0010, 0011 ([C2a](../todos/done/C2a-render-adrs.md))
- [x] `laminapdf-render` with the `RenderBackend` trait and PDFium/hayro adapters ([C2b](../todos/done/C2b-lamina-render-crate.md))
- [x] `@laminapdf/render-pdfium` (ships pdfium.wasm, used by the web reader) and `@laminapdf/render-hayro` (pure-Rust renderer as its own .wasm), both optional so `@laminapdf/sdk` stays small ([C2c](../todos/done/C2c-render-packages.md))
- [ ] `@laminapdf/viewer`: worker RPC, page virtualiser, canvas pool, text layer, search
- [ ] `@laminapdf/ui` (React 19) and `apps/web` on Workers static assets; per-PR previews
- [ ] Playwright and visual regression tests; accessibility checks

### Round D — Desktop reader

Todos: see the [desktop-app plan §6](desktop-app.md#6-rounds) (D1a–D7c).

- [ ] `desktop-ipc` and `desktop-print` spikes, then ADR 0008
- [ ] `apps/desktop` (Tauri 2) reusing `@laminapdf/ui`. Process model: all parsing and rendering of untrusted files runs in a sandboxed engine process; the main process only does file I/O (mmap, file watching) and passes bytes and tiles across the boundary
- [ ] `apps/updates`: Worker + R2 update manifests with channels and staged rollout
- [ ] Signing: Apple notarisation, Windows Artifact Signing, Linux AppImage/deb; tauri-action release workflow
- [x] Write ADR 0014

### Publishing (later)

Decided 2026-10-28 by the project owner: crates and npm packages stay local (workspace and path dependencies, `pnpm pack` tarballs, git tags) until the owner decides to publish. The owner decided to publish on 2026-11-01, from the repository while it stays private.

Todos: [P1](../todos/done/P1-reserve-names-and-trusted-publishing.md) (the publish jobs), [P2](../todos/done/P2-first-registry-prerelease.md) (the first release, `0.2.0-alpha.1`) and [P3](../todos/done/P3-registry-checks.md) (registry installs, semver checks and the docs site after it).

- [x] Choose the npm scope: `@laminapdf` (2026-10-31; [P0](../todos/done/P0-npm-scope-laminapdf.md))
- [x] Rename the crates `laminapdf-*` and the façade `laminapdf`, whose names are free on crates.io (2026-11-01; [P0b](../todos/done/P0b-crate-names-laminapdf.md))
- [x] Reserve names: `laminapdf` and `laminapdf-*` on crates.io (`0.0.0` placeholders), the `@laminapdf` organisation on npm (owner, 2026-11-01; [P1](../todos/done/P1-reserve-names-and-trusted-publishing.md))
- [x] Write ADR 0017's publishing part ([P1](../todos/done/P1-reserve-names-and-trusted-publishing.md)): crates.io trusted publishing; npm staged publishing with a stage-only token, since npm's trusted publishing rejects self-hosted runners; SBOMs on the GitHub release
- [x] Add the publish jobs to the release workflow, which publish on a `v*` tag and dry-run on a dispatch; each crate has its trusted publisher ([P1](../todos/done/P1-reserve-names-and-trusted-publishing.md)). The version PR stays `pnpm changeset version` and `cargo xtask version sync` by hand (ADR 0017 §2)
- [ ] Now that the repository is public: npm provenance, GitHub artifact attestations and a `release` environment that keeps the release bot's key from other workflows (ADR 0017 §7)
- [ ] Release once a day after the nightly, with npm trusted publishing and docs that match the release (the [releases plan](releases.md): [P4](../todos/done/P4-docs-match-the-release.md), [P5](../todos/done/P5-npm-trusted-publishing-and-version-only-releases.md), [P6](../todos/done/P6-daily-release.md)) The implementation and owner workflow-installation steps are in [`tools/release/README.md`](../../tools/release/README.md); P6 stays ongoing until installation and CI are complete.
- [x] Publish the first pre-release, `0.2.0-alpha.1` ([P2](../todos/done/P2-first-registry-prerelease.md)); verify install and quick start from the registries in each runtime ([P3](../todos/done/P3-registry-checks.md))
- [x] Publish the stability policy and changelog ([P2](../todos/done/P2-first-registry-prerelease.md)); enable cargo-semver-checks against the published version ([P3](../todos/done/P3-registry-checks.md))

### Cross-cutting
- [x] Capability matrix: generate a test-backed table of the ~180 operations (unsupported / read / write / round-trip) ([X2](../todos/done/X2-capability-matrix.md), [`docs/capabilities/`](../capabilities/README.md))
- [x] Nightly lane: differential tests vs pdf.js, PDFium, qpdf, pypdf; benchmarks; 10-minute fuzzing; scaling tests ([X3a](../todos/done/X3a-nightly-lane.md), `.github/workflows/nightly.yml`)
- [x] Nightly external link check of the docs ([X3c](../todos/done/X3c-nightly-link-check.md))
- [x] Nightly lane: hayro-vs-PDFium render diffs ([X3b](../todos/done/X3b-render-diff-nightly.md), `cargo xtask render-diff`)
- [ ] Nightly lane: veraPDF ([B8b](../todos/done/B8b-conformance-profiles-and-verapdf.md))
- [x] Docs link checker in `cargo xtask`, run in CI, so links to todos survive their moves between folders ([X1](../todos/done/X1-docs-link-checker.md))
- [ ] Decide product questions (§8; Q2 and Q5 answered 2026-10-28)

## 8. Open questions for the team

1. Which ships first, `@laminapdf/sdk` or the web reader? The research recommends packages first.
2. ~~Is a hosted lamina service a goal, or just a reference deployment for forks?~~ **Answered 2026-10-28 (project owner):** lamina **will** be a hosted service with user login and cloud features (accounts, cloud storage and sync), not just a reference deployment. Privacy-first local processing stays the default: the apps work without an account, and nothing is uploaded unless the user opts in. ADR 0011 records the topology; the `auth` spike (§4) picks the login stack.
3. Names: package scope, crate names, domain. **Domain answered 2026-10-31 (project owner):** `laminapdf.com` ([ADR 0011 §1](../decisions/0011-cloudflare-topology-and-privacy.md#1-topology)); since 2026-11-05 the website holds the apex and the reader `app.laminapdf.com` ([website plan](website.md)). **Package scope answered 2026-10-31 (project owner):** the npm packages are published under **`@laminapdf`** (`@laminapdf/sdk`, `@laminapdf/wasm`, …), not `@lamina`; the TypeScript API package is `@laminapdf/sdk` (was `@lamina/pdf`, ADR 0016) ([P0](../todos/done/P0-npm-scope-laminapdf.md), ADR 0015). Crate names (`lamina-*`, the façade `lamina`) and the product name stayed lamina at first; reserving them waits for publishing ([Publishing (later)](#publishing-later)). **Crate names answered 2026-11-01 (project owner):** `laminapdf-*` and the façade `laminapdf`, since `lamina` on crates.io is taken ([P0b](../todos/done/P0b-crate-names-laminapdf.md), ADR 0015). **Product name answered 2026-11-01 (project owner):** Lamina PDF. The public names follow it; ADR 0015's amendment lists them ([P0c](../todos/done/P0c-lamina-pdf-names.md)).
4. Are the Mac App Store, Flathub and mobile (Tauri mobile vs UniFFI with native UIs) in scope, and when? For mobile, the [UI/UX plan](ui-ux.md#54-mobile-and-touch-r17) recommends the responsive web reader first ([U3a](../todos/done/U3a-responsive-mobile-and-touch.md)), then the `mobile-shell` spike ([U3b](../todos/ready/U3b-mobile-shell-spike.md)) and ADR 0025 before any native app work. For a mobile SDK for other developers, the [shareable libraries plan](shareable-libraries.md#l6-mobile) adds a `lamina-ffi` UniFFI spike (L6a, whose measurements feed U3b) and a mobile SDK plan after ADR 0025 (L6b).
5. ~~Who is the legal entity for code signing?~~ **Answered 2026-10-28 (project owner):** the owner, personally, as the publisher, code-signs the desktop app for Windows, macOS and Linux. To check: Windows Artifact Signing accepts public-trust individuals only in the US and Canada; if the owner is elsewhere, Windows needs an OV certificate in a cloud HSM (§6). ADR 0014 records the identities and key custody.
