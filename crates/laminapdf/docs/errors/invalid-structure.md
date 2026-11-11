---
title: "invalid-structure"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/invalid-structure/"
---

# invalid-structure

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An object stream, xref section or filter parameter was malformed or out of range.

## Why

Malformed input: an object stream, xref section or filter parameter out of range.

## How to fix

Use another copy of the file.

## Example

```text
12 0 obj << /Type /ObjStm /N 3 /First -4 >>
```

A negative `/First` makes the object stream unusable; `data.structure` says which structure was malformed.

## Data fields

`structure`.
