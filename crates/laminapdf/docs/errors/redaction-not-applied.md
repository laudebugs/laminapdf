---
title: "redaction-not-applied"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/redaction-not-applied/"
---

# redaction-not-applied

- Type: warning
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

A page has redaction marks (`Redact` annotations, ISO 32000-2 §12.5.6.23) that were never applied. A mark only records what someone meant to remove; until a tool applies it, the text, images and drawings under it are all still in the file.

## Why

Editors such as Acrobat redact in two steps: you mark what to remove, then you apply the marks. A file saved between the two steps looks redacted in some viewers, which draw the mark's overlay, but nothing has been taken out.

## How to fix

Apply the redactions in a tool that removes what they cover, then save the file as a full rewrite.

## Example

A page with two redaction marks over a name and an account number, saved before "Apply redactions" was chosen. `laminapdf check` reports `redaction-not-applied` for that page with `annotations` 2.

## Data fields

- `annotations`: how many redaction marks the page has.

The `location` has the page's `pageIndex`.
