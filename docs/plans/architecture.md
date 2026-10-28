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
3. **The save planner owns correctness.** `SaveConstraint` (Free / AppendOnly / RequiresRewrite) makes signature-breaking or redaction-leaking saves impossible by default. Fixed order: redact → full rewrite → sign → append-only saves.
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
| 0003 | Reader tolerance policy and `Limits` defaults | Round B | Ready |
| 0005 | Document model: lazy COS overlay, typed views, stable `LaminaId`s | Round B | Propose |
| 0006 | Op log, transactions, undo semantics | Round B | Propose |
| 0007 | Save planner and `SaveConstraint` | Round B | Propose |
| 0016 | Package design: ESM-only, export conditions, split WASM modules, size budgets | Round B | Propose |
| 0017 | Versioning and release: lockstep core, OIDC trusted publishing, attestations | Round B | Propose |
| 0018 | Test strategy: goldens, differential corpus, fuzzing, veraPDF | Round B | Ready |
| 0004 | `RenderBackend`; PDFium default; hayro oracle; MuPDF prohibited | Round C | Propose |
| 0009 | Web client: React chrome + framework-free viewer core; typed worker RPC | Round C | Propose |
| 0010 | Platform adapter and capability flags | Round C | Propose |
| 0011 | Cloudflare topology, privacy/retention rules and the hosted service (accounts, cloud storage, sync) | Round C | Propose |
| 0008 | Desktop shell: Tauri 2, process model, tile transport, Linux fallback | Round D (after IPC spike) | Propose |
| 0014 | Desktop distribution: update Worker + R2, channels, signing (the owner as publisher), key custody | Round D | Propose |
| 0012 | Collaboration: one Durable Object per document, last-writer-wins records | Later (with collaboration) | Defer |
| 0013 | Heavy jobs: Workflows + Containers; OCR/conversion engines and licences | Later (with OCR) | Defer |

## 4. Spikes to close open questions

The research couldn't answer these. Each is a small, time-boxed spike under `spikes/` with a written result.

| Spike | Question | Blocks |
| --- | --- | --- |
| `desktop-ipc` | Throughput of Tauri Channel vs `ipc::Response` vs a `lamina://` scheme for page tiles on macOS, Windows and Linux; do WKWebView and WebKitGTK honour COOP/COEP? | ADR 0008 |
| `pdfium-wasm` | Load PDFium-WASM in a worker next to the lamina core: bundle size, memory, render time; reload after an incremental save | ADR 0004 |
| `krilla-fit` | Can krilla write into an existing object graph, or do we build our own builder over `lamina-cos`? | Library plan, Round B |
| `hayro-parity` | hayro vs PDFium pass rate on the pdf.js and PDFBox corpora: the baseline for switching the default | ADR 0004 |
| `desktop-print` | How to print faithfully from Tauri on each OS (native print of rendered pages vs webview print), given the `hasNativePrint` capability | Round D |
| `auth` | Better Auth / Auth.js on D1 vs hosted vendors, for the hosted service's user login and accounts (in scope since 2026-10-28, §8 Q2); run before the first round that needs accounts | ADR 0011 |

## 5. Rounds (one pull request each)

Each round is one focused pull request, in its own worktree, reviewed before merge.

- **Round A — Foundations.** ADRs 0001, 0002, 0015, 0019. Monorepo skeleton (`crates/`, `packages/`, `apps/`, `fixtures/`, `tools/xtask`, `deny.toml`, `rust-toolchain.toml`, pnpm workspace). CI fast lane. Port the spike's core into `crates/lamina-syntax` and `lamina-cos`, keeping its tests.
- **Round B — Core model and the first package.** ADRs 0003, 0005–0007, 0016–0018. `lamina-model`, `lamina-ops`, `lamina-save`, `lamina-build`; the `lamina` façade crate; `lamina-wasm` + `@lamina/pdf` with generation; the `krilla-fit` spike. CI full lane with golden tests. First pre-release to npm and crates.io.
- **Round C — Web reader.** ADRs 0004, 0009–0011. `pdfium-wasm` and `hayro-parity` spikes; `@lamina/viewer`, `@lamina/ui`, `apps/web` on Workers static assets; Playwright tests; preview deploys per PR.
- **Round D — Desktop reader.** `desktop-ipc` spike; ADRs 0008, 0014; `apps/desktop` (Tauri 2); `apps/updates` Worker + R2; signing and notarisation in CI.
- **Later rounds.** Editing features by phase ([06 §9](../research/pdf/06-feature-inventory.md)), collaboration (ADR 0012), heavy jobs (ADR 0013). Each feature group gets its own plan and pull requests.

Product-level detail (features, milestones, acceptance criteria) lives in the product plans (library, web app, desktop app, full-featured editor), which follow in the next pull request.

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
- [x] Write ADR 0001 (core language, licence, bytes in / bytes out)
- [x] Write ADR 0002 (binding strategy)
- [x] Write ADR 0015 (monorepo layout and tooling)
- [x] Write ADR 0019 (supply-chain and licence hygiene)
- [x] Create the root Cargo workspace (`[workspace.lints]`, `rust-toolchain.toml`, edition 2024, MSRV policy)
- [x] Create the pnpm workspace (`pnpm-workspace.yaml`, shared `tsconfig.base.json`, Biome config)
- [x] Add `tools/xtask` with `cargo xtask check`, `test`, `wasm`, `fmt` (the single task entry point, per the code-organization guide)
- [x] Add `deny.toml` (licence allow-list without AGPL/GPL; advisories) and an npm licence check
- [x] CI fast lane: fmt, clippy `-D warnings`, nextest, wasm32 build, TS typecheck/lint/test, cargo-deny (the wasm size budget moves to Round B, with the first `.wasm` artefact from `lamina-wasm`)
- [ ] Port the spike reader/writer into `crates/lamina-syntax` + `crates/lamina-cos`, with its fixtures moved to `fixtures/`
- [ ] Reserve package names: `lamina*` on crates.io and the `@lamina` scope on npm
- [ ] Update `CLAUDE.md` (layout, commands, decided architecture)

### Round B — Core model and the first package
- [ ] Write ADRs 0003, 0005, 0006, 0007, 0016, 0017, 0018
- [ ] `krilla-fit` spike, then decide the generation strategy
- [ ] `lamina-model`: typed pages, annotations, fields, outline; stable IDs
- [ ] `lamina-ops`: `Op` enum, `transact()`, inverses, undo/redo
- [ ] `lamina-save`: save planner, `SaveConstraint`, streaming `WriteSink`
- [ ] `lamina-build`: builder API (`Document → page → surface → finish() → Document`)
- [ ] `lamina` façade crate: re-exports the public API of the core crates; this is what goes to crates.io
- [ ] `lamina-text`: font embedding and subsetting for generation (see [03](../research/pdf/03-text-and-fonts.md))
- [ ] `lamina-wasm` (wasm-bindgen + tsify) and `@lamina/pdf` (ESM-only, `init()`, export conditions; publint + attw)
- [ ] Golden-file and corpus test harness in CI (full lane); fuzz targets under `fuzz/`
- [ ] Changesets + Cargo version sync; OIDC trusted publishing; first `0.1.0-alpha` release

### Round C — Web reader
- [ ] `pdfium-wasm` and `hayro-parity` spikes, with written results
- [ ] Write ADRs 0004, 0009, 0010, 0011
- [ ] `lamina-render` with the `RenderBackend` trait and PDFium/hayro adapters
- [ ] `@lamina/render-pdfium` (ships pdfium.wasm, used by the web reader) and `@lamina/render-hayro` (pure-Rust renderer as its own .wasm), both optional so `@lamina/pdf` stays small
- [ ] `@lamina/viewer`: worker RPC, page virtualiser, canvas pool, text layer, search
- [ ] `@lamina/ui` (React 19) and `apps/web` on Workers static assets; per-PR previews
- [ ] Playwright and visual regression tests; accessibility checks

### Round D — Desktop reader
- [ ] `desktop-ipc` and `desktop-print` spikes, then ADR 0008
- [ ] `apps/desktop` (Tauri 2) reusing `@lamina/ui`. Process model: all parsing and rendering of untrusted files runs in a sandboxed engine process; the main process only does file I/O (mmap, file watching) and passes bytes and tiles across the boundary
- [ ] `apps/updates`: Worker + R2 update manifests with channels and staged rollout
- [ ] Signing: Apple notarisation, Windows Artifact Signing, Linux AppImage/deb; tauri-action release workflow
- [ ] Write ADR 0014

### Cross-cutting
- [ ] Capability matrix: generate a test-backed table of the ~180 operations (unsupported / read / write / round-trip)
- [ ] Nightly lane: differential tests vs pdf.js, PDFium, qpdf, pypdf; hayro-vs-PDFium render diffs; veraPDF; benchmarks
- [ ] Decide product questions (§8; Q2 and Q5 answered 2026-10-28)

## 8. Open questions for the team

1. Which ships first, `@lamina/pdf` or the web reader? The research recommends packages first.
2. ~~Is a hosted lamina service a goal, or just a reference deployment for forks?~~ **Answered 2026-10-28 (project owner):** lamina **will** be a hosted service with user login and cloud features (accounts, cloud storage and sync), not just a reference deployment. Privacy-first local processing stays the default: the apps work without an account, and nothing is uploaded unless the user opts in. ADR 0011 records the topology; the `auth` spike (§4) picks the login stack.
3. Names: package scope, crate names, domain.
4. Are the Mac App Store, Flathub and mobile (Tauri mobile vs UniFFI with native UIs) in scope, and when?
5. ~~Who is the legal entity for code signing?~~ **Answered 2026-10-28 (project owner):** the owner, personally, as the publisher, code-signs the desktop app for Windows, macOS and Linux. To check: Windows Artifact Signing accepts public-trust individuals only in the US and Canada; if the owner is elsewhere, Windows needs an OV certificate in a cloud HSM (§6). ADR 0014 records the identities and key custody.
