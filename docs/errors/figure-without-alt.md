# `figure-without-alt`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file claims PDF/UA (part 1 or 2), and an edit replaced the image of a figure. The figure's alternative text described the old image, so Lamina PDF removed it, and the figure now has none. Saving the file as it is writes one that no longer meets PDF/UA.

## Why

PDF/UA asks that every figure carry alternative text, which a screen reader reads in place of the picture (ISO 14289-1 §7.3). Text written for the old image would describe a picture the page no longer shows.

## How to fix

Undo the image replacement, or add alternative text with an accessibility tool and open the file again: once the figure has alternative text, the warning goes. If the file needn't stay PDF/UA, save without the claim: apply `removeConformanceClaims` with `pdfua` set. The warning never blocks the edit or the save.

## Example

Replacing the image of a tagged figure in a file whose XMP metadata has `pdfuaid:part` 1, then `doc.plan()`, lists `figure-without-alt`.

## Data fields

- `pages`: the IDs of the pages the figures are on, in page order.

## For contributors

The decisions and todos behind this code:

- [E7i](../todos/done/E7i-structure-and-conformance-maintenance.md): tags kept through content edits
- [E7 plan §3](../plans/editor/e7-editing-existing-content.md#3-conformance-the-edits-must-keep): conformance the edits must keep
