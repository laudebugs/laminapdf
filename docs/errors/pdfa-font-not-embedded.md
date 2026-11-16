# `pdfa-font-not-embedded`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/A, and a text box the edits added or changed is drawn in Helvetica, Times or Courier. Those fonts aren't embedded in the file, so saving it as it is writes one that no longer meets PDF/A.

## Why

PDF/A requires every font a page or annotation draws with to be embedded (ISO 19005-2 §6.2.11.4.1), so the file looks the same wherever it is opened, decades later. Helvetica, Times and Courier are the standard 14 fonts that readers supply themselves, which is why Lamina PDF can draw text boxes in them without a font file.

## How to fix

Give the box a font by its bytes: add a font file with `addBlob` and name its ID as the box's `font`. Lamina PDF embeds the characters the box uses as a subset. If the file needn't stay PDF/A, save anyway: the warning never blocks a save.

## Example

A text box in Helvetica added to a PDF/A-2b file, then `doc.plan()`, lists `pdfa-font-not-embedded`.

## Data fields

- `annotations`: the IDs of the text boxes, in page order.

## For contributors

The decisions and todos behind this code:

- [E4e](../todos/ongoing/E4e-freetext-callouts.md): text boxes and callouts
