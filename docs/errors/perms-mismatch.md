# `perms-mismatch`

- Type: warning
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

An R5/R6 document's `/Perms` does not match its `/P` and `/EncryptMetadata`.

## Why

`/P` may have been tampered with (ISO 32000-2 Algorithm 13).

## How to fix

Treat the permissions as untrusted.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

None.
