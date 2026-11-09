---
title: "color-components"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/color-components/"
---

# color-components

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Color::icc` was given a number of components that differs from the profile's channel count.

## Why

An `ICCBased` colour has one component per channel of its profile (`/N`: Gray 1, RGB and Lab 3, CMYK 4).

## How to fix

Give `IccProfile::components()` values.

## Example

```rust
let c = Color::icc(&cmyk_profile, &[0.1, 0.2, 0.3])?; // color-components: 4 expected
```

## Data fields

- `expected`: the colour space's number of components.
- `actual`: how many were given.
