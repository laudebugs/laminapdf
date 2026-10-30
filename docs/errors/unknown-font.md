# `unknown-font`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A `FontKey` was used with a `FontRegistry` that did not hand it out.

## Why

Keys are indices into one registry; each document has its own.

## How to fix

Use the key returned by this registry's `add`.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `key`: the key's index.
