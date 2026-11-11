---
title: "password-required"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/password-required/"
---

# password-required

- Type: error
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

The document is encrypted with a non-empty user password and none was given.

## Why

Encrypted input.

## How to fix

Ask the user for the password and pass it in `OpenOptions` (`{ password }` in TS).

## Example

```ts
try {
  Document.open(bytes);
} catch (e) {
  if (isLaminaPdfError(e, "password-required")) Document.open(bytes, { password: await ask() });
}
```

## Data fields

None.
