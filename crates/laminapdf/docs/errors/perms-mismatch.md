---
title: "perms-mismatch"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/perms-mismatch/"
---

# perms-mismatch

- Type: warning
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

An R5/R6 document's `/Perms` does not match its `/P` and `/EncryptMetadata`.

## Why

`/P` may have been tampered with (ISO 32000-2 Algorithm 13).

## How to fix

Treat the permissions as untrusted.

## Example

```text
/Encrypt << /V 5 /R 6 /P -3904 /Perms <…> … >>   % /Perms decrypts to a different /P
```

The document opens, but `Document::permissions()` comes from a `/P` that the encrypted `/Perms` contradicts.

## Data fields

None.
