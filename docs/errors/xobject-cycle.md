# `xobject-cycle`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A Form XObject draws itself, directly or through other forms; the inner `Do` was skipped (ADR 0023).

## Why

A malformed or hostile file.

## How to fix

Nothing to do.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `object`: the form reached again.

The `location` is `object` (the form reached again).
