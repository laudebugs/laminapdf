# C2c — `@lamina/render-pdfium` and `@lamina/render-hayro`: the web render adapters

| Field | Value |
| --- | --- |
| ID | `C2c` |
| Status | `done` |
| Plan | [web reader §6 C2. Render backend and ADRs](../../plans/web-app.md#c2-render-backend-and-adrs) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B6b](./B6b-lamina-pdf-package.md), [C2b](./C2b-lamina-render-crate.md) |
| Blocks | [C3a](../ready/C3a-viewer-worker-rpc.md), [D1a](../ready/D1a-desktop-ipc-spike.md), [D1b](../ready/D1b-desktop-print-spike.md) |
| Dependency notes | [C2b](../done/C2b-lamina-render-crate.md) (trait + hayro adapter to compile); library **B6** (`lamina-wasm` build pipeline, ADR 0016 package conventions, size-budget checks; [pdf-library B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf)) |
| ADRs | Implements 0004 (web side) and follows 0009 (worker-only use); follows [0002](../../decisions/0002-binding-strategy.md) (wasm-bindgen + tsify), 0016 (package design, from B6), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (pinned, hashed third-party binary) |
| Branch | `feat/c2c-render-packages` |
| PR title | `feat(render-pdfium): add the pdfium-wasm and hayro web render adapters` |
| Size | L (600–1,500 lines; vendored `.wasm` excluded) |
| PR | [#119](https://github.com/laudebugs/lamina/pull/119) |

## Goal

Give the browser the same `RenderBackend` contract the Rust crate has. `@lamina/render-pdfium` ships the pinned PDFium-WASM build behind a small TypeScript adapter; `@lamina/render-hayro` ships `lamina-render`'s hayro adapter compiled to its own `.wasm`. Both are optional packages, loaded only inside the document worker, so `@lamina/pdf` stays small ([pdf-library B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf) budgets) and the viewer can switch backends at runtime (debug menu, diffing).

## Context

- ADR 0004 (from [C2a](../done/C2a-render-adrs.md)) — which PDFium build, its hash and update policy; trait surface.
- [C1b result](../done/C1b-pdfium-wasm-spike.md) (`spikes/pdfium-wasm/README.md`) — loading strategy, input strategy (`File.slice()` custom access vs OPFS), measured sizes.
- [TypeScript guidelines](../../guidelines/typescript.md) — ESM-only, `exports` with `types` first, `sideEffects: false`, named exports, no Node APIs, Vitest; [modules and packaging](../../guidelines/typescript.md#modules-and-packaging).
- [web-app.md §5](../../plans/web-app.md#5-design-summary) — packages list and optionality.
- B6's `packages/pdf` and `crates/lamina-wasm` — copy their build, `init()` and size-budget patterns.

## Scope

**In scope**

- `packages/render` (`@lamina/render`, a tiny types-and-helpers package): the TS `RenderBackend` interface mirroring the Rust trait (`open`, `pageCount`, `pageSize`, `render(req, signal) → { width, height, pixels: Uint8ClampedArray }`, `close`), error codes shared through `fixtures/codes.json`.
- `packages/render-pdfium`: vendored or fetched PDFium-WASM (`pdfium.wasm` + glue), verified by SHA-256 at build time, adapter implementing the interface, custom file access over a `ByteSource`-like `read(offset, length)` callback, password support.
- `crates/lamina-render-wasm` (edge) + `packages/render-hayro`: wasm-bindgen + tsify build of the hayro adapter, `init()` like `@lamina/pdf`, built by `cargo xtask wasm`.
- Size budgets for both `.wasm` files in CI; licence notices shipped in each package.

**Out of scope** (and which todo covers it)

- Worker RPC, scheduling, tiles → [C3a](../ready/C3a-viewer-worker-rpc.md), [C3b](../ready/C3b-viewer-virtualiser-and-zoom.md).
- Debug-menu UI to switch backends → [C4b](../ready/C4b-web-app-shell.md).
- Publishing to npm (packages stay local; [architecture: Publishing (later)](../../plans/architecture.md#publishing-later)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`. Load `/rust-best-practices` for the Rust part.
- [x] Create the render interface package (`packages/render`, `@lamina/render`, `private: true`): `RenderBackend`, `RenderRequest`, `RenderFlags`, `RenderedPixels`, `RenderErrorCode`; doc comments; unit tests on helpers (request validation, pixel-cap).
- [x] `packages/render-pdfium`:
  - [x] `scripts/fetch-pdfium-wasm.ts` (or an xtask subcommand) downloads the pinned build named in ADR 0004 and checks SHA-256; the build fails on mismatch. Decide vendoring vs fetch-at-build (record in README).
  - [x] `src/pdfium.ts`: load glue + `.wasm` from a URL passed by the caller (`init({ wasmUrl })`) so the app controls caching and lazy loading; no top-level side effects.
  - [x] `src/adapter.ts`: `openDocument(source, { password })` using `FPDF_LoadCustomDocument` with a block reader over the source (no full copy for `File` inputs), `render()` into a PDFium bitmap → copy/view into `Uint8ClampedArray` → caller wraps as `ImageData`; free bitmaps deterministically; map errors (`FPDF_ERR_PASSWORD` → `wrong-password`, etc.).
  - [x] Honour `AbortSignal` between steps (PDFium render is synchronous; check before and after, and support progressive render if the build exposes `FPDF_RenderPageBitmap_Start`).
  - [x] `LICENSES/` and `THIRD_PARTY_NOTICES.md` for PDFium and its bundled libraries.
- [x] `crates/lamina-render-wasm` + `packages/render-hayro`: wasm-bindgen exports `open(bytes, password)`, `pageCount`, `pageSize`, `render(req) -> Uint8ClampedArray`; tsify types; TS wrapper implements the same interface; built with the `wasm-release` profile and `wasm-opt -Oz` via `cargo xtask wasm`.
- [x] Size budgets: add both `.wasm` gz sizes to the B6 size-budget check (PDFium budget = measured C1b size + 5 %; hayro budget per ADR 0016 measurements).
- [x] Tests (Vitest browser mode with Playwright, Chromium/Firefox/WebKit): render page 1 of `fixtures/corpus/hello.pdf` and `imaging-demo.pdf` with each adapter and compare against C2b's PNG goldens with the same perceptual threshold; wrong password → `wrong-password`; aborted request → rejects with `AbortError` and frees memory; 50 sequential renders do not grow WASM memory beyond a fixed bound.
- [x] `cargo xtask check ts` runs the new packages' typecheck, Biome and tests; npm licence check passes.
- [x] READMEs for the three packages (`render`, `render-pdfium`, `render-hayro`); update `CLAUDE.md` layout (`packages/render*`, `crates/lamina-render-wasm`).

## Acceptance criteria

- [x] Both adapters implement the same `RenderBackend` interface (one shared contract test suite runs against both).
- [x] Browser tests pass in Chromium, Firefox and WebKit; rendered pixels match the C2b goldens within threshold.
- [x] The PDFium build's SHA-256 is verified on every build; a mismatched file fails the build (tested by a unit test of the verifier).
- [x] `.wasm` size budgets enforced in CI; `@lamina/pdf`'s own budget is unchanged (neither adapter is a dependency of it).
- [x] No Node-only APIs or DOM globals beyond what a worker has (`self`, `fetch`, `WebAssembly`), checked by the portability test.
- [x] Third-party notices present for PDFium.

## Post-merge checks

None.

## Tests

Contract tests in `packages/render/test/contract.ts` reused by `packages/render-pdfium/test/` and `packages/render-hayro/test/` (Vitest browser mode); verifier unit test; memory-bound test; shared fixtures from `fixtures/corpus` and goldens from `fixtures/golden/render/`.

## Docs to update

Package READMEs; `CLAUDE.md`; tick "Create `packages/render-pdfium`… and `packages/render-hayro`" in [web-app.md §9](../../plans/web-app.md#9-to-dos) and the `@lamina/render-*` line in [architecture §7](../../plans/architecture.md#7-to-dos).

## Risks and open questions

- **Vendoring a 4.6 MB binary in git** vs fetching at build: fetching keeps the repo small but adds a network dependency to CI (cache it); vendoring is simpler and reproducible. Decide and record in ADR 0004 if C2a didn't.
- **Emscripten glue** may assume `window` or use `eval`; it must run in a module worker under CSP `script-src 'self' 'wasm-unsafe-eval'` (verify, patch or choose another build).
- **Premultiplied alpha** mismatch between PDFium output and `ImageData` (straight alpha) causes fringe artefacts; test on a transparent fixture.
- Interface package naming (`@lamina/render` vs a viewer subpath) — decide in review.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome

- **[`packages/render`](../../../packages/render/README.md) (`@lamina/render`):**
  - the TS `RenderBackend` / `RenderDocument` interface;
  - `RenderSource` (`readSync`, `undefined` meaning `data-pending`);
  - `planTile` and `pageToTile`, ports of the Rust planner, so all four adapters agree on sizes;
  - `RenderError` with a structural `isRenderError`;
  - `test/contract.ts`, which both adapters run in Node and in Chromium, Firefox and WebKit.
- **[`packages/render-pdfium`](../../../packages/render-pdfium/README.md):**
  - **The build:** `@embedpdf/pdfium` 2.15.1, pinned and fetched through npm, not vendored. Its `pdfium.wasm` SHA-256 is checked before every build, typecheck and test, and the verifier is unit-tested.
  - **Rendering:** custom file access over the source, a form-fill environment per document, and the native adapter's two render paths.
  - **Notices:** `THIRD_PARTY_NOTICES.md`, from EmbedPDF's build in `embedpdf/runtime` (no XFA, V8 or Skia).
- **`crates/lamina-render-wasm` and [`packages/render-hayro`](../../../packages/render-hayro/README.md):** a flat wasm-bindgen surface over `HayroBackend`, built by `cargo xtask wasm` as `lamina_render_hayro` into `@lamina/wasm`.
- **Results:** both adapters pass the contract in Node and in all three browsers, including C2b's goldens: PDFium-WASM at SSIM ≥ 0.98 against the native `chromium/7881` goldens, hayro-WASM at ≥ 0.99.
- **Sizes and budgets:**

  | Module | gzip | Budget |
  | --- | ---: | ---: |
  | `pdfium.wasm` | 2,158,033 B | 2,243,210 B (C1b + 5 %) |
  | `lamina_render_hayro.wasm` | 1,765,396 B | 1,855,000 B |

  `@lamina/pdf`'s budget is unchanged.
- **CI:**
  - `cargo xtask wasm` runs the hayro Node contract.
  - `cargo xtask pdf-runtimes chromium|firefox|webkit` runs both browser contracts in the existing browser jobs, with no workflow change, because the `wasm` artifact already carries `packages/wasm/dist/`.
  - `cargo xtask size` checks both budgets.
- **Deviations:**
  - **Progressive rendering:** exported by the PDFium build, but not used yet; the signal is checked before and after.
  - **Packaging:** the contract types live in `@lamina/render`, not a viewer subpath.
  - **Unrelated fix:** a Biome `noCommaOperator` error in `@lamina/pdf`'s `builder.ts` (from B8a) was fixed so `check ts` passes.

