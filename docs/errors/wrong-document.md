# `wrong-document`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`FontRegistry::finish_fonts` was given a document in which a font's reserved object is not a free number reserved in that document.

## Why

A `FontRegistry` belongs to one document: `reference` reserves numbers in the document it is given, and `finish_fonts` fills those numbers. Finishing into another document would overwrite its objects, so nothing is written.

## How to fix

Use one `FontRegistry` per document, and pass `finish_fonts` the document its `reference` calls used.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `object`: the reserved object number.
