---
title: "inline-image-unterminated"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/inline-image-unterminated/"
---

# inline-image-unterminated

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

No `EI` ends an inline image (neither at its declared or computed length nor by searching for it); the rest of the content was skipped.

## Why

Damaged or truncated content.

## How to fix

Check the page: everything after the inline image is missing.

## Example

```text
BI /W 10 /H 10 /CS /G /BPC 8 ID …(100 bytes, then the stream ends)…
```

No `EI` follows the image data, so the rest of the stream is skipped.

## Data fields

- `offset`: the `BI`'s byte offset in the decoded content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
