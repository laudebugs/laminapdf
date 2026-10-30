# `page-count-mismatch`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The page tree's `/Count` differs from the number of pages walked; the walk wins.

## Why

Writers that don't update `/Count`.

## How to fix

Nothing; lamina uses the walked count.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

`declared`, `walked`.
