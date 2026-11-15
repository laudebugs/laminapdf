---
title: "empty-destination-name"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/empty-destination-name/"
---

# empty-destination-name

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op names a destination with an empty name.

## Why

Links and bookmarks find a named destination by its name, so it needs at least one character.

## How to fix

Give the destination a name. Names are case-sensitive and can hold any text.

## Example

```rust
doc.apply(vec![Op::from(DeleteNamedDestination::new(""))])?; // empty-destination-name
```

## Data fields

None.
