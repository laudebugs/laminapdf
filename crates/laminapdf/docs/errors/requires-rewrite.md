---
title: "requires-rewrite"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/requires-rewrite/"
---

# requires-rewrite

- Type: error (`laminapdf_save::SaveError::RequiresRewrite`)
- Kind: `state` (the call isn't valid in the document's current state)

## What happened

An incremental save was requested, but only a full rewrite is safe for this document. Nothing was written.

## Why

The save planner found one of four reasons the file must be rewritten:

- a repaired base, whose cross-reference data an appended revision can't build on;
- an encrypted base being saved without its encryption;
- a document built from scratch, with no base bytes at all;
- a pending op, such as a redaction, that removes content an earlier revision would keep.

## How to fix

Save with `SaveMode::Auto` (it resolves to a full rewrite) or `SaveMode::FullRewrite`. If the document is also signed, that needs the explicit override ([`would-invalidate-signatures`](would-invalidate-signatures.md)).

## Example

```rust
let err = doc.save(&mut out, &SaveOptions::new().with_mode(SaveMode::Incremental)).unwrap_err();
assert_eq!(err.code(), "requires-rewrite");
```

## Data fields

- `reasons`: the rewrite reasons (`RepairedBase`, `EncryptedBase`, `NoBase`, `OpRequiresRewrite`).
