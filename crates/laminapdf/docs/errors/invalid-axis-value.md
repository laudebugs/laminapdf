---
title: "invalid-axis-value"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/invalid-axis-value/"
---

# invalid-axis-value

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Font::instance` was given a NaN or infinite axis value.

## Why

Variation coordinates are user-space numbers such as `wght` 700; NaN and infinities have no position on an axis.

## How to fix

Pass finite values; values outside the axis range are clamped, so there is no need for infinities.

## Example

```rust
font.instance(&[(OpenTypeTag::WEIGHT, f32::NAN)])?; // invalid-axis-value
```

## Data fields

- `tag`: the axis.
