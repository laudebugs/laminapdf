---
title: "would-invalidate-signatures"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/would-invalidate-signatures/"
---

# would-invalidate-signatures

- Type: error (`laminapdf_save::SaveError::WouldInvalidateSignatures`)
- Kind: `security` (signatures block the call)

## What happened

The save planner refused a save because it would break one or more signatures. Either it was a full rewrite of a signed document, or its pending changes or rewrite reasons conflict with the document's signatures: a DocMDP, FieldMDP or usage-rights restriction, or a repaired or encrypted base. Nothing was written.

## Why

A signature covers the bytes of the file as it was signed (ISO 32000-2 §12.8). A full rewrite moves those bytes, and a change a certification forbids invalidates it even when appended. Lamina PDF never breaks a signature without an explicit override.

## How to fix

Save with `SaveMode::Auto` or `SaveMode::Incremental` when the constraint allows it. Otherwise ask the user, naming the signatures in the error, and save again with `SaveOptions::with_invalidate_signatures(true)`: the invalidated signatures' values and `/Perms` entries are removed and `SaveReport::invalidated` lists them.

## Example

```rust
let err = doc.save(&mut out, &SaveOptions::new().with_mode(SaveMode::FullRewrite)).unwrap_err();
assert_eq!(err.code(), "would-invalidate-signatures");
```

## Data fields

- `signatures`: every signature, as `{ field, kind, value, revision, covers }` (field ID, kind, signature dictionary as `num gen`, revision, covered length).
- `rewrite_reasons`: why only a full rewrite is safe, if anything says so.
- `violations`: pending changes a signature's restriction forbids, as `{ class, restriction, signature }`.
- `signatures_unknown`: part of the form couldn't be followed, so there may be signatures beyond `signatures`.

None of them hold document text.
