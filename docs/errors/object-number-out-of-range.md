# `object-number-out-of-range`

- Type: warning
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

An xref entry above `Limits::max_object_number` was dropped.

## Why

Corrupt or hostile xref data.

## How to fix

Raise `max_object_number` only for trusted input.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None.
