# `invalid-geometry`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A page size, page box, `/UserUnit` or image size given to the builder is not usable.

## Why

Page sizes and `/UserUnit` must be finite and positive, page boxes finite, and images at least one pixel wide and tall.

## How to fix

Check the values passed to `Builder::page`, `PageBuilder::user_unit`, the box setters and `Image::rgba`.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `what`: which value was invalid (fixed text).
