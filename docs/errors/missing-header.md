# `missing-header`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

No `%PDF-x.y` header was found in the first 1,024 bytes; parsing went ahead.

## Why

Files are often truncated at the front or prefixed with other data.

## How to fix

Nothing, if the document reads correctly. Use strict mode to refuse such files.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None.
