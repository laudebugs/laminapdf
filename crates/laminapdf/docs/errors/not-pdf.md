---
title: "not-pdf"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/not-pdf/"
---

# not-pdf

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The input does not look like a PDF at all.

## Why

The bytes are empty, or no `n g obj` header appears anywhere in them: an HTML error page saved as `.pdf`, a different format, or the wrong buffer.

## How to fix

Check that you passed the right bytes.

## Example

```rust
let err = laminapdf::Document::open_bytes(b"<html>…</html>".to_vec(), Default::default()).unwrap_err();
assert_eq!(err.code(), "not-pdf");
```

## Data fields

None.
