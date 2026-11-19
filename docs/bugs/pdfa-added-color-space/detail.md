# Images and copied pages added to a PDF/A file ignored its output intent

[Resolution](resolution.md). Reported as [#750](https://github.com/laudebugs/laminapdf/issues/750), from the review of [E6b](../../todos/done/E6b-place-images-and-shapes.md).

## What we saw

A CMYK photo placed on a PDF/A-2b page whose output intent is sRGB saved with no warning: `doc.plan()` listed no conformance warning, and the saved file no longer met PDF/A-2b. The same photo added as an image stamp, and a page of another PDF placed as content, were missed too. Shapes and annotation appearances that Lamina PDF draws were fine when the intent is RGB or CMYK, because they paint in the intent's space.

PDF/A ties a file's device colours to its output intent (ISO 19005-2 §6.2.4.3; ISO 19005-1 §6.2.3.3 for PDF/A-1). `DeviceRGB` needs an RGB intent and `DeviceCMYK` a CMYK one. `DeviceGray` is allowed with any intent, and without an intent no device space is allowed. On PDF/A-2 and later a `Default` space in the resources may replace the device space (ISO 32000-2 §8.6.5.6). A space with its own ICC profile (`ICCBased`), or another device-independent space, is always allowed.

## How to reproduce

Open `fixtures/golden/examples/pdf_a_archive-2b.pdf` (PDF/A-2b, an sRGB intent with `/N 3`). Add `fixtures/images/photo-cmyk.jpg` with `AddContentObject` and `ContentObjectSpec::Image`, or as `AddAnnot` with `StampKind::Image`, then ask the save planner for its conformance warnings. Before the fix the list had nothing about colour; the image XObject in the overlay had `/ColorSpace /DeviceCMYK`.

## Cause

All three paths take their colour space from what was added, not from the file's intent. Two writers are involved:

- `laminapdf-build`'s image writer (`resources::write_image`) writes a JPEG or PNG in its own device space: `DeviceGray`, `DeviceRGB` or `DeviceCMYK` from the frame header, never an ICC space. Images placed as content and image stamps both go through it.
- `copy_page` keeps a copied page's content and resources as they were: the colours its operators set (`rg`, `k`, `cs`), its images, inline images, patterns and shadings.

The save planner's PDF/A checks for added content (`added::pdfa_pages`) and annotations (`annot::conformance`) looked at transparency and fonts only, so no check compared these spaces with the intent. Pages copied with `InsertPagesFrom` were covered already: they raise `imported-pdfa-unchecked`.
