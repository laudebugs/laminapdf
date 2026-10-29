# `sink-not-empty`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

The `WriteSink` already held bytes when the save started, so the writer refused before writing anything.

## Why

Output offsets (the xref table, `startxref`) count from the sink's first byte, and a sink only appends (ADR 0007 §6). An `AppendedOnly` incremental save also accepts a sink that holds exactly the base.

## How to fix

Pass an empty sink. The document is unchanged.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `held`: the bytes the sink held.
