---
title: "xref-repaired"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/xref-repaired/"
---

# xref-repaired

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The stored cross-reference data was unusable and was rebuilt by scanning the file.

## Why

Edited, truncated or concatenated files often have stale offsets.

## How to fix

Nothing, if the result is right. Incremental saves refuse a repaired base (`repaired-base`); save in full instead.

## Example

```text
startxref
123456
%%EOF
```

`startxref` points past the end of a file that was truncated and re-padded. Lamina PDF scans the file for `n g obj` headers, rebuilds the table, raises `xref-repaired` and opens normally. `save(SaveMode::Incremental)` is then refused with [`repaired-base`](repaired-base.md); `SaveMode::Auto` rewrites the file.

## Data fields

None.
