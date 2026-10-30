# `warnings-truncated`

- Type: warning
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

More than `Limits::max_warnings` warnings were raised; the rest were dropped.

## Why

A badly damaged or hostile file.

## How to fix

Raise `max_warnings` if you need the full list.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

None.
