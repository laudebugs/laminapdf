# heron-web

`check.mjs` runs Docling's Heron layout model through onnxruntime-web, the runtime a browser uses, and checks it finds the regions the native reference run found. It renders the two reference pages with hayro and runs Heron through [`@laminapdf/layout-heron`](../../packages/layout-heron/README.md). Then it compares the regions with [`heron.json`](../../crates/laminapdf-layout-onnx/tests/reference/heron.json), the regions [`tools/models/heron_reference.py`](../models/heron_reference.py) recorded with ONNX Runtime's Python package and PDFium. The test is the Rust crate's: every region the reference scored 0.5 or more is found with the same label (a title and a section header count as one) and boxes overlapping by at least 80%.

This folder is its own pnpm project, outside the root workspace, so onnxruntime-web (150 MB) stays out of every other install. It is run by hand, never by tests:

```sh
cargo xtask wasm
pnpm --filter @laminapdf/render --filter @laminapdf/layout-heron build
cargo xtask models fetch docling-layout-heron
pnpm --dir tools/heron-web install --frozen-lockfile
node tools/heron-web/check.mjs target/models/docling-layout-heron/<sha256>/model.onnx
```

It runs ONNX Runtime's WebAssembly build in Node with one thread, the build a browser runs. On 2026-11-08 it found all nine regions, with overlaps from 0.964 to 0.997, in about 17 seconds for two pages.
