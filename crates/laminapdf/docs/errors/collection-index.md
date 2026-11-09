---
title: "collection-index"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/collection-index/"
---

# collection-index

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Font::from_collection` was asked for a member past the end of the font collection.

## Why

Members are numbered from 0; a single font counts as a collection of one.

## How to fix

Use an index below `Font::collection_len(bytes)`.

## Example

```rust
let font = Font::from_collection(ttc_bytes, 4)?; // the collection has 4 members (0..=3): collection-index
```

## Data fields

- `index`: the member that was asked for.
- `count`: how many members the collection has.
