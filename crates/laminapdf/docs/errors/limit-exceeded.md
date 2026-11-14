---
title: "limit-exceeded"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/limit-exceeded/"
---

# limit-exceeded

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

A hard `Limits` value was exceeded.

## Why

Hostile or unusually large input.

## How to fix

Raise the named limit only if you trust the input.

## Example

```rust
let options = OpenOptions::new().with_limits(Limits::new().with_max_nesting_depth(1));
let doc = Document::open_bytes(bytes, options)?;
let err = doc.page_count().unwrap_err();
// { "code": "limit-exceeded", "kind": "limit", "data": { "limit": "max_nesting_depth" },
//   "location": { "object": 9, "streamOffset": 89 }, … }
```

## Data fields

`limit` (the limit's name).

The `location` tells where the limit was hit. The nesting and string-length limits and the read window give `offset`, a file offset; the same limits inside an object stream give `object` with `streamOffset`. A reference chain gives `object` alone, and decoding budgets give no location.
