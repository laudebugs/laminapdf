# C2b — `crates/lamina-render`: the `RenderBackend` trait, PDFium and hayro adapters, render goldens

| Field | Value |
| --- | --- |
| ID | `C2b` |
| Status | `done` |
| Plan | [web reader §6 C2. Render backend and ADRs](../../plans/web-app.md#c2-render-backend-and-adrs) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B2c](./B2c-lamina-model-corpus-fuzz.md), [C2a](./C2a-render-adrs.md) |
| Blocks | [L4b](../ready/L4b-cli-batch-render-schema.md), [L6b](../ready/L6b-mobile-sdk-plan.md), [C2c](./C2c-render-packages.md), [D2b](../ready/D2b-engine-process.md), [U2c](../ready/U2c-page-appearance-modes.md), [U3b](../ready/U3b-mobile-shell-spike.md), [E3h](../ready/E3h-images-to-pdf-and-page-export.md), [E13a](../ready/E13a-adr-0013-heavy-jobs.md), [H1](../ready/H1-auth-spike.md), [X3b](./X3b-render-diff-nightly.md) |
| Dependency notes | [C2a](../done/C2a-render-adrs.md) (ADR 0004 defines the trait); library B2 (`Document::open`, page boxes; [pdf-library B2](../../plans/pdf-library.md#b2-document-model)) |
| ADRs | Implements 0004. Follows [0001](../../decisions/0001-core-language-and-licence.md), [0003](../../decisions/0003-reader-tolerance-and-limits.md) (`Limits`), [0015](../../decisions/0015-monorepo-layout-and-tooling.md), [0018](../../decisions/0018-test-strategy.md) tier 8 and §2 lanes, [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `feat/c2b-lamina-render` |
| PR title | `feat(render): add lamina-render with the RenderBackend trait and pdfium and hayro adapters` |
| Size | L (600–1,500 lines incl. tests and xtask changes; PNG goldens excluded) |
| PR | [#113](https://github.com/laudebugs/lamina/pull/113) |

## Goal

Create the single Rust abstraction every lamina surface renders through. `crates/lamina-render` defines `RenderBackend` exactly as ADR 0004 specifies, with a PDFium adapter (native, `pdfium-render`) and a hayro adapter behind Cargo features, render goldens for both on `fixtures/`, and `cargo xtask render-golden`; the nightly hayro-vs-PDFium diff over the external corpus builds on this crate in [X3b](./X3b-render-diff-nightly.md). This is what desktop links natively and what the hayro WASM package (C2c) compiles.

## Context

- ADR 0004 (written in [C2a](../done/C2a-render-adrs.md)) — trait surface, adapters, two-parsers rule, switch criteria.
- [C1a result](../done/C1a-hayro-parity-spike.md) (`spikes/hayro-parity/README.md`) — thresholds and comparison metric to reuse; the nightly metric to track.
- [ADR 0018 §1 tier 8 and §2](../../decisions/0018-test-strategy.md#1-tiers) — render goldens are PNGs per page compared with perceptual thresholds, updated only by `cargo xtask render-golden --update`; full lane runs them on PRs; nightly runs hayro vs PDFium.
- [Rust guidelines](../../guidelines/rust.md) and [code organization §2](../../guidelines/code-organization.md#2-boundaries-and-dependency-direction) — `lamina-render` is an **edge** crate when it links PDFium (FFI/dynamic loading); the trait itself stays pure. Load `/rust-best-practices` before writing code.
- [Rendering notes](../../research/architecture/notes/rendering.md) §3 (`FPDF_ANNOT` separates annotation rendering).

## Scope

**In scope**

- `crates/lamina-render` (layer per ADR 0004; likely `core` for the trait + hayro, with the PDFium adapter in `crates/lamina-render-pdfium` as `edge` if `unsafe`/dynamic loading is needed — decide by the portability rules in xtask).
- Trait, types (`RenderRequest`, `RenderFlags`, `PixelFormat`, `RenderedTile`, `RenderError` via `thiserror`), the hayro adapter behind feature `hayro`; the PDFium adapter in the separate edge crate `crates/lamina-render-pdfium` (ADR 0004 §5).
- Golden PNGs in `fixtures/golden/render/{pdfium,hayro}/` for every `fixtures/corpus` page at 96 DPI; `cargo xtask render-golden [--update]`; comparison helper with perceptual thresholds.
- Full-lane job running render goldens over `fixtures/corpus` (committed fixtures only; no external corpus in this PR).
- `deny.toml`/licence allow-list updates for new crates.

**Out of scope** (and which todo covers it)

- The PDFium-**WASM** TS adapter and the hayro `.wasm` package → [C2c](../done/C2c-render-packages.md).
- Worker RPC and tiles in the browser → [C3a](../done/C3a-viewer-worker-rpc.md), [C3b](../done/C3b-viewer-virtualiser-and-zoom.md).
- Desktop engine process wiring → desktop D2.
- Nightly hayro-vs-PDFium diffs over the external corpus (`cargo xtask render-diff`, reporting the C1a metric) → [X3b](./X3b-render-diff-nightly.md); `cargo xtask corpus fetch` and the pinned corpus manifest (including the pdf.js/PDFBox corpora C1a used) → [X3a](../done/X3a-nightly-lane.md). This PR does not add or extend `corpus fetch`.

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`. Load `/rust-best-practices`.
- [x] Add `crates/lamina-render/Cargo.toml` (workspace package fields, `[lints] workspace = true`, `[package.metadata.lamina] layer = …`, feature `hayro`, default none; `crates/lamina-render-pdfium` gets its own manifest, layer `edge`) and register it in the root `Cargo.toml`.
- [x] `src/lib.rs`: `pub trait RenderBackend { fn page_count(&self) -> Result<u32, RenderError>; fn page_size(&self, page: PageIndex) -> Result<PageSize, RenderError>; fn render(&self, req: &RenderRequest, out: &mut [u8]) -> Result<RenderedTile, RenderError>; }` plus a loader (`fn open(bytes/ByteSource, password: Option<&str>, limits: &Limits)`) — exact signatures per ADR 0004. `RenderRequest` = page, scale or target pixel size, source rect (for tiles), rotation, flags (annotations, form widgets, print, background), pixel format (RGBA8 premultiplied or not; document which).
- [x] Validate requests: reject zero/oversized dimensions, cap total pixels per request (a `Limits`-style `max_tile_pixels`) so hostile page sizes cannot allocate unbounded buffers; return `RenderError` codes, never panic.
- [x] `src/hayro.rs` (feature `hayro`): wrap `hayro` 0.8.x; map errors; convert its pixmap into the caller's buffer without extra allocation where possible.
- [x] `src/pdfium.rs` (feature `pdfium`, or the separate edge crate): wrap `pdfium-render`, bind the pinned PDFium library from ADR 0004 (path via env var / xtask-fetched), render with/without `FPDF_ANNOT`, handle password, map `FPDF_GetLastError` to `RenderError`. Every `unsafe` block (if any) has `// SAFETY:`.
- [x] `tools/xtask`: add `fetch-pdfium` (pinned URL + SHA-256, per host OS, cached in a gitignored dir), `render-golden [--update]` (renders every page of `fixtures/corpus/*.pdf` with both adapters to `fixtures/golden/render/<backend>/<file>-p<N>.png`); `doctor` checks the PDFium library is available.
- [x] `tests/golden.rs`: for each corpus page and backend, render and compare against the golden with the threshold from C1a (SSIM ≥ 0.99 against its **own** backend's golden; cross-backend thresholds only in the nightly diff). A test never writes a golden.
- [x] **Feature gating so the fast lane never runs goldens.** `cargo xtask check`/`test` use `--workspace --all-features`, which turns on `pdfium` and `hayro`. So: (a) the PDFium adapter loads the library **dynamically at runtime** (no build-time link), so `--all-features` builds and clippy pass on every OS without a fetched library; (b) every golden test and every test that needs the PDFium library is `#[ignore = "render golden: run via cargo xtask render-golden --check (full lane, Linux)"]`; (c) `cargo xtask render-golden --check` runs `cargo test -p lamina-render --features pdfium,hayro -- --ignored` after `fetch-pdfium`, and is the only thing that runs them. The fast lane and dev machines therefore run only the pure tests (request validation, hayro error mapping), never the PDFium/hayro goldens. (`cargo test -- --include-ignored` locally runs them if you have fetched PDFium.)
- [x] `tests/errors.rs`: wrong password, damaged file (`hello-broken.pdf`), oversize request, out-of-range page → typed errors.
- [x] `tests/determinism.rs`: rendering the same page twice gives identical bytes for each backend.
- [x] CI: add `cargo xtask render-golden --check` to the full lane (Linux; PDFium fetched and cached) — see [X4](../done/X4-full-lane-gate.md). The nightly external-corpus diff is [X3b](./X3b-render-diff-nightly.md)'s.
- [x] `deny.toml`: allow the licences `pdfium-render`, `hayro` and their deps bring (all permissive); `cargo xtask deny` passes.
- [x] `README.md` for the crate: what the trait is, feature flags, how to fetch PDFium, how to update goldens.
- [x] Update `CLAUDE.md` layout (new crate) and commands (`cargo xtask fetch-pdfium`, `cargo xtask render-golden`).

## Acceptance criteria

- [x] `cargo xtask render-golden --check` (that is, `cargo test -p lamina-render --features pdfium,hayro -- --ignored` with PDFium fetched) passes on Linux; golden tests cover every page of every file in `fixtures/corpus/` for both backends (except files listed with a reason, e.g. encrypted ones hayro cannot open).
- [x] `cargo xtask check` passes on macOS, Linux and Windows **without** a fetched PDFium library and runs no golden test (they are `#[ignore]`d), including clippy `-D warnings` native and (for the pure part) wasm32, portability rules for the crate's declared layer, and cargo-deny.
- [x] `cargo xtask render-golden --update` regenerates goldens deterministically (running it twice yields no diff).
- [x] The full-lane CI job runs `cargo xtask render-golden --check` on PRs touching `crates/`, `fixtures/` or `tools/`; the fast lane does not.
- [x] No panics on hostile input: oversize and broken-file tests return errors.

## Post-merge checks

None.

## Tests

Integration tests in `crates/lamina-render/tests/` (golden, errors, determinism) using `fixtures/corpus/`; goldens in `fixtures/golden/render/` (the nightly external-corpus differential is X3b's). A `fuzz/fuzz_targets/render_hayro.rs` target (hayro path only, small DPI) is a nice-to-have; add it if time permits and note it otherwise.

## Docs to update

Crate README; `CLAUDE.md` (layout + commands); `fixtures/README.md` (render goldens section, provenance: generated, ours); tick "Create `crates/lamina-render`…" in [web-app.md §9](../../plans/web-app.md#9-to-dos) (the "Add golden render tests and the nightly hayro-vs-PDFium diff" item is ticked by X3b, which completes it) and the `lamina-render` line in [architecture §7](../../plans/architecture.md#7-to-dos).

## Risks and open questions

- **Golden PNG size in git**: ~14 corpus files × pages × 2 backends at 96 DPI is small, but keep PNGs optimised (oxipng) and consider Git LFS only if they exceed a few MB.
- **Cross-OS pixel differences** in PDFium/FreeType: goldens are `#[ignore]`d and run only via `render-golden --check` in the Linux full lane; documented in the crate README.
- **pdfium-render API churn** (0.9.x) and its single maintainer ([architecture §6](../../plans/architecture.md#6-risks)): wrap it thinly so it can be replaced by direct FFI.
- **Layer classification**: dynamic loading of PDFium is platform code; if xtask's core rules reject it, split the adapter into an edge crate (decide early in the PR).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome

- **Crates:**
  - [`crates/lamina-render`](../../../crates/lamina-render/README.md) (core): the trait, `TilePlan`, `RenderLimits`, `RenderError`, and `HayroBackend` behind `hayro`.
  - [`crates/lamina-render-pdfium`](../../../crates/lamina-render-pdfium/README.md) (edge): `PdfiumBackend`, bound at runtime to `chromium/7881`.
- **ADR 0004** is amended to the implemented contract:
  - `render` returns the `TilePlan`, and `limits()`/`plan()` exist;
  - `RenderLimits::max_input_bytes`;
  - the native adapter records pending reads instead of using `FPDFAvail_*`;
  - `pdfium-render` doesn't expose progressive rendering, so a running native render finishes (a follow-up if needed).
- **`CancelToken`** moved from the façade to `lamina-syntax`, so core crates below the façade take the same type. The façade re-exports it unchanged.
- **xtask:**
  - `fetch-pdfium`: pinned SHA-256 for macOS arm64 and x64, Linux x64 and arm64, and Windows x64; it unpacks the notices too.
  - `render-golden` gains the corpus part, which runs `lamina-render-pdfium`'s ignored tests, or writes the goldens with `--update`.
- **Deviations:**
  - **macOS, not Linux:** the goldens and their CI run are on the full lane's existing examples job on macOS arm64, not Linux. The Linux runner only runs while its heartbeat is fresh, and the goldens must be checked on the platform that wrote them.
  - **No `fuzz/render_hayro` target:** the nice-to-have was left for X3b or a follow-up.
- **Goldens:** all 14 corpus files open in both backends (16 pages each), so `skip.json` is empty. The goldens regenerate byte for byte.

