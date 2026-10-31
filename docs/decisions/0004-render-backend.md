# 0004 — Render backend: the `RenderBackend` trait, PDFium by default, hayro as the oracle

- Status: Accepted (2026-10-31, when C2a's PR merges)
- Date: 2026-10-31

## Context

lamina owns the document model (ADR 0005), editing (ADR 0006) and saving (ADR 0007), but not pixels. No permissive engine gives both pure-Rust safety and the long tail of real-world files on day one ([research: PDFium ships now, hayro is the destination](../research/architecture/app-and-package-architecture.md#pdfium-ships-now-hayro-is-the-destination-and-a-trait-decides); [rendering notes §1, §5](../research/architecture/notes/rendering.md)):

- **PDFium** (BSD-3 / Apache-2.0): the fidelity reference, the engine behind Chrome's viewer. It is reachable natively through `pdfium-render` 0.9.4 (`MIT OR Apache-2.0`) and in the browser as PDFium-WASM (`@embedpdf/pdfium`, MIT wrapper). It is C++, so its memory-safety surface has to be sandboxed.
- **hayro** 0.8.0 (`MIT OR Apache-2.0`): pure Rust, renderer only, version 0.x, one maintainer ecosystem.
- **pdf.js** (Apache-2.0): JavaScript, with its own document model.
- **MuPDF**: AGPL-3.0 or commercial, so excluded by [ADR 0001](0001-core-language-and-licence.md).

Two spikes measured the choice on one machine (Apple M1 Pro, 16 GB):

- **[C1a, `hayro-parity`](../../spikes/hayro-parity/README.md)** compared hayro with PDFium on 2,196 pages of the pdf.js and PDFBox corpora at 96 DPI:
  - **81.8 % pass and 90.8 % pass or differ slightly.** 153 pages fail, and there are 50 errors or timeouts.
  - **The failures cluster in a few areas:** fonts (embedded decoding, no fallback for non-embedded CJK, Cyrillic and symbol fonts), annotations without appearance streams, knockout and non-isolated groups, and optional-content expressions. CalRGB/CalGray colour differs a lot but has no verdict yet.
  - **Speed:** the median page takes 5.5 ms in both engines, but hayro's tail is heavier (16 s on one bomb file).
- **[C1b, `pdfium-wasm`](../../spikes/pdfium-wasm/README.md)** ran PDFium-WASM beside lamina in one worker, in Chromium, Firefox and WebKit:
  - **Download:** `pdfium.wasm` is 4,646,932 bytes raw, 2,136,390 gzip and 1,649,881 brotli; its JS wrapper is another 62,663 gzip.
  - **First page:** both engines instantiate in 25–60 ms. Page 1 of a 1 MB file shows in 15–39 ms cold and 6–8 ms warm. Text pages render in 2–4 ms.
  - **Memory and input:** reading the file by ranges halves PDFium's heap. It is also how WebKit opens a 500 MB file.
  - **Reload after an edit:** reopening PDFium after an incremental save takes milliseconds; lamina's save is the cost.
- **hayro on WebAssembly** (measured for this ADR, same machine): hayro 0.8.0 was compiled to wasm32 with `opt-level = "z"`, LTO, `panic = "abort"`, stripped, and one exported render entry point. There was no wasm-opt and no bindings, so these are upper bounds for a `@lamina/render-hayro` module.

  | Build | Raw | gzip -9 | brotli -11 | gzip vs PDFium |
  | --- | ---: | ---: | ---: | ---: |
  | hayro default features (embedded standard 14 fonts and CMaps) | 4,450,417 | 1,678,932 | 1,307,364 | 79 % |
  | hayro without `embed-fonts` and `embed-cmaps` | 3,734,770 | 1,111,619 | 789,534 | 52 % |
  | `pdfium.wasm` (C1b) | 4,646,932 | 2,136,390 | 1,649,881 | 100 % |

  **hayro is not the tiny download the research assumed.** With its fonts and CMaps it is only about a fifth smaller than PDFium. Without them it is about half, and the fonts and CMaps (about 570 KB gzip) could then be fetched only for documents that need them.

The [UI/UX plan §5.6](../plans/ui-ux.md#56-perceived-performance-r6-r7) also needs tiles. iOS Safari caps one canvas at about 16.7 MP and all canvases at 224–384 MB, so no backend may allocate a full-page bitmap at high zoom.

## Decision

### 1. One trait, read-only, in `crates/lamina-render`

Every lamina surface renders through `RenderBackend`. A renderer only produces pixels:
- **Never written through:** every write goes through lamina's op log and save planner (ADR 0006, 0007).
- **No model data from the renderer:** text, glyph quads, the structure tree, annotation and form-field geometry, links and outlines come from lamina's model, never from the renderer.

So hit-testing, selection and accessibility don't depend on which backend is active.

```rust
/// A rendering engine: opens documents. Object-safe, so shells hold `Box<dyn RenderBackend>`.
pub trait RenderBackend: Send {
    /// `"pdfium"` or `"hayro"`; with `version()`, part of every golden's and diff report's key.
    fn name(&self) -> &'static str;
    fn version(&self) -> &str;
    fn open(&self, source: Arc<dyn ByteSource>, options: &RenderOpenOptions)
        -> Result<Box<dyn RenderDocument>, RenderError>;
}

/// One open document. `Send + !Sync`: one call at a time on the document. Owns everything
/// it needs (`'static`), so it can move to the shell's render thread.
pub trait RenderDocument: Send {
    fn page_count(&self) -> u32;
    /// Boxes and `/Rotate` in points, ADR 0027 (0-based `index`).
    fn page(&self, index: u32) -> Result<RenderPage, RenderError>;
    /// Renders `request` into `out` (exactly `request.byte_len()` bytes), checking `cancel`.
    fn render(&self, request: &RenderRequest, out: &mut [u8], cancel: &CancelToken)
        -> Result<RenderedTile, RenderError>;
}
```

- **`ByteSource`** is `lamina-cos`'s trait (ADR 0005 §2). The renderer reads the same source as lamina, never a second copy. A source that answers `Pending` makes `open`, `page` or `render` fail with `data-pending` and the ranges, as lamina's own reads do. The PDFium adapter maps PDFium's availability API (`FPDFAvail_*` with download hints) onto it, and C2b verifies that.
- **The web adapters' contract.** On the web, PDFium is a separate Emscripten module and can't read a Rust `ByteSource` inside `@lamina/pdf`'s memory. So the shared source is **one JS-level reader in the document worker**, which both engines read through (ADR 0009 §2):

  ```ts
  interface RenderSource {
    readonly size: number;
    /** Synchronous: PDFium's getBlock can't await. undefined = not available yet. */
    readSync(offset: number, length: number): Uint8Array | undefined;
  }
  ```

  - **Local files:** `FileReaderSync` over `File.slice()`, or an OPFS sync access handle.
  - **Remote files:** the worker's block cache, filled by `httpRangeSource`.
  - **Pending data:** `undefined` from `readSync` is the web form of `Pending`. The adapter asks `FPDFAvail_*` which ranges it needs, fails the call with `data-pending` and those ranges, and the worker fetches them and retries, as `doc.ensure` does for lamina.
  - **One reader, both engines:** lamina's `callbackSource` reads the same blocks asynchronously.
  - **Implementations:** `@lamina/render-pdfium` and `@lamina/render-hayro` both take a `RenderSource` (C2c).
- **`RenderOpenOptions`:**
  - the password. The backend uses it in `open` and keeps no copy. Reopening an encrypted document (after a save, §4.2, or after a crash) needs it again, so the **shell** keeps it: the document worker holds it in memory for the document's lifetime and drops it on close (ADR 0009 §3.8). It is never written anywhere;
  - `RenderLimits`.
- **`RenderLimits`:**
  - `max_render_pixels`, default 16,777,216 (4096 × 4096, the iOS per-canvas cap);
  - `max_render_side`, default 16,384 px.

  Above either, the request fails with `render-too-large` before any allocation.
- **`RenderRequest`:**
  - `page: u32`.
  - `scale: f32`: output pixels per point, with the device pixel ratio already applied, so 1.0 is 72 DPI.
  - `clip: Option<PixelRect>`: a tile, as `x`, `y`, `width` and `height` in the pixel space of the whole page at `scale`, with a top-left origin. `None` means the whole page, still bounded by the limits.
  - `rotation`: 0, 90, 180 or 270, a view rotation added to the page's `/Rotate`.
  - `flags`: `annotations` (default on: appearance streams, `FPDF_ANNOT`), `form_widgets` (default on, drawn read-only), `print` (print intent: honours `/Print` flags and hides `NoView`), `grayscale`.
  - `background: Option<Rgba8>`: `None` means transparent; the reader passes white.
- **Pixel format:** RGBA8, **straight (not premultiplied) alpha**, sRGB, rows packed at `width × 4` with no padding. That is what `ImageData` and `createImageBitmap` take, so the web path never converts. Adapters repack internally: PDFium uses `FPDF_REVERSE_BYTE_ORDER`, and hayro un-premultiplies.
- **`RenderedTile`:** the `width` and `height` written, and whether annotations were drawn.
- **Cancellation:** a `CancelToken` (ADR 0028), returning the `cancelled` code.
  - **Where the token can flip during a call:** natively, another thread can cancel it. PDFium then uses progressive rendering (`FPDF_RenderPageBitmap_Start` / `FPDF_RenderPage_Continue`) with a pause callback that polls the token. C2b checks that the native build exports it.
  - **Single-threaded WebAssembly:** a `cancel` message can't run while the render does (ADR 0028 §5: cancellation lands between steps). The token can only stop work that hasn't started, so web renders are kept small (tiles: 2–15 ms per page in C1b) and cancellation drops queued tiles.
  - **A render already running on the web** finishes, and its result is discarded (ADR 0009 §3.5). Only terminating the worker stops it sooner.
  - **Exception:** C2c may make the web PDFium adapter yield to the event loop between `FPDF_RenderPage_Continue` steps, if the WASM build exports them and C3c shows long single tiles.
  - **hayro:** the token is checked before and after the call, because hayro can't be paused.
- **Determinism:** the same backend version, input and request give the same bytes. Goldens depend on it, and C2b tests it.
- **Errors:** `RenderError` maps to the ADR 0026 contract. It reuses `page-index-out-of-range`, `password-required`, `wrong-password`, `data-pending`, `cancelled` and `limit-exceeded`, and adds `render-too-large`, `render-failed` (the engine refused the page) and `render-backend-unavailable` (the library or module isn't loaded). C2b adds these to `fixtures/codes.json`.
- **Thread safety is the backend's job, across documents.** PDFium is not thread-safe at the library level: its global state is shared by every document in a process. So the PDFium adapter serialises **every** call of every document it opened through one backend-wide lock (pdfium-render's `thread_safe` feature, as C1a used). Two documents on two threads then render one after the other, never at once. hayro's documents are independent and take no shared lock. Shells that want parallel PDFium rendering use separate processes (desktop, ADR 0008) or separate workers (web, ADR 0009), each with its own PDFium instance.
- **Ownership:** a `RenderDocument` is `'static`. pdfium-render's `PdfDocument<'a>` borrows its `Pdfium` bindings, so the adapter binds PDFium once per process and holds the bindings in a process-lifetime value (a `OnceLock`), which every document borrows for `'static`. C2b confirms this compiles with pdfium-render 0.9.4, or records the self-owning wrapper it uses instead.
- **Coordinates:** page geometry follows ADR 0027. The renderer's page size must equal lamina's `page_info` for the same index, and C2b's goldens assert it.

### 2. Adapters and default per surface

| Surface | Default | Second adapter |
| --- | --- | --- |
| Web reader (`apps/web`) | **PDFium-WASM** in the document worker (`@lamina/render-pdfium`) | hayro (`@lamina/render-hayro`), selectable in the debug menu (`#debug`, a fragment, so it never reaches our server, ADR 0011 §3.6) |
| Desktop (`apps/desktop`, Round D) | **Native PDFium** through `pdfium-render`, in the engine process (ADR 0008) | hayro, in the debug menu |
| Mobile (later, ADR 0025/0029) | **Native PDFium**, the same pinned binaries | hayro |
| Library users and edge runtimes (`@lamina/pdf` on Cloudflare Workers, Deno, Node) | **None bundled.** `@lamina/render-hayro` is the recommended optional renderer: pure Rust, no Emscripten glue or C++ surface, and about 1.1 MB gzip with fonts and CMaps fetched separately (1.7 MB with them built in) against PDFium's 2.1 MB | `@lamina/render-pdfium` where its size fits (Workers paid plan) |
| CI | **Both**: the render goldens (C2b), and the nightly hayro-vs-PDFium diff over the pinned corpora with C1a's metric (X3b) | — |

PDFium is the default wherever lamina ships a viewer, because a reader that grows into an editor needs fidelity on day one, and C1a shows hayro is not there yet. hayro is wired in from day one as the oracle and the debug choice, so a switch is a configuration change.

- **pdf.js is not a backend.** It carries a second document model in JavaScript, which is exactly what the trait exists to avoid.
- **MuPDF is prohibited** (ADR 0001), including MuPDF.js and any wrapper around it.

### 3. The pinned PDFium builds

PDFium is a third-party binary under [ADR 0019](0019-supply-chain-and-licence-hygiene.md): pinned exactly, hash-checked and updated deliberately.

- **Web:** `@embedpdf/pdfium` **2.15.1**, pinned exactly, from the npm tarball (`sha512-qDH4gEkGSQT0iM+07Hpm3LyvoB7PEFbW3Qyk6+LPkEYLJYfHagkHM1OzYlDq2EDOhJRguykTkzdMOT3/w27x5Q==`).
  - Its `dist/pdfium.wasm` has SHA-256 `5e4cd023c3dad4a895b3571ca573d3fc51bac6de48360d95283134385b954eaa`. `@lamina/render-pdfium` checks that at build time, and any other value fails the build (C2c).
  - The package doesn't record which PDFium revision it carries. C2c records it from EmbedPDF's build repository in the package README, and it becomes part of every golden's key.
  - It is an Emscripten build with a growable heap, which C1b verified.
- **Native:** `bblanchon/pdfium-binaries`, the `chromium/7881` release that C1a used and `pdfium-render`'s `pdfium_latest` targets.
  - `cargo xtask fetch-pdfium` downloads it per host OS from a pinned URL and checks a SHA-256 per asset recorded in `tools/xtask` (C2b).
  - `pdfium-render` binds it dynamically at runtime, so builds and clippy never need the library.
- **Notices:** every product that ships a PDFium build ships its third-party notices too.
  - **Web build:** the wasm package ships none (C1b), so `@lamina/render-pdfium` bundles them: PDFium's BSD-3 and Apache-2.0, FreeType (under the FTL), zlib, OpenJPEG, libpng, Little-CMS 2 and libjpeg-turbo. C2c confirms the list against EmbedPDF's build script.
  - **Native build:** `chromium/7881` carries its notices in `pdfium/licenses/` (C1a): FreeType (FTL), libjpeg-turbo (IJG + BSD-3-Clause), OpenJPEG (BSD-2-Clause), lcms2 (MIT), zlib, libpng, libtiff, AGG 2.3, ICU (Unicode), Abseil (Apache-2.0), fast_float, simdutf and llvm-libc. The desktop and mobile bundles copy that folder whole (D-rounds), and `cargo xtask fetch-pdfium` keeps it next to the library.
  - **Re-pins:** a PDFium re-pin rechecks both lists.
- **Updates:**
  - A PDFium bump is its own PR that re-pins both builds where possible, re-records the hashes and regenerates the PDFium goldens. It runs the nightly diff before merging.
  - Security fixes from upstream PDFium (Chromium's security releases) are picked up within the next release lane after a package containing them is published.
  - If the wrapper lags a published security fix by more than 30 days, lamina builds PDFium-WASM itself from the same revision with the same flags. That is a C2c follow-up, recorded when it first happens.
  - Dependabot opens the PRs but never auto-merges them.

### 4. The two-parsers rule

lamina parses for the model, and PDFium parses for pixels. They are kept consistent by rule, not by sharing state:

1. **Overlays come from lamina's model and are never re-rendered by the backend until saved:** annotations being edited, form values, selections and highlights, and the page order (reorder, rotate, delete and insert are a view mapping from `LaminaId` pages to source pages).
2. **Content edits and applied redactions change bytes.** lamina saves incrementally, and the backend reopens the result: the original `File` plus the appended revision, read by ranges.
   - **PDFium's side:** C1b measured it in milliseconds even at 500 MB, under 3 ms in Chromium and Firefox, so no partial-reload API is needed.
   - **lamina's side:** the save is the cost. An incremental save reads the whole base to hash `/ID` (about 4 s and the whole file for 500 MB opened by ranges), and Chromium runs a first large whole-output save about 10× slower until V8 tiers up. Shells run big saves as stepped save jobs (ADR 0028), and the `/ID` follow-up in C1b's README is a prerequisite for making large files editable in the web reader.
3. **Geometry disagreements are bugs, tracked by tests.** The nightly differential compares lamina's quads with PDFium's character boxes (`cargo xtask quads`, B9c), and the renderer's page sizes with lamina's.

### 5. Packaging

- **`crates/lamina-render`** (layer `core`): the trait, the types, request validation and the hayro adapter behind feature `hayro`, off by default. It has no `pdfium` feature and never depends on the PDFium crate.
- **`crates/lamina-render-pdfium`** (layer `edge`): the PDFium adapter, always this separate crate. It depends on `lamina-render` and implements its trait, with dynamic loading and `unsafe` FFI under `// SAFETY:`. Shells that want PDFium depend on it directly, so there is no feature cycle.
- **npm:** `@lamina/render-pdfium` (the PDFium-WASM adapter) and `@lamina/render-hayro` (`lamina-render`'s hayro adapter as its own `.wasm`) are **optional packages**. `@lamina/pdf` never depends on either, so its size budgets (ADR 0016 §6) don't move. Each gets its own budget in `packages/wasm/size-budgets.json` when C2c creates it.
- **Worker only:** both web adapters run only inside a worker (ADR 0009). The PDFium adapter reads its input through PDFium's custom file access (`FPDF_LoadCustomDocument`) over the worker's `RenderSource` (§1), which is C1b's default strategy.

### 6. When hayro becomes the default (C1a's criteria)

The owner decides, on a PR that changes a surface's default. That PR must show all of the following on the then-current pinned corpus (pdf.js + PDFBox + lamina's `fixtures/`), measured by X3b's nightly diff with C1a's metric (3×3 blur, ΔE76 > 10 with a 3×3 neighbourhood match, an SSIM guard; classes `pass`, `minor`, `fail`):

1. **Pass rate:** pass or minor on **≥ 98 % of all pages at both 96 and 150 DPI**, with **zero hayro-only incidents** (errors, panics, timeouts). On 2026-10-28 it was 90.8 % with 7 incidents.
2. **Empty buckets:** fonts (embedded decoding, plus non-embedded fallback with shipped Noto CJK, Cyrillic and symbol fonts), annotations without appearances and annotation flags, and optional content. Pages where hayro is verified correct are excluded.
3. **Colour:** CalRGB/CalGray is not a gate until a reference rendering settles which engine is right. Then the wrong engine's cases become a bucket that must be empty.
4. **Speed and memory:** median and p95 page time within **1.5× of PDFium** in a paired run on the CI runner. Every pathological page must also stay bounded by `Limits` and the renderer's cancellation.
5. **For the web surface:** the size and memory measured in the worker must favour hayro, since a smaller download is the reason to switch there.

The criteria apply per surface: desktop could switch before web, or the reverse. Until then, X3b's nightly number is tracked, not gated: a regression opens an issue.

## Consequences

- **One render path, two engines:** shells, goldens and the diff all go through one trait, so switching or A/B-testing a backend is configuration.
- **The web reader pays about 2.1 MB gzip for PDFium**, the main cold-start cost. ADR 0009 starts the download with the app shell to keep the 3 s Fast 4G target.
- **Bounded output:** tiles and `max_render_pixels` make full-page canvases impossible by construction, as the UI/UX plan requires.
- **A C++ engine parses untrusted input.** That is acceptable only because it runs in a worker (web) or a separate engine process (desktop, ADR 0008), with `Limits` on lamina's side.
- **Two parsers can disagree.** The rule in §4 and the nightly quads and size checks turn disagreement into tracked bugs instead of silent drift.
- **Library users keep a pure-Rust option:** `@lamina/pdf` stays small, and `@lamina/render-hayro` is the small optional renderer.
- **PDFium updates are operational work:** pins, hashes and goldens move together, in their own PR.

## Alternatives considered

- **hayro first (default everywhere now).** Pure Rust, memory-safe, and the long-term destination; on the web about half PDFium's gzip size without its built-in fonts and CMaps, and four-fifths of it with them. Rejected for now: 90.8 % pass or minor and clustered font and annotation failures (C1a) are not enough for a reader that grows into an editor. §6 says when to revisit.
- **pdf.js as the web renderer.** Best text layer and release cadence. Rejected: a second, JavaScript document model would drift from lamina's, and its fidelity on complex fonts lags PDFium's (rendering notes §1).
- **MuPDF / MuPDF.js.** High fidelity. Prohibited: AGPL-3.0 or a commercial licence (ADR 0001).
- **Building PDFium-WASM ourselves from day one.** Full control of flags and revision. Rejected for v0.1 because of the build complexity, kept as the fallback in §3 if the wrapper lags security fixes.
- **`pdfium-render` on WASM** (binding a separately loaded PDFium module). One Rust adapter on every surface. Rejected for the web: it still needs the same Emscripten module plus a second glue layer. A thin TS adapter over `@embedpdf/pdfium` is smaller, and C1b measured it.
- **The renderer as the source of text and geometry** (PDFium's text API). Less work in C5. Rejected: accessibility and selection would depend on the backend, and switching to hayro would lose them.
- **Premultiplied RGBA or BGRA output.** Native to the engines. Rejected: the browser path would then convert every pixel in JS, while the adapters can convert while copying.
- **Apple PDFKit / platform renderers.** Proprietary and platform-locked (rendering notes §1). They might become an optional adapter later, but never a default.
