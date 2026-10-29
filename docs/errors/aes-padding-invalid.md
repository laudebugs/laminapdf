# `aes-padding-invalid`

- Type: warning
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

AES-encrypted data was not `IV ‖ whole blocks` or had invalid padding; it was kept as stored or unstripped.

## Why

Damaged data, or plaintext spliced into an encrypted file (PDFex-style).

## How to fix

Treat the affected content as suspect.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None.
