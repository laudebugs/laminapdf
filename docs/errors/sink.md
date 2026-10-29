# `sink`

- Type: error
- Kind: `io` (the host failed to supply or take bytes)

## What happened

The `WriteSink` failed to take the bytes, already held bytes before the save, or could not give them back.

## Why

The destination failed (a full disk, a closed upload, a refused read-back), or a sink that was not empty was passed: output offsets count from its first byte (ADR 0007 §6).

## How to fix

Check the destination and retry the save with an empty sink. The document is unchanged; the sink may hold a prefix of the output.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `kind`: the I/O error kind, for a writer that failed.
- `message`: the sink's or writer's own text (**untrusted**: it comes from the host and may name a path; never in the message).
- `held`: the bytes a non-empty sink held.
