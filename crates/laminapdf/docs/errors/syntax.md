---
title: "syntax"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/syntax/"
---

# syntax

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A token or structure did not match the PDF grammar.

## Why

Malformed input.

## How to fix

Use another copy of the file.

## Example

```text
1 0 obj << /Type /Catalog /Pages 2 0 R >> >> endobj
```

A stray `>>` where an object should end: parsing that object fails with `syntax` at its offset.

## Data fields

None. The `location` is a file `offset`, or `object` with `streamOffset` inside an object stream.
