# Plan: lamina architecture

- **Status:** Draft, for review
- **Date:** 2026-10-28
- **Research:** [docs/research/architecture/app-and-package-architecture.md](../research/architecture/app-and-package-architecture.md) (and its [notes](../research/architecture/notes/)), plus PDF research [05 §8](../research/pdf/05-creating-pdfs-from-code.md), [06](../research/pdf/06-feature-inventory.md) and [07](../research/pdf/07-spike-rust-and-typescript.md)
- **Scope:** the shared architecture behind every lamina product. The product plans (library, web app, desktop app) build on this one.

## 1. Goal

One architecture that lets lamina ship, in order:

1. **Packages:** a Rust crate family (`lamina`) and a TypeScript package (`@lamina/pdf`) that generate PDFs, and later read and edit them.
2. **A web app** on Cloudflare: a reader first, growing into a full editor.
3. **A desktop app** (macOS, Windows, Linux) with the same features, with downloads and updates served from Cloudflare.

All three share one engine, so each of the ~180 operations in the [feature inventory](../research/pdf/06-feature-inventory.md) is built once.

## 2. The architecture in one picture

```
   apps/web (Cloudflare)        apps/desktop (Tauri 2)          packages
   React chrome +               same UI packages +              @lamina/pdf (npm)
   framework-free viewer        native menus, updater           lamina (crates.io)
        │ typed RPC                  │ Channel / lamina://            │ direct calls
        ▼ (Web Worker)               ▼ (native, sandboxed process)    ▼ (WASM or native)
   ┌──────────────────────── lamina core (Rust, bytes in / bytes out) ───────────────────────┐
   │ crates: lamina-syntax · -cos · -model · -ops · -save · -build · -text (façade: lamina)   │
   └───────────────────────────────┬─────────────────────────────────────────────────────────┘
                                   ▼
        lamina-render: RenderBackend trait → PDFium (default) · hayro (CI oracle → future default)

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
| [0005](../decisions/0005-document-model.md) | Document model: lazy COS overlay, typed views, stable `LaminaId`s | Round B | Accepted (generation strategy Proposed) |
| [0006](../decisions/0006-op-log-and-undo.md) | Op log, transactions, undo semantics | Round B | Accepted |
| [0007](../decisions/0007-save-planner.md) | Save planner and `SaveConstraint` | Round B | Accepted |
| 0016 | Package design: ESM-only, export conditions, split WASM modules, size budgets | Round B | Propose |
| 0017 | Versioning and release: lockstep core, OIDC trusted publishing, attestations | Round B | Propose |
| [0018](../decisions/0018-test-strategy.md) | Test strategy: goldens, differential corpus, fuzzing, veraPDF | Round B | Accepted |
| 0004 | `RenderBackend`; PDFium default; hayro oracle; MuPDF prohibited | Round C | Propose |
| 0009 | Web client: React chrome + framework-free viewer core; typed worker RPC | Round C | Propose |
| 0010 | Platform adapter and capability flags | Round C | Propose |
| 0011 | Cloudflare topology, privacy/retention rules and the hosted service (accounts, cloud storage, sync) | Round C | Propose |
| 0008 | Desktop shell: Tauri 2, process model, tile transport, Linux fallback | Round D (after IPC spike) | Propose |
| 0014 | Desktop distribution: update Worker + R2, channels, signing (the owner as publisher), key custody | Round D | Propose |
| 0024 | App shell UX: inverted-L layout, modes, one command registry, selection surfaces, page appearance modes, phone layout ([UI/UX plan](ui-ux.md)) | Round U (U1a) | Propose |
| 0025 | Mobile shell: Tauri 2 mobile vs UniFFI with native UI (after the `mobile-shell` spike) | Round U (U3b) | Defer |
| 0012 | Collaboration: one Durable Object per document, last-writer-wins records | Later (with collaboration) | Defer |
| 0013 | Heavy jobs: Workflows + Containers; OCR/conversion engines and licences | Later (with OCR) | Defer |
| 0026 | Error and diagnostic contract | [Shareable libraries](shareable-libraries.md#6-decisions-to-record-adrs) L1 | Propose |
| 0027 | Coordinates, units and page indexing in public APIs | Shareable libraries L1 | Propose |
| 0028 | Façade binding rules, jobs, cancellation and progress | Shareable libraries L1 | Propose |
| 0029 | Mobile bindings and packaging (UniFFI pin, SPM, AARs) | Shareable libraries L6 | Propose |
| 0030 | Extraction output (`lamina.extract/1`) and model plug-ins | Shareable libraries L3 | Propose |
| 0031 | CLI and MCP contract | Shareable libraries L4 | Propose |

## 4. Spikes to close open questions

The research couldn't answer these. Each is a small, time-boxed spike under `spikes/` with a written result.

| Spike | Question | Blocks |
| --- | --- | --- |
| `desktop-ipc` | Throughput of Tauri Channel vs `ipc::Response` vs a `lamina://` scheme for page tiles on macOS, Windows and Linux; do WKWebView and WebKitGTK honour COOP/COEP? | ADR 0008 |
| `pdfium-wasm` | Load PDFium-WASM in a worker next to the lamina core: bundle size, memory, render time; reload after an incremental save | ADR 0004 |
| `krilla-fit` | Can krilla write into an existing object graph, or do we build our own builder over `lamina-cos`? | Library plan, Round B. **Done** ([result](../../spikes/krilla-fit/README.md)): no, krilla only emits finished bytes; recommends an own builder over `lamina-cos` reusing `pdf-writer`/`subsetter`/`skrifa`/HarfRust (ADR 0005 §6) |
| `hayro-parity` | hayro vs PDFium pass rate on the pdf.js and PDFBox corpora: the baseline for switching the default | ADR 0004. **Done** ([result](../../spikes/hayro-parity/README.md)): 81.8 % of 2,196 pages pass and 90.8 % pass or differ slightly at 96 DPI; where hayro is clearly wrong, it is mostly fonts, annotations without appearance streams and transparency groups (CalRGB/CalGray differences still need a reference rendering to judge). Recommends PDFium as the default everywhere and hayro as the CI oracle, with switch criteria for ADR 0004 |
| `desktop-print` | How to print faithfully from Tauri on each OS (native print of rendered pages vs webview print), given the `hasNativePrint` capability | Round D |
| `mobile-shell` | Tauri 2 mobile vs UniFFI with native UI (SwiftUI, Compose): native selection-menu conflicts, Apple Pencil latency, share sheet and Files integration, Vision / ML Kit OCR access ([U3b](../todos/ready/U3b-mobile-shell-spike.md)) | ADR 0025, §8 Q4 |
| `auth` | Better Auth / Auth.js on D1 vs hosted vendors, for the hosted service's user login and accounts (in scope since 2026-10-28, §8 Q2); run before the first round that needs accounts | ADR 0011 |

## 5. Rounds

Each round is split into todos, one pull request each, every one in its own worktree and reviewed before merge (see [Todos](#todos) below).

- **Round A — Foundations** (done: [A1](../todos/done/A1-foundation-adrs.md), [A2](../todos/done/A2-monorepo-skeleton.md), [A3](../todos/done/A3-port-spike-core.md)). ADRs 0001, 0002, 0015, 0019. Monorepo skeleton (`crates/`, `packages/`, `apps/`, `fixtures/`, `tools/xtask`, `deny.toml`, `rust-toolchain.toml`, pnpm workspace). CI fast lane. Port the spike's core into `crates/lamina-syntax` and `lamina-cos`, keeping its tests.
- **Round B — Core model and the first package** (todos B1a–B9c, listed in the [library plan §7](pdf-library.md#7-rounds)). ADRs 0003, 0005–0007, 0016–0018. `lamina-model`, `lamina-ops`, `lamina-save`, `lamina-build`; the `lamina` façade crate; `lamina-wasm` + `@lamina/pdf` with generation; the `krilla-fit` spike. CI full lane with golden tests ([X4](../todos/ready/X4-full-lane-gate.md)). The first `0.1.0-alpha` is versioned and tagged but **not published**: crates and npm packages stay local until the owner decides to publish (see [Publishing (later)](#publishing-later)).
- **Round C — Web reader** (todos C1a–C7c, listed in the [web-app plan §6](web-app.md#6-rounds)). ADRs 0004, 0009–0011. `pdfium-wasm` and `hayro-parity` spikes; `@lamina/viewer`, `@lamina/ui`, `apps/web` on Workers static assets; Playwright tests; preview deploys per PR.
- **Round D — Desktop reader** (todos D1a–D7c, listed in the [desktop-app plan §6](desktop-app.md#6-rounds)). `desktop-ipc` spike; ADRs 0008, 0014; `apps/desktop` (Tauri 2); `apps/updates` Worker + R2; signing and notarisation in CI.
- **Round U — UI/UX** (todos U1a–U5c, listed in the [UI/UX plan §6](ui-ux.md#6-rounds)). ADR 0024; `mobile-shell` spike (ADR 0025). Runs alongside C, D and E: command registry and palette, inverted-L shell and modes, selection surfaces and copy modes, reading-order corpus, page appearance modes, phone layout, network indicator, reader mode, pledge, benchmark. Its v0.1 parts gate C7a and C7c.
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
- [x] CI fast lane: fmt, clippy `-D warnings`, nextest, wasm32 build, TS typecheck/lint/test, cargo-deny (the wasm size budget moves to Round B, with the first `.wasm` artefact from `lamina-wasm`)
- [x] Port the spike reader/writer into `crates/lamina-syntax` + `crates/lamina-cos`, with its fixtures moved to `fixtures/`
- [ ] ~~Reserve package names~~ ([P1](../todos/ready/P1-reserve-names-and-trusted-publishing.md)) deferred: crates and npm packages stay local for now (decided 2026-10-28); see [Publishing (later)](#publishing-later)
- [ ] Update `CLAUDE.md` (layout, commands, decided architecture)

### Round B — Core model and the first package
- [x] Write ADRs 0003, 0005, 0006, 0007, 0018 ([B1a](../todos/done/B1a-round-b-adrs.md); 0005 §6, the generation strategy, settled by `krilla-fit`)
- [ ] Write ADRs 0016 ([B6a](../todos/ready/B6a-lamina-wasm-and-adr-0016.md)), 0017 ([B7b](../todos/ready/B7b-versioning-and-0-1-0-alpha.md))
- [x] `krilla-fit` spike ([B1b](../todos/done/B1b-krilla-fit-spike.md), [result](../../spikes/krilla-fit/README.md)), then decide the generation strategy (Option B, recorded in [ADR 0005 §6](../decisions/0005-document-model.md))
- [ ] `lamina-model`: typed pages, annotations, fields, outline; stable IDs ([B2a](../todos/done/B2a-limits-owned-document.md) owned `Document` and `Limits`, [B2b](../todos/done/B2b-standard-security-decryption.md) decryption, [B2c](../todos/ongoing/B2c-lamina-model-corpus-fuzz.md) views and IDs; annotations and fields in the editor rounds)
- [ ] `lamina-ops`: `Op` enum, `transact()`, inverses, undo/redo ([B3a](../todos/ready/B3a-overlay-and-ops.md))
- [ ] `lamina-save`: save planner, `SaveConstraint`, streaming `WriteSink` ([B3b](../todos/ready/B3b-lamina-save-writers.md), [B3c](../todos/ready/B3c-save-planner-and-roundtrip.md))
- [ ] Move the writer (`Writer`, `incremental_update`, `XrefEol`) from `lamina-cos` into `lamina-save`, keeping the determinism and golden tests ([B3b](../todos/ready/B3b-lamina-save-writers.md))
- [ ] `lamina-build`: builder API (`Document → page → surface → finish() → Document`) ([B5a](../todos/ready/B5a-lamina-build-core.md), [B5b](../todos/ready/B5b-builder-text-and-document-features.md))
- [ ] Promote the spike's content builder and golden samples (`hello`, `imaging_demo`), now test support in `crates/lamina-cos/tests/common/`, into `lamina-build` (API and examples) ([B5a](../todos/ready/B5a-lamina-build-core.md))
- [ ] `lamina` façade crate: re-exports the public API of the core crates; this is what goes to crates.io ([B5c](../todos/ready/B5c-lamina-facade-and-examples.md))
- [ ] `lamina-text`: font embedding and subsetting for generation (see [03](../research/pdf/03-text-and-fonts.md)) ([B4a](../todos/ready/B4a-font-loading-and-subsetting.md), [B4b](../todos/ready/B4b-shaping-bidi-line-breaking.md), [B4c](../todos/ready/B4c-standard-14-and-simple-fonts.md))
- [ ] `lamina-wasm` (wasm-bindgen + tsify) and `@lamina/pdf` (ESM-only, `init()`, export conditions; publint + attw) ([B6a](../todos/ready/B6a-lamina-wasm-and-adr-0016.md), [B6b](../todos/ready/B6b-lamina-pdf-package.md), [B6c](../todos/ready/B6c-runtime-matrix.md))
- [ ] Golden-file and corpus test harness in CI (full lane); fuzz targets under `fuzz/` ([B2c](../todos/ongoing/B2c-lamina-model-corpus-fuzz.md) harness and fuzz targets, [X4](../todos/ready/X4-full-lane-gate.md) full lane)
- [ ] Changesets + Cargo version sync; tag `0.1.0-alpha` locally (no registry publish yet) ([B7b](../todos/ready/B7b-versioning-and-0-1-0-alpha.md); docs site [B7a](../todos/ready/B7a-docs-site.md))

### Round C — Web reader

Todos: see the [web-app plan §6](web-app.md#6-rounds) (C1a–C7c).

- [ ] `pdfium-wasm` and `hayro-parity` spikes, with written results (`hayro-parity` done: [C1a](../todos/done/C1a-hayro-parity-spike.md), [result](../../spikes/hayro-parity/README.md); `pdfium-wasm`: [C1b](../todos/ready/C1b-pdfium-wasm-spike.md))
- [ ] Write ADRs 0004, 0009, 0010, 0011
- [ ] `lamina-render` with the `RenderBackend` trait and PDFium/hayro adapters
- [ ] `@lamina/render-pdfium` (ships pdfium.wasm, used by the web reader) and `@lamina/render-hayro` (pure-Rust renderer as its own .wasm), both optional so `@lamina/pdf` stays small
- [ ] `@lamina/viewer`: worker RPC, page virtualiser, canvas pool, text layer, search
- [ ] `@lamina/ui` (React 19) and `apps/web` on Workers static assets; per-PR previews
- [ ] Playwright and visual regression tests; accessibility checks

### Round D — Desktop reader

Todos: see the [desktop-app plan §6](desktop-app.md#6-rounds) (D1a–D7c).

- [ ] `desktop-ipc` and `desktop-print` spikes, then ADR 0008
- [ ] `apps/desktop` (Tauri 2) reusing `@lamina/ui`. Process model: all parsing and rendering of untrusted files runs in a sandboxed engine process; the main process only does file I/O (mmap, file watching) and passes bytes and tiles across the boundary
- [ ] `apps/updates`: Worker + R2 update manifests with channels and staged rollout
- [ ] Signing: Apple notarisation, Windows Artifact Signing, Linux AppImage/deb; tauri-action release workflow
- [ ] Write ADR 0014

### Publishing (later)

Decided 2026-10-28 by the project owner: crates and npm packages stay local (workspace and path dependencies, `pnpm pack` tarballs, git tags) until the owner decides to publish. When that happens:

Todos: [P1](../todos/ready/P1-reserve-names-and-trusted-publishing.md) and [P2](../todos/ready/P2-first-registry-prerelease.md), both blocked on the owner's decision to publish.

- [ ] Reserve names: `lamina` and `lamina-*` on crates.io, the `@lamina` scope on npm ([P1](../todos/ready/P1-reserve-names-and-trusted-publishing.md))
- [ ] Write ADR 0017's publishing part ([P1](../todos/ready/P1-reserve-names-and-trusted-publishing.md)): OIDC trusted publishing on npm and crates.io (first crates.io publish is manual), provenance, GitHub artifact attestations for `.wasm`
- [ ] Add the release workflow (Changesets version PR → publish job) and switch every crate to trusted-publishing-only ([P1](../todos/ready/P1-reserve-names-and-trusted-publishing.md))
- [ ] Publish the first pre-release; verify install and quick start from the registries in each runtime ([P2](../todos/ready/P2-first-registry-prerelease.md))
- [ ] Publish the stability policy and changelog; enable cargo-semver-checks against the published version ([P2](../todos/ready/P2-first-registry-prerelease.md))

### Cross-cutting
- [ ] Capability matrix: generate a test-backed table of the ~180 operations (unsupported / read / write / round-trip) ([X2](../todos/ready/X2-capability-matrix.md))
- [ ] Nightly lane: differential tests vs pdf.js, PDFium, qpdf, pypdf; hayro-vs-PDFium render diffs; veraPDF; benchmarks ([X3a](../todos/ready/X3a-nightly-lane.md); render diffs [X3b](../todos/ready/X3b-render-diff-nightly.md); veraPDF added by [B8b](../todos/ready/B8b-conformance-profiles-and-verapdf.md))
- [x] Docs link checker in `cargo xtask`, run in CI, so links to todos survive their moves between folders ([X1](../todos/done/X1-docs-link-checker.md))
- [ ] Decide product questions (§8; Q2 and Q5 answered 2026-10-28)

## 8. Open questions for the team

1. Which ships first, `@lamina/pdf` or the web reader? The research recommends packages first.
2. ~~Is a hosted lamina service a goal, or just a reference deployment for forks?~~ **Answered 2026-10-28 (project owner):** lamina **will** be a hosted service with user login and cloud features (accounts, cloud storage and sync), not just a reference deployment. Privacy-first local processing stays the default: the apps work without an account, and nothing is uploaded unless the user opts in. ADR 0011 records the topology; the `auth` spike (§4) picks the login stack.
3. Names: package scope, crate names, domain.
4. Are the Mac App Store, Flathub and mobile (Tauri mobile vs UniFFI with native UIs) in scope, and when? For mobile, the [UI/UX plan](ui-ux.md#54-mobile-and-touch-r17) recommends the responsive web reader first ([U3a](../todos/ready/U3a-responsive-mobile-and-touch.md)), then the `mobile-shell` spike ([U3b](../todos/ready/U3b-mobile-shell-spike.md)) and ADR 0025 before any native app work. For a mobile SDK for other developers, the [shareable libraries plan](shareable-libraries.md#l6-mobile) adds a `lamina-ffi` UniFFI spike (L6a, whose measurements feed U3b) and a mobile SDK plan after ADR 0025 (L6b).
5. ~~Who is the legal entity for code signing?~~ **Answered 2026-10-28 (project owner):** the owner, personally, as the publisher, code-signs the desktop app for Windows, macOS and Linux. To check: Windows Artifact Signing accepts public-trust individuals only in the US and Canada; if the owner is elsewhere, Windows needs an OV certificate in a cloud HSM (§6). ADR 0014 records the identities and key custody.
