---
title: "unknown-named-destination"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-named-destination/"
---

# unknown-named-destination

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A rename or delete names a destination that the file doesn't define.

## Why

The name is spelled differently, or an earlier op renamed or deleted it. Names are case-sensitive.

## How to fix

Call `namedDestinations()` (`named_destinations()` in Rust) and use one of its names. To create a name, use `setNamedDestination`.

## Example

```rust
doc.apply(vec![Op::from(DeleteNamedDestination::new("no-such-name"))])?; // unknown-named-destination
```

## Data fields

None.
