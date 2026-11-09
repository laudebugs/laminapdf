---
title: "encrypted-base"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/encrypted-base/"
---

# encrypted-base

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

A save was requested for an encrypted document without `SaveOptions::with_remove_encryption(true)` (every mode: `Auto`, `Incremental` and `FullRewrite`). Nothing was written. `laminapdf_save::SaveError::EncryptedBase` and the writers' `laminapdf_cos::Error::EncryptedBase` share this code.

## Why

Lamina PDF can't encrypt on save yet, and it never drops or corrupts encryption silently.

## How to fix

Save with `SaveOptions::with_remove_encryption(true)` to write a plaintext full rewrite (every string and stream decrypted, no `/Encrypt`), or wait until Lamina PDF can encrypt on save. A signed and encrypted document also needs `with_invalidate_signatures(true)`.

## Example

```rust
let err = doc.save(&SaveOptions::default()).unwrap_err(); // encrypted-base
let saved = doc.save(&SaveOptions::default().with_remove_encryption(true))?;
```

## Data fields

None.
