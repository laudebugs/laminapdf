# Render goldens

ADR 0018 tier 8, started by todo B5c for the `lamina` examples (the C round reuses the tooling). For every page of every example in `crates/lamina/examples/`:

- `<example>/<page>-pdfium.png` and `<page>-hayro.png`: the page rendered at **72 dpi** by PDFium (pypdfium2, pinned in `tools/render/requirements.txt`) and by hayro (pinned in `tools/render/Cargo.toml`). `<page>` is the 0-based page index.
- At **150 dpi** nothing is committed: hayro and PDFium are compared with each other, with the hayro-parity spike's metric ([`spikes/hayro-parity/README.md`](../../spikes/hayro-parity/README.md), "Method").

`thresholds.toml` holds the limits of both checks and the known differences (one entry per page and check, with its reason; an unused entry fails). A render must match its golden exactly (no pixel over ΔE 2.3, SSIM ≥ 0.99), so a one-pixel-wide change fails; parity uses the spike's `pass` thresholds.

```sh
cargo xtask render-golden            # compare; diff images (expected | actual | diff) in target/xtask/render-golden/diff/
cargo xtask render-golden --update   # rewrite the PNGs; review them like code
```

Both need Python with `tools/render/requirements.txt` (`$LAMINA_PYTHON`, else `python3`). The PNGs are lamina's own output rendered by the two engines; they carry the licence of the examples (MIT OR Apache-2.0) and of the OFL fonts drawn in them (`fixtures/fonts/OFL.txt`).
