---
title: "unexpected-eof"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/unexpected-eof/"
---

# unexpected-eof

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The input ended before a complete token or object was read.

## Why

Truncated files.

## How to fix

Use a complete copy of the file.

## Example

```text
1 0 obj << /Type /Catalog /Pages 2 0 R
```

The file ends inside a dictionary; the `location` is the file offset where reading stopped.

## Data fields

None. The `location` is a file `offset`, or `object` with `streamOffset` inside an object stream.
