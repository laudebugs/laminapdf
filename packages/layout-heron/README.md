# @laminapdf/layout-heron

`@laminapdf/layout-heron` lets a web app use Docling's Heron layout model with Lamina PDF's extraction. Heron finds each page's text, headings, lists, tables, pictures and running headers, and `extractWithModels` then reads the page by those regions. The regions help on pages the rules alone misread. The package does Heron's image preparation and box filtering; the app supplies ONNX Runtime and the weights.

```ts
import * as ort from "onnxruntime-web";
import { Document, extractWithModels, init } from "@laminapdf/sdk";
import { HeronLayout } from "@laminapdf/layout-heron";

await init({ extract: true });
const heron = await HeronLayout.load(weights, ort);
const doc = Document.open(bytes);
const markdown = await extractWithModels(
  doc,
  { format: "markdown", models: { layout: heron.info } },
  { layout: async ({ page, dpi }) => heron.detect(await renderPage(page, dpi)) },
);
```

`renderPage` is yours: it draws page `page` at `dpi` with `@laminapdf/render-pdfium` or `@laminapdf/render-hayro`, and returns `{ pixels, width, height, widthPt, heightPt }`, the RGBA pixels and the page's size in points.

## What you supply

- **onnxruntime-web 1.31** (MIT). Pass the module itself: the package names only the few calls it makes, so it adds no dependency to your bundle.
- **The weights**, the official ONNX export of Heron (171 MB, Apache-2.0). `HeronLayout.load` refuses any other file: its SHA-256 must be the one [`tools/models/models.toml`](../../tools/models/models.toml) pins. Fetch it once, cache it (Cache Storage, OPFS), and never bundle it.

## What it does with the page

The package resizes the page to 640 × 640 with Pillow's bilinear filter, over white, as `laminapdf-layout-onnx` does natively, so both runtimes see the same pixels. It keeps the boxes scoring at least 0.3, best first, and drops a box a better one overlaps by 90% or more, since Heron sometimes names one box twice. Heron's labels become the extraction's; forms, code, indexes and checkboxes become `other`.

## Tests

`pnpm --filter @laminapdf/layout-heron test` checks the resize, the labels, the box filter and the session's inputs against a fake ONNX Runtime. [`tools/heron-web`](../../tools/heron-web/README.md) runs the real model through onnxruntime-web and compares its regions with the native reference run.

## Licences

The package is MIT OR Apache-2.0. Heron's weights are Apache-2.0, Docling's model code MIT, RT-DETR Apache-2.0 and onnxruntime-web MIT; an app that ships them ships their licences too.
