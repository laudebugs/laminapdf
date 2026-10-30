# `aes-padding-invalid`

- Type: warning
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

AES-encrypted data was not `IV ‖ whole blocks` or had invalid padding; it was kept as stored or unstripped.

## Why

Damaged data, or plaintext spliced into an encrypted file (PDFex-style).

## How to fix

Treat the affected strings and streams as suspect: they may have been altered or spliced in. Don't use them for anything security-relevant.

## Example

```text
(an AESV2 string whose last block does not end in valid PKCS#5 padding)
```

lamina keeps the data as stored (or unstripped) instead of failing, and raises the warning with the string's or stream's `object`.

## Data fields

None.
