---
title: "annot-not-tagged"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/annot-not-tagged/"
---

# annot-not-tagged

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/UA-1, but it isn't tagged: it has no structure tree, so a new annotation has no element to join. Saving the file as it is writes one that no longer meets PDF/UA-1.

## Why

PDF/UA-1 asks for every annotation to be an `Annot` element of the structure tree (ISO 14289-1 §7.18.1), which places it in the reading order next to the text it is about. In a tagged file Lamina PDF adds that element itself; a file without a tree gives it nowhere to go.

## How to fix

Export the file again as a tagged PDF from the program that made it, then add the annotation. If the claim is wrong, remove it. The warning never blocks a save.

## Example

A note added to a file whose XMP metadata has `pdfuaid:part` 1 but whose catalog has no `/StructTreeRoot`, then `doc.plan()`, lists `annot-not-tagged`.

## Data fields

- `annotations`: the IDs of the untagged annotations, in page order.
