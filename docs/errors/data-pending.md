# `data-pending`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

The `ByteSource` hasn't fetched the listed byte ranges yet (ADR 0005 §2).

## Why

Range loading: the call needs more of the file.

## How to fix

Fetch the ranges and retry the call; the document stays usable.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

`ranges` (offset, length pairs).
