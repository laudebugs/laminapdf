# `stale-generation`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A reference's generation number did not match its xref entry, so it resolved to `null` (ISO 32000-2 §7.3.10).

## Why

Stale references left by an editor.

## How to fix

Nothing; the reference is treated as missing.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

`generation`.
