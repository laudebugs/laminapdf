---
title: "unsupported-security-handler"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/unsupported-security-handler/"
---

# unsupported-security-handler

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

The document uses a security handler Lamina PDF can't decrypt.

## Why

Not `/Standard`, an unknown `/V`/`/R`, or an unknown crypt filter method.

## How to fix

Use a tool that supports the handler.

## Example

```text
trailer << /Encrypt << /Filter /Adobe.PubSec /SubFilter /adbe.pkcs7.s5 … >> >>
```

A public-key (certificate) handler: Lamina PDF implements only `/Standard`, so `open` fails. `data.filter` holds the handler's name as read from the file.

## Data fields

`filter` (**untrusted**: read from the file; the message never includes it), `version`, `revision`.
