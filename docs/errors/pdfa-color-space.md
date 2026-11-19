# `pdfa-color-space`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/A, and something the edits added paints in a device colour space the file's output intent doesn't allow. The output intent is an ICC profile that says how the file's device colours look. A CMYK photo placed as page content or as an image stamp in a file with an RGB intent is the common case. An RGB image in a file with a CMYK intent is the reverse.

A page of another PDF counts too: the colours its content sets, its images, patterns and shadings. Saving the file as it is writes one that no longer meets PDF/A.

## Why

PDF/A ties every device colour to the output intent (ISO 19005-2 §6.2.4.3, ISO 19005-1 §6.2.3.3). `DeviceRGB` is allowed only with an RGB intent and `DeviceCMYK` only with a CMYK one. `DeviceGray` is allowed with any intent, and a file without an intent allows no device colour at all. A colour that carries its own ICC profile (`ICCBased`) is always allowed, and so is a device colour that the page's resources replace with a `Default` colour space in PDF/A-2 and later.

Lamina PDF writes a JPEG or PNG in the image's own colour space, so a CMYK photo stays CMYK on the page. A page of another PDF keeps the colours its own content uses. Shapes and annotation appearances that Lamina PDF draws paint in the intent's space when the intent is RGB or CMYK. In a file whose intent is neither, they warn too.

## How to fix

Convert the image to the output intent's colour space before you add it: RGB for an RGB intent, CMYK for a CMYK intent. A greyscale image fits either. Or give the image an ICC profile, which fits any intent. For a page of another PDF, convert that PDF to the same output intent before you add its page. If the file needn't stay PDF/A, save anyway or remove the claim with `removeConformanceClaims`: the warning never blocks a save.

## Example

A CMYK JPEG added with `doc.content.add` to a PDF/A-2b file with an RGB intent, then `doc.plan()`, lists `pdfa-color-space` with the page. The same JPEG added as an image stamp lists it with the stamp's ID.

## Data fields

- `annotations`: the IDs of the annotations whose new appearances paint in a space the intent doesn't allow, such as image stamps, in page order. Empty when only pages are named.
- `pages`: the IDs of the pages whose added images, shapes or pages paint in a space the intent doesn't allow, in page order. Empty when only annotations are named.

## For contributors

The decisions and todos behind this code:

- [E6b](../todos/done/E6b-place-images-and-shapes.md): images, shapes, logos and stamps placed as page content
- [`pdfa-added-color-space`](../bugs/pdfa-added-color-space/detail.md): the bug that added this warning
