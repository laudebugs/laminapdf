# `pdfa-color-space`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/A, and an image or a page of another PDF that the edits added paints in a device colour space the file's output intent doesn't allow. A CMYK photo placed as page content or as an image stamp in a file with an RGB intent is the common case. An RGB image in a file printed for CMYK is the reverse. Saving the file as it is writes one that no longer meets PDF/A.

## Why

PDF/A fixes colours to one output intent, an ICC profile that says how the file's device colours look (ISO 19005-2 §6.2.4.3, ISO 19005-1 §6.2.3.3). `DeviceRGB` is allowed only with an RGB intent and `DeviceCMYK` only with a CMYK one. `DeviceGray` is allowed with either, and a file without an intent allows no device colour at all. A colour that carries its own ICC profile (`ICCBased`) is always allowed.

Lamina PDF writes a JPEG or PNG in the colour space its file uses, so the photo's space reaches the page unchanged. Shapes and annotation appearances that Lamina PDF draws already paint in the intent's space, so they never raise this warning.

## How to fix

Convert the image to the output intent's colour space before you add it: RGB for an sRGB intent, CMYK for a print intent. A grayscale image fits either. For a page of another PDF, export that page with the same output intent first. If the file needn't stay PDF/A, save anyway or remove the claim with `removeConformanceClaims`: the warning never blocks a save.

## Example

A CMYK JPEG added with `doc.content.add` to a PDF/A-2b file whose output intent is sRGB, then `doc.plan()`, lists `pdfa-color-space` with the page. The same JPEG added as an image stamp lists it with the stamp's ID.

## Data fields

- `annotations`: the IDs of the annotations whose new appearances paint so, such as image stamps, in page order. Empty when only pages are named.
- `pages`: the IDs of the pages whose added images or pages paint so, in page order. Empty when only annotations are named.

## For contributors

The decisions and todos behind this code:

- [E6b](../todos/done/E6b-place-images-and-shapes.md): images, shapes, logos and stamps placed as page content
- [`pdfa-added-color-space`](../bugs/pdfa-added-color-space/detail.md): the bug that added this warning
