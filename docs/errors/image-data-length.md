# `image-data-length`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Image::rgba(width, height, bytes)` was given a number of bytes other than `width × height × 4`.

## Why

Raw RGBA samples are 4 bytes per pixel, row by row from the top, with no padding.

## How to fix

Pass exactly 4 bytes per pixel.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `expected`: the length the dimensions call for.
- `actual`: the length given.
