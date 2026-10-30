# `encrypted-base`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

A save was requested for an encrypted document without `SaveOptions::with_remove_encryption(true)` (every mode: `Auto`, `Incremental` and `FullRewrite`). Nothing was written. `lamina_save::SaveError::EncryptedBase` and the writers' `lamina_cos::Error::EncryptedBase` share this code.

## Why

Encryption on write is not implemented yet (full-editor E11), and lamina never drops or corrupts encryption silently ([ADR 0007](../decisions/0007-save-planner.md) §9).

## How to fix

Save with `SaveOptions::with_remove_encryption(true)` to write a plaintext full rewrite (every string and stream decrypted, no `/Encrypt`), or wait for encryption on save (E11b). A signed and encrypted document also needs `with_invalidate_signatures(true)`.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

None.
