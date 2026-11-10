---
title: "strict"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/strict/"
---

# strict

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

Strict mode refused something the tolerant reader would have repaired.

## Why

The file needs repair; see the warning code in the data.

## How to fix

Open without strict mode, or fix the file.

## Example

```rust
let options = OpenOptions::new().with_strict(true);
let err = Document::open_bytes(bytes_with_a_broken_xref, options).unwrap_err();
// { "code": "strict", "data": { "warning": "xref-repaired" }, … }
```

## Data fields

`warning` (the warning code).
