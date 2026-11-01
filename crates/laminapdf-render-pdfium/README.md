# laminapdf-render-pdfium

The PDFium adapter of [`laminapdf-render`](../laminapdf-render/README.md) ([ADR 0004](../../docs/decisions/0004-render-backend.md) §2, §3, §5). It is an **edge** crate: FFI into a C++ library that is loaded at runtime.

- **The library:** `pdfium-render` 0.9.4 (`MIT OR Apache-2.0`), with features `pdfium_7881` and `thread_safe`. It binds `bblanchon/pdfium-binaries` `chromium/7881` **at runtime**, from the folder in `LAMINA_PDF_PDFIUM_DIR`, so building, clippy and the fast lane never need the library.
- **Fetching it:** `cargo xtask fetch-pdfium` downloads the build for this host and checks its SHA-256 (pinned in `tools/xtask/src/pdfium.rs`). It unpacks the library, its notices (`licenses/`, which every bundle must ship, ADR 0004 §3) and `VERSION`, then prints `LAMINA_PDF_PDFIUM_DIR=…`.
  - **Pinned builds:** macOS arm64 and x64, Linux x64 and arm64, Windows x64.
  - **Cache:** `$LAMINA_PDF_PDFIUM_CACHE`, else `$RUNNER_TOOL_CACHE/lamina-pdfium` on CI, else `target/pdfium/`.
- **Input:** PDFium reads Lamina PDF's `ByteSource` through its custom file access, never a second copy. Reads the source answers `Pending` for are remembered, and the call fails with `data-pending` and those ranges. A render that hit one is discarded, because PDFium can't report a failed read mid-page.
- **Threads:** one process-wide lock covers every `open`, `page` and `render`, on top of `pdfium-render`'s per-call lock, because PDFium's state is global. Two documents on two threads render one after the other.
- **Tiles:** only the tile lands in a tile-sized bitmap that wraps the caller's buffer (`PdfBitmap::from_bytes`). There's no copy and no full-page allocation. `FPDF_REVERSE_BYTE_ORDER` gives RGBA.
  - **With widgets** (`FPDF_RenderPageBitmap` then `FPDF_FFLDraw`), the page is laid out at its whole pixel size and shifted by the tile's offset.
  - **Without widgets,** pdfium-render switches to `FPDF_RenderPageBitmapWithMatrix`, which ignores that offset, so it gets `TilePlan::page_to_tile` instead: the same matrix hayro draws with.
- **Every engine call is under the lock,** closing a document included (`Drop`). Each call loads its page once, and a read the source refused during any call, even one PDFium then recovered from with defaults, makes it `data-pending`.
- **Cancellation:** checked before and after the call. `pdfium-render` doesn't expose PDFium's progressive rendering (`FPDF_RenderPageBitmap_Start`), so a running render finishes.

```rust
let backend = laminapdf_render_pdfium::PdfiumBackend::from_env()?; // or ::bind(dir)
let doc = backend.open(Arc::new(bytes), &RenderOpenOptions::new().with_password(pw))?;
```

## Tests and goldens

Every test needs the library, so all are `#[ignore]`d. `cargo xtask render-golden` (the full lane's examples job, macOS arm64) runs them after `fetch-pdfium`:

- **`tests/pdfium.rs`:**
  - **Errors:** out-of-range pages, non-PDF bytes, password required and wrong, an oversized request, cancellation, and `data-pending` from a source that has only the head and tail of the file.
  - **Geometry:** tiles stitch into the whole page, and a 90° view equals the upright render rotated.
  - **Page sizes:** PDFium, hayro and Lamina PDF's `page_info` agree on the size and `/Rotate` of every corpus page.
- **`tests/golden.rs`:** every page of every `fixtures/corpus/` file, rendered by each backend at 96 DPI on white, against `fixtures/golden/render/<backend>/<stem>-p<N>.png` (SSIM on luma ≥ 0.99 against its own backend's golden). Files a backend can't render go in `fixtures/golden/render/skip.json` with a reason; there are none today. Cross-backend parity is X3b's nightly diff.
- **`tests/determinism.rs`:** the same request gives the same bytes in both backends.

`cargo xtask render-golden --update` rewrites the goldens with `examples/goldens.rs`; running it twice gives no diff. The goldens are written on macOS arm64: PDFium and FreeType can differ by a few pixels on other platforms, which is why the fast lane never checks them.

Locally: `cargo xtask fetch-pdfium`, then `LAMINA_PDF_PDFIUM_DIR=… cargo test -p laminapdf-render-pdfium -- --ignored`.
