# `encrypted-base`

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

A low-level writer that can't encrypt was asked to append to an encrypted document. Nothing was written. `laminapdf_save::SaveError::EncryptedBase` and the writers' `laminapdf_cos::Error::EncryptedBase` share this code.

## Why

Only the small helper writers that Lamina PDF's own tests use can't encrypt the objects they append. A normal save can: an encrypted document keeps its protection on every save, and Lamina PDF never drops or corrupts encryption silently.

## How to fix

Save through `Save::save`, with an entropy seed (`SaveOptions::with_entropy_seed`), to keep the protection. `SaveOptions::with_remove_encryption(true)` writes a plaintext full rewrite instead (every string and stream decrypted, no `/Encrypt`). A signed and encrypted document also needs `with_invalidate_signatures(true)` for a rewrite.

## Example

```rust
let saved = doc.save(&mut out, &SaveOptions::new().with_entropy_seed(seed))?; // keeps the protection
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [ADR 0007](../decisions/0007-save-planner.md) §9
- [E11b](../todos/done/E11b-encrypt-on-write.md): encryption on save
