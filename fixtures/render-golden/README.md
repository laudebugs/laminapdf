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

Both need Python with `tools/render/requirements.txt` (`$LAMINA_PYTHON`, else `python3`). The PNGs are lamina's own output rendered by the two engines; they carry the licence of the examples (MIT OR Apache-2.0) and of the OFL fonts drawn in them (`fixtures/fonts/OFL.txt`).
