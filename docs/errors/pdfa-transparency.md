# `pdfa-transparency`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/A-1, and something the edits added or changed is see-through: an annotation that is translucent or blends with the page as a highlight does, or an image or shape placed on a page with an opacity below 1, or an image with an alpha channel. Saving the file as it is writes one that no longer meets PDF/A-1.

## Why

PDF/A-1 predates transparency in PDF and forbids it (ISO 19005-1 §6.4). A highlight multiplies its colour with the text under it so the text stays legible, and that blending is transparency. So is a faded image, and a PNG whose edges fade to clear, which Lamina PDF writes with a soft mask. Later parts of PDF/A, PDF/A-2 and PDF/A-3, allow it.

## How to fix

Mark the text another way, with an underline or a strike-out, or make the mark opaque. Place an added image or shape at full opacity, and use a JPEG or a PNG without an alpha channel. If the file needn't stay PDF/A-1, save anyway: the warning never blocks a save.

## Example

A highlight added to a PDF/A-1b file, then `doc.plan()`, lists `pdfa-transparency`.

## Data fields

- `annotations`: the IDs of the translucent or blending annotations, in page order.
- `pages`: the IDs of the pages with see-through objects the edits added, in page order; empty when only annotations are.

## For contributors

The decisions and todos behind this code:

- [E4c](../todos/done/E4c-appearances-markup-notes-caret.md): text markup, notes and carets
- [E6b](../todos/done/E6b-place-images-and-shapes.md): images, shapes, logos and stamps placed as page content
- [E4 plan §3](../plans/editor/e4-markup-and-drawing-annotations.md#3-conformance-the-edits-must-keep): conformance the edits must keep
