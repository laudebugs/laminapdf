# `unsupported-layout-option`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

`break_lines` was given a layout option lamina does not implement yet: `LayoutOptions::justify` set to `true`.

## Why

Justification (stretching inter-word gaps, kashida) is reserved for a later plan (research 03 §7.6); silently ignoring the option would lay out ragged lines the caller did not ask for.

## How to fix

Leave `justify` at `false` and pick an `Alignment` (start, end, left, right or centre).

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `option`: the option's name (fixed text).
