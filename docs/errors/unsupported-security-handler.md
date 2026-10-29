# `unsupported-security-handler`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

The document uses a security handler lamina can't decrypt.

## Why

Not `/Standard`, an unknown `/V`/`/R`, or an unknown crypt filter method.

## How to fix

Use a tool that supports the handler.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

`filter` (**untrusted**: read from the file; the message never includes it), `version`, `revision`.
