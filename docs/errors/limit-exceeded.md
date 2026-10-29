# `limit-exceeded`

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

A hard `Limits` value was exceeded.

## Why

Hostile or unusually large input (ADR 0003 §3).

## How to fix

Raise the named limit only if you trust the input.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

`limit` (the limit's name).

The `location` tells where it was hit: `offset` (a file offset) for the nesting and string-length limits and the read window, `object` with `streamOffset` for the same limits inside an object stream, `object` alone for a reference chain; none for decoding budgets.
