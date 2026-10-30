# `limit-exceeded`

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

A hard `Limits` value was exceeded.

## Why

Hostile or unusually large input (ADR 0003 §3).

## How to fix

Raise the named limit only if you trust the input.

## Example

```rust
let options = OpenOptions::default().with_limits(Limits::new().with_max_nesting_depth(1));
let doc = Document::open_bytes(bytes, options)?;
let err = doc.page_count().unwrap_err();
// { "code": "limit-exceeded", "kind": "limit", "data": { "limit": "max_nesting_depth" },
//   "location": { "object": 9, "streamOffset": 89 }, … }
```

## Data fields

`limit` (the limit's name).

The `location` tells where it was hit: `offset` (a file offset) for the nesting and string-length limits and the read window, `object` with `streamOffset` for the same limits inside an object stream, `object` alone for a reference chain; none for decoding budgets.
