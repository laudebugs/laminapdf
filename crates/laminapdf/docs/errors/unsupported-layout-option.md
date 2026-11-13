---
title: "unsupported-layout-option"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/unsupported-layout-option/"
---

# unsupported-layout-option

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

`break_lines` was given a layout option Lamina PDF does not implement yet: `LayoutOptions::justify` set to `true`.

## Why

Justification (stretching inter-word gaps, kashida) is reserved for a later plan (research 03 §7.6); silently ignoring the option would lay out ragged lines the caller did not ask for.

## How to fix

Leave `justify` at `false` and pick an `Alignment` (start, end, left, right or centre).

## Example

```rust
let options = LayoutOptions { justify: true, ..LayoutOptions::new(300.0, 12.0) };
break_lines(&shaper, &fonts, text, &options)?; // unsupported-layout-option
```

## Data fields

- `option`: the option's name (fixed text).
