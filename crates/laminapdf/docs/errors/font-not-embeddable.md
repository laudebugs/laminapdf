---
title: "font-not-embeddable"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/font-not-embeddable/"
---

# font-not-embeddable

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`FontRegistry::add` refused a font whose `OS/2.fsType` forbids embedding (restricted licence, or bitmap embedding only).

## Why

Font vendors mark fonts whose licence does not allow embedding in documents (OpenType `OS/2` table; research 03 §6.3). `fsType` is advisory; the licence is what counts.

## How to fix

Use another font, or, if your licence allows embedding, register it with `EmbedPolicy::Override`.

## Example

```rust
registry.add(restricted_font, EmbedPolicy::Respect)?;  // font-not-embeddable
registry.add(restricted_font, EmbedPolicy::Override)?; // only if your licence allows embedding
```

## Data fields

- `font`: the font's PostScript name (**untrusted**: read from the font).
- `fs_type`: the `fsType` bits.
