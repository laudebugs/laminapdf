# C1b — `pdfium-wasm` spike: PDFium-WASM next to `lamina-wasm` in one worker

| Field | Value |
| --- | --- |
| ID | `C1b` |
| Status | `ongoing` |
| Plan | [web reader §6 C1. Rendering spikes](../../plans/web-app.md#c1-rendering-spikes) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B3c](../done/B3c-save-planner-and-roundtrip.md), [B6b](../done/B6b-lamina-pdf-package.md), [C1a](../done/C1a-hayro-parity-spike.md) |
| Blocks | [C2a](../ready/C2a-render-adrs.md) |
| Dependency notes | Library **B6** (`lamina-wasm` + `@lamina/pdf`; [pdf-library §7 B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf)) — and through it B3 (`lamina-save` incremental save); [C1a](../done/C1a-hayro-parity-spike.md) (same machine and corpus; the joint recommendation builds on its numbers) |
| ADRs | Informs ADR 0004 (render backend) and ADR 0009 (worker topology, single- vs multi-threaded WASM). Must follow [0001](../../decisions/0001-core-language-and-licence.md), [0002](../../decisions/0002-binding-strategy.md), [0005](../../decisions/0005-document-model.md) §2–3 (`ByteSource`, overlay, incremental save), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `spike/c1b-pdfium-wasm` |
| PR title | `spike(pdfium-wasm): measure pdfium-wasm beside lamina-wasm in one web worker` |
| Size | M (200–600 lines of spike TS + README) |
| PR | — |

## Goal

Prove, in a real browser worker, that PDFium-WASM and `lamina-wasm` can share one Web Worker, and measure what that costs: download size (raw/gzip/brotli), instantiate time, peak memory, render time per page, and — the [two-parsers](../../research/architecture/app-and-package-architecture.md#pdfium-ships-now-hayro-is-the-destination-and-a-trait-decides) question — how long PDFium takes to reload a document after `lamina-save` appends an incremental update. Together with [C1a](../done/C1a-hayro-parity-spike.md) the README recommends the defaults ADR 0004 records (which PDFium build, how it's loaded, single- vs multi-threaded, reload strategy) and gives ADR 0009 its worker-topology facts.

## Context

- [UI/UX plan §5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7) and [product research: engines](../../research/product/pdf-app-product-and-ux.md#five-conclusions-from-the-comparison) — some PDFium WASM builds have a fixed-size heap and hit an unrecoverable OOM on long documents (pdfium-render's warning): pick or build a **growable-heap** build, run one PDFium instance per worker (PDFium isn't thread-safe), and record upstream PDFium's licence and third-party notices.
- [web-app.md §4 non-functional targets](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) — the budgets this spike checks feasibility against: app shell ≤ 200 KB, first page ≤ 700 ms warm / ≤ 3 s cold Fast 4G, ≤ 600 MB for 1,000 pages.
- [Research report: rendering](../../research/architecture/app-and-package-architecture.md#pdfium-ships-now-hayro-is-the-destination-and-a-trait-decides) — `pdfium.wasm` 4,632,999 bytes raw, ~2.7 MB gz; reload via `ByteSource` must be "measured early".
- [Research report: web client](../../research/architecture/app-and-package-architecture.md#the-web-client-is-react-chrome-around-an-imperative-viewer-core) and [web-frontend notes](../../research/architecture/notes/web-frontend.md) §2 (rasterise in WASM memory, transfer `ImageBitmap`), §5 (typed RPC, COOP/COEP, threaded variant optional), §6 (OPFS, avoid double copies).
- [Rendering notes](../../research/architecture/notes/rendering.md) §1 (EmbedPDF `@embedpdf/pdfium` 2.15.1, MIT wrapper over Apache-2.0 PDFium) and §4 (no published memory numbers for 1,000+ pages).
- [ADR 0005 §2–3](../../decisions/0005-document-model.md#2-bytesource-and-ownership-of-the-input) — `ByteSource`, base revision + overlay, incremental save through `WriteSink`.
- [pdf-library B6](../../plans/pdf-library.md#b6-wasm-and-laminapdf) — the `@lamina/pdf` API (`init()`, `Document`) this spike calls.
- [C1a result](../done/C1a-hayro-parity-spike.md) — machine and corpus to reuse.

## Scope

**In scope**

- `spikes/pdfium-wasm/`: a standalone Vite (or plain esbuild) page + module worker, its own `package.json` and lockfile, outside the pnpm workspace.
- Candidate PDFium builds: `@embedpdf/pdfium` (prebuilt, MIT wrapper) and, if time allows, a self-built pinned PDFium WASM (emscripten) to compare size; single-threaded only, plus a note on what a threaded build would need (COOP/COEP).
- `lamina-wasm` / `@lamina/pdf` loaded in the same worker from the local B6 build (path or `pnpm pack` tarball; packages are unpublished).
- Measurements on 3 reference files: the 10-page ~1 MB text PDF from the §4 targets, a 1,000-page text PDF (generated with `@lamina/pdf`'s builder), and an image-heavy scan (~100 MB) — plus the C1a corpus sample for render time.
- Reload after incremental save: open in both engines → apply a proving op (`SetInfo` or `RotatePage` from B3) in lamina → incremental save via `WriteSink` → hand the new bytes to PDFium → re-render the touched page; compare full reload vs any partial path PDFium offers; measure with 1 MB, 50 MB and 500 MB inputs (or as large as memory allows).
- Browsers: Chromium, Firefox and WebKit via Playwright, on the C1a machine.

**Out of scope** (and which todo covers it)

- hayro fidelity → [C1a](../done/C1a-hayro-parity-spike.md).
- Product RPC, virtualiser, canvas pool → [C3a](../ready/C3a-viewer-worker-rpc.md), [C3b](../ready/C3b-viewer-virtualiser-and-zoom.md).
- Writing ADRs 0004/0009 → [C2a](../ready/C2a-render-adrs.md).
- The threaded (SharedArrayBuffer) build in production; only noted as an option here.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree `spike-c1b-pdfium-wasm` off the updated `main`.
- [ ] Check licences: `@embedpdf/pdfium` (MIT), PDFium (Apache-2.0 + third-party: FreeType FTL/BSD, libjpeg-turbo, OpenJPEG BSD-2, lcms2 MIT, zlib, ICU). List them in the README; confirm nothing GPL/AGPL is linked.
- [ ] Build B6 locally (`cargo xtask wasm`, `pnpm --filter @lamina/pdf build`) and depend on it from the spike by path or `pnpm pack` tarball.
- [ ] Create `spikes/pdfium-wasm/` with `package.json` (private), `index.html`, `src/main.ts` (file input, buttons, result table), `src/worker.ts` (loads both WASM modules), `src/bench.ts`, `scripts/make-fixtures.ts` (generates the 1,000-page PDF with `@lamina/pdf`), `playwright.config.ts`, `tests/bench.spec.ts`, `.gitignore` (large inputs, `out/`).
- [ ] Worker: instantiate both modules with `WebAssembly.instantiateStreaming` in parallel; report instantiate time for each and combined; check for symbol or memory conflicts (two separate `WebAssembly.Memory` instances; record both sizes).
- [ ] Input path: from a `File`, avoid double copies — compare (a) `arrayBuffer()` then copy into each module, (b) PDFium custom file access (`FPDF_LoadCustomDocument` with a `getBlock` callback over `File.slice()`), (c) OPFS `createSyncAccessHandle` read by both. Record memory for each.
- [ ] Render: page → RGBA buffer in PDFium memory → `ImageData` → `createImageBitmap` → transfer to main thread → `drawImage`. Time each step at 1× and 2× DPR for page 1 and a mid page.
- [ ] Measure: `.wasm` raw/gzip/brotli bytes for each module, instantiate ms, first-page-visible ms (from file selection), per-page render ms (median/p95 over the C1a corpus sample and the 1,000-page file), `performance.measureUserAgentSpecificMemory()` (Chromium; cross-origin isolated only — run that variant with COOP/COEP headers) and WASM `memory.buffer.byteLength` peaks for all browsers.
- [ ] Reload after incremental save: implement the open → op → `save({ mode: "incremental" })` → reload-in-PDFium cycle; time the save, the copy, `FPDF_CloseDocument` + reload, and re-render; repeat with large inputs; record when the 4 GB wasm32 ceiling bites.
- [ ] Encrypted file: open `fixtures/corpus/enc.pdf` (and B2's encrypted fixtures) in both engines with the password; confirm both decrypt and that the password never leaves the worker.
- [ ] Note what a threaded PDFium/lamina build would need (COOP/COEP, `crossOriginIsolated`), without building it.
- [ ] Run everything in Chromium, Firefox and WebKit through Playwright on the C1a machine; save `out/results.json`.
- [ ] Write `spikes/pdfium-wasm/README.md`: question, setup, reproduce steps, results tables per browser, the reload numbers, memory per strategy, licences, limitations, and a **joint recommendation for ADR 0004 and ADR 0009** with C1a: which PDFium build to pin (and how to hash it), lazy-load strategy (start fetching on app load, in parallel with file selection), input strategy (`ByteSource` over `File.slice()` vs OPFS), reload strategy after edits, single-threaded baseline vs threaded variant, and whether the §4 budgets look achievable.
- [ ] Add the spike to `CLAUDE.md` (layout line) and mark `pdfium-wasm` done in [architecture §4](../../plans/architecture.md#4-spikes-to-close-open-questions).

## Acceptance criteria

- [ ] One worker loads both PDFium-WASM and `lamina-wasm` and renders page 1 of all three reference files in Chromium, Firefox and WebKit (Playwright run committed as `tests/bench.spec.ts`, passing).
- [ ] README reports, on the same machine as C1a: bundle sizes (raw/gz/br) for each module, instantiate time, first-page-visible time for the 10-page 1 MB file (target context: ≤ 700 ms warm), per-page render median/p95, peak memory for the 1,000-page file scrolled through, and the reload-after-incremental-save timings for at least three input sizes.
- [ ] README compares at least two input strategies by peak memory.
- [ ] README states licences of every shipped component and confirms no GPL/AGPL code.
- [ ] README ends with the joint recommendation (with C1a) that C2a records in ADRs 0004 and 0009.
- [ ] Nothing large is committed (generated 1,000-page file, scan, `.wasm` builds); the spike is outside the pnpm workspace and CI is unaffected.

## Post-merge checks

None.

## Tests

Only the Playwright benchmark spec in the spike (run by hand, not in CI). Smoke: `fixtures/corpus/hello.pdf` renders and `enc.pdf` opens with its password in all three browsers.

## Docs to update

- `spikes/pdfium-wasm/README.md`.
- `CLAUDE.md` layout tree / spike commands.
- [architecture.md §4](../../plans/architecture.md#4-spikes-to-close-open-questions) (`pdfium-wasm` done) and tick the C1 to-dos in [web-app.md §9](../../plans/web-app.md#9-to-dos) ("Build `spikes/pdfium-wasm`", "Measure bundle size…", "Measure reload cost…", "Write both results…").

## Risks and open questions

- **Memory measurement is uneven across browsers** (`measureUserAgentSpecificMemory` is Chromium-only and needs cross-origin isolation); fall back to WASM memory sizes and report the method per browser.
- **EmbedPDF's build may lag PDFium security fixes**; the recommendation must say how we pin, hash and update it (ADR 0004 / 0019).
- **Reload of huge files** may exceed wasm32 memory when both engines hold a copy; this is the strongest finding for ADR 0004 (and for hayro reading lamina's graph later) — report it honestly even if it fails.
- **Custom file access** across two engines may need two independent readers over the same `File`; check Safari's `File.slice()` performance.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
