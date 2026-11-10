---
title: "objstm-index-mismatch"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/objstm-index-mismatch/"
---

# objstm-index-mismatch

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An object stream's index was stale; the object was found by searching.

## Why

Object streams rewritten by a tool that did not update the index at their start.

## How to fix

Nothing.

## Example

```text
12 0 obj << /Type /ObjStm /N 3 /First 18 >> stream
20 0 21 14 22 31
…
```

The index says object 21 starts at offset 14 of the stream, but it starts elsewhere. Lamina PDF searches the stream for it, uses what it finds and raises the warning.

## Data fields

None.
