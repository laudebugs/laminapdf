# `color-components`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Color::icc` was given a number of components that differs from the profile's channel count.

## Why

An `ICCBased` colour has one component per channel of its profile (`/N`: Gray 1, RGB and Lab 3, CMYK 4).

## How to fix

Give `IccProfile::components()` values.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `expected`: the colour space's number of components.
- `actual`: how many were given.
