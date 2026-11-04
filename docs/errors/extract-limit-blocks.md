# `extract-limit-blocks`

- Type: warning
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

A page had more blocks than `ExtractLimits::max_blocks_per_page` allows. Extraction kept the first blocks in reading order and left the rest out of the output.

## Why

A page of many tiny separate pieces of text, such as a dense table, a map with hundreds of labels, or a file built to slow extraction down.

## How to fix

Raise `max_blocks_per_page` if the page really holds that many blocks.

## Example

```rust
let mut options = laminapdf_extract::ExtractOptions::default();
options.limits.max_blocks_per_page = 50_000;
```

## Data fields

- `limit`: the limit that was reached.

The `location` has the page's `pageIndex`.

## For contributors

The decisions and todos behind this code:

- [L3a](../todos/ongoing/L3a-lamina-extract-reading-order.md): blocks and reading order
