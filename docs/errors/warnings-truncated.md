# `warnings-truncated`

- Type: warning
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

More than `Limits::max_warnings` warnings were raised; the rest were dropped.

## Why

A badly damaged or hostile file.

## How to fix

Raise `max_warnings` if you need the full list.

## Example

A damaged file raising 50,000 repairs with the default `max_warnings`: the first warnings are kept, then one `warnings-truncated`, and the rest are dropped.

```rust
let limits = laminapdf::Limits::default().with_max_warnings(100_000);
let doc = laminapdf::Document::open_bytes(bytes, laminapdf::OpenOptions::new().with_limits(limits))?;
```

## Data fields

None.
