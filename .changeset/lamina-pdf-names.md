---
"@laminapdf/wasm": minor
"@laminapdf/sdk": minor
"@laminapdf/render": minor
"@laminapdf/render-hayro": minor
"@laminapdf/render-pdfium": minor
"@laminapdf/viewer": minor
---

The library is called Lamina PDF, and its public names follow. `@laminapdf/sdk`'s error class is `LaminaPdfError`, checked with `isLaminaPdfError`, and its warnings are `LaminaPdfWarning`. Thrown errors carry the schema ID `laminapdf.error/1`. `@laminapdf/wasm`'s modules are `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract` and `lamina_pdf_render_hayro` (`.js`, `.d.ts` and `.wasm`), and the environment variables the tools read start with `LAMINA_PDF_`.
