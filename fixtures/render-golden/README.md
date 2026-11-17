# Render goldens

ADR 0018 tier 8, started by todo B5c for the `laminapdf` examples (the C round reuses the tooling). For every page of every example golden in `fixtures/golden/examples/` (the output of `crates/laminapdf/examples/`, kept equal to it by `cargo xtask examples` and `crates/laminapdf/tests/examples.rs`):

- `<example>/<page>-pdfium.png` and `<page>-hayro.png`: the page rendered at **72 dpi** by PDFium (pypdfium2, pinned in `tools/render/requirements.txt`) and by hayro (`tools/render`, its own workspace: hayro and its whole graph pinned by `tools/render/Cargo.lock`). `<page>` is the 0-based page index.
- At **150 dpi** nothing is committed: hayro and PDFium are compared with each other, with the hayro-parity spike's metric ([`spikes/hayro-parity/README.md`](../../spikes/hayro-parity/README.md), "Method").

`thresholds.toml` holds the limits of both checks and the known differences (one entry per page and check, with its reason; an unused entry fails). A render must match its golden exactly (no pixel over ΔE 2.3, SSIM ≥ 0.99), so a one-pixel-wide change fails; parity uses the spike's `pass` thresholds. A known difference's limits replace the check's defaults for that page.

Zero differing pixels holds because the renderers are deterministic for a given build and CPU architecture: both engines are pinned exactly (pypdfium2's wheel; hayro, `vello_cpu`, `fearless_simd`, `kurbo` and the rest by `tools/render/Cargo.lock`, built `--locked`; Dependabot covers only the root workspace, so nothing moves without a deliberate bump and `--update`). hayro picks its SIMD level at run time (`fearless_simd::Level::new`) and offers no setting to force one. On aarch64, where the goldens are made and the full lane runs (macOS ARM64), that level is always NEON. On another architecture (x86-64, a different SIMD path) anti-aliased edges may differ by a few pixels; compare there with a `[[known]]` entry or regenerate, never by loosening the global limit.

So that a change to an example shows up here, update its byte golden first (`cargo xtask golden --update`), then run the render goldens.

```sh
cargo xtask render-golden            # compare; diff images (expected | actual | diff) in target/xtask/render-golden/diff/
cargo xtask render-golden --update   # rewrite the PNGs; review them like code (still fails if hayro and PDFium disagree)
```

Both need Python with `tools/render/requirements.txt` (`$LAMINA_PDF_PYTHON`, else `python3`). The PNGs are lamina's own output rendered by the two engines; they carry the licence of the examples (MIT OR Apache-2.0) and of the OFL fonts drawn in them (`fixtures/fonts/OFL.txt`).

## Annotation appearance parity (`annots-parity/`)

The annotation appearances Lamina PDF writes, drawn by three viewers' engines (the E4 plan's [§10](../../docs/plans/editor/e4-markup-and-drawing-annotations.md#10-tests)). `tools/render`'s `annots-parity` makes one PDF through the façade, as an app would: 16 square pages, the four property sets of `tools/render/src/annots_matrix.rs` (RGB red, opaque, solid; RGB blue, 50 %, dashed; CMYK green, opaque, cloudy; gray, 50 %, 4 pt) each on pages turned 0°, 90°, 180° and 270°. Every page holds one of each: highlight, underline, strike-out, squiggly, note, caret, ink, square, circle, line with endings, polygon, polyline, text box, callout, typewriter text and a dimension line with leader lines. The note's icon changes from page to page through all seven, and every other page's caret has its paragraph symbol. The tool writes the file twice and fails if the bytes differ.

- `annots-parity/<page>-<engine>.png`: the page at **72 dpi** by PDFium (`pdfium.py`), hayro and pdf.js (`tools/render/pdfjs-render.mjs`: pdfjs-dist's legacy build on `@napi-rs/canvas`, appearance streams drawn, no form layer). Each must match its golden exactly, as above.
- At **150 dpi** nothing is committed: hayro and pdf.js are each compared with PDFium with the hayro-parity spike's metric.

`annots-parity.toml` holds the limits per engine (`[golden.<engine>]`, `[parity.<engine>]`) and what they were measured from. The parity defaults sit just above the upright pages' worst, so a mark missing from one engine fails; each turned page that needs more has a known difference in `thresholds.toml`'s form (`example = "annots-matrix"`), with its measured value and reason. On a turned page PDFium keeps the note's icon upright, as its NoRotate flag asks, while hayro and pdf.js turn it. pdf.js and its canvas are pinned by `tools/render/package.json` and `pnpm-lock.yaml`, their own pnpm project (pdfjs-dist: Apache-2.0; @napi-rs/canvas: MIT, with Skia's BSD-3-Clause), installed by xtask when missing; like the other engines they are test oracles, never part of the product.

```sh
cargo xtask annots-parity            # compare; diff images in target/xtask/annots-parity/diff/
cargo xtask annots-parity --update   # rewrite the PNGs; review them like code (still fails if an engine disagrees with PDFium)
cargo xtask annots-parity --corpus   # also the corpus edit test (after `cargo xtask corpus fetch --source pdfjs`)
```

The corpus edit test moves every annotation of the pdf.js corpus's `annotation-*.pdf` files one unit right and one up through the façade and saves each file incrementally. Lamina PDF must then reopen it with each moved annotation, and its Popup, exactly that far on in user space and every other annotation where it was, and `qpdf --check` must pass. PDFium, pdf.js and hayro each draw the original and the saved file at 72 dpi; they must give the same pages, of the same sizes, within `annots-parity.toml`'s `[corpus]` limit, so a saved file that draws blank or broken fails. A file Lamina PDF may not save incrementally as it is (signed, encrypted or repaired), or with nothing it can move (only widgets or locked annotations), is skipped. Every file's outcome, moved or the reason it was skipped, is pinned in `annots-parity-corpus.toml`: a different outcome fails, and so does a corpus file the list lacks or a listed file the corpus lacks. A file Lamina PDF can't open or save fails. Nothing from the corpus is committed. The nightly lane runs `cargo xtask annots-parity --corpus` on macOS arm64, where the goldens are made, and uploads the diff images when it fails. `cargo xtask render-golden` leaves this folder to it.
