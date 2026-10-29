# `requires-rewrite`

- Type: error (`lamina_save::SaveError::RequiresRewrite`)
- Kind: `state` (the call isn't valid in the document's current state)

## What happened

An incremental save was requested, but only a full rewrite is safe for this document. Nothing was written.

## Why

The save planner found a rewrite reason ([ADR 0007](../decisions/0007-save-planner.md) §2): a repaired base, whose cross-reference data an appended revision can't build on; an encrypted base being saved without its encryption; or a pending op, such as a redaction, whose removed content an earlier revision would keep.

## How to fix

Save with `SaveMode::Auto` (it resolves to a full rewrite) or `SaveMode::FullRewrite`. If the document is also signed, that needs the explicit override ([`would-invalidate-signatures`](would-invalidate-signatures.md)).

## Example

```rust
let err = doc.save(&mut out, &SaveOptions::new().with_mode(SaveMode::Incremental)).unwrap_err();
assert_eq!(err.code(), "requires-rewrite");
```

## Data fields

- `reasons`: the rewrite reasons (`RepairedBase`, `EncryptedBase`, `OpRequiresRewrite`).
