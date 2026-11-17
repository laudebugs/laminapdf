# `pdfa-font-not-embedded`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/A, and an annotation the edits added, changed or flattened uses a font that isn't embedded in the file. A text box drawn in Helvetica, Times or Courier is one case. Flattening is another: it draws an annotation's appearance into the page, and an appearance another app made may name a font it didn't embed. An edit to a page's content that adds a font without embedding it is the third. Saving the file as it is writes one that no longer meets PDF/A.

## Why

PDF/A requires every font a page or annotation draws with to be embedded (ISO 19005-2 §6.2.11.4.1), so the file looks the same wherever it is opened, decades later. Helvetica, Times and Courier are the standard 14 fonts that readers supply themselves, which is why Lamina PDF can draw text boxes in them without a font file.

## How to fix

For a text box, give it a font by its bytes: add a font file with `addBlob` and name its ID as the box's `font`. Lamina PDF embeds the characters the box uses as a subset. For a flattened annotation, undo the flattening, then embed the font in the annotation's appearance with the app that made it, or flatten a copy that needn't stay PDF/A. If the file needn't stay PDF/A, save anyway: the warning never blocks a save.

## Example

A text box in Helvetica added to a PDF/A-2b file, then `doc.plan()`, lists `pdfa-font-not-embedded`. So does a PDF/A-2b file whose Helvetica text box is then flattened with `doc.flattenAnnotations("all", "print")`.

## Data fields

- `annotations`: the IDs of the annotations, in page order. For flattening, the IDs of the annotations it removed, whose appearances are now page content. Empty when only pages are named.
- `pages`: the IDs of the pages whose edited content uses an added font that isn't embedded, in page order. Empty when only annotations are named.

## For contributors

The decisions and todos behind this code:

- [E4e](../todos/done/E4e-freetext-callouts.md): text boxes and callouts
- [E5e](../todos/done/E5e-flatten-and-summary.md): flattening annotations
- [E7i](../todos/ongoing/E7i-structure-and-conformance-maintenance.md): content edits
