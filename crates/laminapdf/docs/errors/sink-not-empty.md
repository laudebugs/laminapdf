---
title: "sink-not-empty"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/sink-not-empty/"
---

# sink-not-empty

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

The `WriteSink` already held bytes when the save started, so the writer refused before writing anything.

## Why

Output offsets (the xref table, `startxref`) count from the sink's first byte, and a sink only appends. An `AppendedOnly` incremental save also accepts a sink that holds exactly the base.

## How to fix

Pass an empty sink. The document is unchanged.

## Example

```rust
let mut sink = b"%PDF-".to_vec();
ops.save(&mut sink, &options)?; // sink-not-empty, data.held = 5
```

## Data fields

- `held`: the bytes the sink held.
