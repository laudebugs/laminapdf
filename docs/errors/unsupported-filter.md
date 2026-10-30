# `unsupported-filter`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

A stream uses a filter lamina doesn't implement; only decoding that stream fails.

## Why

Rare or proprietary filters.

## How to fix

Use a tool that supports the filter.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

`filter` (**untrusted**: read from the file). The message never includes it.
