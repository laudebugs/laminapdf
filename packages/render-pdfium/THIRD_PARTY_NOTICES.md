# Third-party notices: `@lamina/render-pdfium`

This package loads `pdfium.wasm` from [`@embedpdf/pdfium`](https://www.npmjs.com/package/@embedpdf/pdfium) 2.15.1 (SHA-256 `5e4cd023c3dad4a895b3571ca573d3fc51bac6de48360d95283134385b954eaa`, checked by `scripts/verify-pdfium.mjs`). The `@embedpdf/pdfium` package ships none of the notices below ([C1b](../../spikes/pdfium-wasm/README.md#licences)); [ADR 0004 §3](../../docs/decisions/0004-render-backend.md#3-the-pinned-pdfium-builds) requires every product that ships PDFium to carry them. An app that bundles this package ships this file, plus the full licence texts it links to, with its other notices.

## Where the list comes from

- EmbedPDF builds `pdfium.wasm` from its PDFium fork, [`embedpdf/runtime`](https://github.com/embedpdf/runtime) (`scripts/embedpdf-runtime/build-target.sh`).
- That build sets `pdf_enable_xfa=false`, `pdf_enable_v8=false` and `pdf_use_skia=false`. So V8, Skia and XFA's own dependencies (libtiff among them) are **not** linked, and the rasteriser is PDFium's copy of AGG.
- **Confirmed in the binary's strings:** libpng, zlib, OpenJPEG. The symbol names are stripped, so the others below are inferred from PDFium's non-XFA build: FreeType, Little-CMS 2, libjpeg-turbo and AGG are core dependencies of PDFium's codecs and font engine.
- **Possibly linked, depending on the PDFium revision:** Abseil, fast_float and simdutf. Their notices are listed so a bundle is never short of one.
- **To do on every re-pin (ADR 0004 §3):** re-check this list against the fork's `DEPS` and `BUILD.gn` at the pinned revision.

## Components

| Component | Licence | Required notice or attribution |
| --- | --- | --- |
| `@embedpdf/pdfium` (the wrapper and its JS glue) | MIT | © 2024 CloudPDF, Ji Chang. The MIT text ships as the package's `LICENSE`. |
| PDFium | BSD-3-Clause; parts Apache-2.0 | © 2014 The PDFium Authors. Both texts ship as the package's `LICENSE.pdfium`. |
| FreeType | FreeType Project License (FTL), chosen over GPLv2 | "Portions of this software are copyright © The FreeType Project (www.freetype.org). All rights reserved." ([FTL](https://gitlab.freedesktop.org/freetype/freetype/-/blob/master/docs/FTL.TXT)) |
| Little-CMS 2 | MIT | Copyright © Marti Maria Saguer ([COPYING](https://github.com/mm2/Little-CMS/blob/master/LICENSE)) |
| libjpeg-turbo | IJG License, BSD-3-Clause, zlib | "This software is based in part on the work of the Independent JPEG Group." ([LICENSE.md](https://github.com/libjpeg-turbo/libjpeg-turbo/blob/main/LICENSE.md)) |
| libpng | PNG Reference Library License v2 | Copyright © the libpng authors ([LICENSE](https://github.com/pnggroup/libpng/blob/libpng16/LICENSE)) |
| zlib | zlib | Copyright © Jean-loup Gailly and Mark Adler ([zlib licence](https://zlib.net/zlib_license.html)) |
| OpenJPEG | BSD-2-Clause | Copyright © Université catholique de Louvain, CNES, and the OpenJPEG contributors ([LICENSE](https://github.com/uclouvain/openjpeg/blob/master/LICENSE)) |
| Anti-Grain Geometry 2.3 (PDFium's copy) | AGG 2.3 licence (permissive) | Copyright © 2002–2005 Maxim Shemanarev ([PDFium's copy](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/third_party/agg23/)) |
| Abseil (if linked) | Apache-2.0 | © The Abseil Authors |
| fast_float (if linked) | MIT OR Apache-2.0 OR BSL-1.0 | © The fast_float authors |
| simdutf (if linked) | MIT OR Apache-2.0 | © The simdutf authors |

**Licences:** all of these are permissive, and none is GPL, LGPL or AGPL ([ADR 0001](../../docs/decisions/0001-core-language-and-licence.md)). FreeType is used under the FTL.
