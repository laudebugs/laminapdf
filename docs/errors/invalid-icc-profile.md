# `invalid-icc-profile`

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

`IccProfile::new` (`lamina-build`) rejected the bytes.

## Why

An ICC profile (ICC.1:2010) starts with a 128-byte header holding its size, the `acsp` signature, its class and its data colour space. PDF's `ICCBased` colour spaces (ISO 32000-2 §8.6.5.5) take Gray, RGB, CMYK and Lab profiles; device-link, abstract and named-colour profiles can't be used.

## How to fix

Pass the complete bytes of an input, display, output or colour-space profile with a Gray, RGB, CMYK or Lab data colour space.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `reason`: what was wrong (fixed text).
