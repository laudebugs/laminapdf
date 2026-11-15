---
title: "unknown-font"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-font/"
---

# unknown-font

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A `FontKey` was used with a `FontRegistry` that did not hand it out.

## Why

Keys are indices into one registry; each document has its own.

## How to fix

Use the key returned by this registry's `add`.

## Example

```rust
let key = registry_a.add(font, EmbedPolicy::Respect)?;
registry_b.encode(key, "Hi")?; // unknown-font: key came from registry_a
```

## Data fields

- `key`: the key's index.
