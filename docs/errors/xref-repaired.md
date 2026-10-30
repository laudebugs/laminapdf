# `xref-repaired`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The stored cross-reference data was unusable and was rebuilt by scanning the file.

## Why

Edited, truncated or concatenated files often have stale offsets (ADR 0003 §1).

## How to fix

Nothing, if the result is right. Incremental saves refuse a repaired base (`repaired-base`); save in full instead.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

None.
