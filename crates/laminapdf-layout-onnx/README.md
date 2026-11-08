# laminapdf-layout-onnx

`laminapdf-layout-onnx` gives `laminapdf-extract` a layout model: ONNX Runtime runs Docling's Heron detector, which finds each page's text, headings, lists, tables, pictures and running headers, and extraction then reads the page by those regions. It helps on pages the rules alone misread, such as magazines and columns set close together. The crate is an edge crate: it loads a native library, so it stays out of the core and out of WASM.

```rust,no_run
use std::sync::Arc;
use laminapdf_extract::{ExtractOptions, ModelSlot, Models, PageRenderer};
use laminapdf_layout_onnx::HeronLayout;

fn options(renderer: Arc<dyn PageRenderer>) -> Result<ExtractOptions, Box<dyn std::error::Error>> {
    let weights = std::fs::read("model.onnx")?;
    let heron = HeronLayout::new(&weights, "/usr/lib/libonnxruntime.so.1.31.1")?;
    let models = Models::new()
        .with_layout(ModelSlot::Plugin(Arc::new(heron)))
        .with_renderer(renderer);
    Ok(ExtractOptions::new().with_models(models))
}
```

## What you supply

- **The weights.** `HeronLayout::new` takes the bytes of the official ONNX export and refuses any other file: the SHA-256 must be the one pinned in [`tools/models/models.toml`](../../tools/models/models.toml). `cargo xtask models fetch docling-layout-heron` downloads it (171 MB) into `target/models/` and checks it; an app downloads it on first use or ships it beside itself, never inside a crate or package.
- **ONNX Runtime 1.31.** The crate loads the library from the path you give, once per process, and downloads nothing at build time (`ort` without its default features, `load-dynamic`). Microsoft's release archives and the `onnxruntime` Python package both carry the library.
- **A renderer.** Extraction never renders; give it a `PageRenderer`, for example one over `laminapdf-render`.

## What it does with the page

The export's graph scales and normalises the image itself, so the crate only resizes the page to 640 × 640 (bilinear) and passes the pixels as they are. The crate keeps the candidate boxes scoring at least 0.3, best first (`HeronLayout::with_threshold` changes the threshold). Heron sometimes names one box twice, a page title as both title and section header, so a box that a better one overlaps by 90% or more is dropped. Heron's 17 labels map onto `laminapdf-extract`'s `RegionLabel`; forms, code, indexes and checkboxes become `Other`.

## Licences

| Part | Licence | Source |
| --- | --- | --- |
| Heron weights and the ONNX export | Apache-2.0 | [model card](https://huggingface.co/docling-project/docling-layout-heron-onnx) |
| Docling's model code | MIT | [docling-ibm-models](https://github.com/docling-project/docling-ibm-models) |
| RT-DETR | Apache-2.0 | [RT-DETR](https://github.com/lyuwenyu/RT-DETR) |
| Training data (DocLayNet) | CDLA-Permissive-1.0 | [DocLayNet](https://huggingface.co/datasets/docling-project/DocLayNet) |
| ONNX Runtime | MIT | [onnxruntime](https://github.com/microsoft/onnxruntime) |
| `ort` | MIT OR Apache-2.0 | [ort](https://github.com/pykeio/ort) |

An app that ships the weights or ONNX Runtime ships their licence and notices too.

## Not published yet

The crate is `publish = false` until the first release decides how apps get ONNX Runtime. `ort` is still a release candidate (`=2.0.0-rc.15`), pinned exactly.

## Tests

The unit tests cover the resize, the label map and the box filter without a runtime. `tests/heron.rs` runs the model on two example pages rendered by hayro and compares the regions with a reference run, [`tools/models/heron_reference.py`](../../tools/models/heron_reference.py), made with ONNX Runtime's Python package and PDFium. It is ignored by default, since it needs the weights and the library:

```sh
cargo xtask models fetch docling-layout-heron
LAMINA_PDF_HERON_MODEL=target/models/docling-layout-heron/<sha256>/model.onnx \
LAMINA_PDF_ONNXRUNTIME=/path/to/libonnxruntime.so.1.31.1 \
  cargo test -p laminapdf-layout-onnx --test heron -- --ignored
```

Every region the reference scored 0.5 or more must be found with the same label (title and section header count as one) and boxes overlapping by at least 80%: the two renders differ, so the scores and boxes do too.
