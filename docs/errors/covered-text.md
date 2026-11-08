# `covered-text`

- Type: warning
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

A page draws text and then paints something opaque over it: a filled box, an image or an annotation. A reader sees the box, but the text under it is still in the file, so copying, search and every text extractor still get it.

## Why

Drawing a black box over a name is the most common failed redaction. The box hides the words on screen and in print, but it doesn't remove them: a PDF keeps every glyph its content draws, whatever is painted over them later. Extraction already leaves such text out of its blocks and lists it under `dropped` as `covered`; this warning says how much of it each page has.

## How to fix

Remove the text itself with a tool that deletes what a redaction covers, then save the file as a full rewrite, so no earlier revision keeps it either. If the box is part of the design and the text under it is meant to be there, the warning tells you nothing new and you can ignore it.

## Example

A page draws "Pay John Smith now." and then a black rectangle over "John Smith". `laminapdf check` reports `covered-text` for page 0 with `chars` 9, and `laminapdf extract` gives `Pay now.`.

## Data fields

- `chars`: how many characters, spaces left out, the page has under covers.
- `regions`: how many covered stretches of text the page has.

The `location` has the page's `pageIndex`. The warning never holds the covered text.

## For contributors

The decisions and todos behind this code:

- [ADR 0030](../decisions/0030-extraction-output-and-model-plugins.md) §4: hidden text and the `covered` flag
- [ADR 0031](../decisions/0031-cli-and-mcp-contract.md) §8: what `laminapdf check` reports
- [T2b](../todos/done/T2b-check-failed-redactions.md): the warning
