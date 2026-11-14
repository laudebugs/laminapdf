---
title: "xrefstm-ignored"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/xrefstm-ignored/"
---

# xrefstm-ignored

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A hybrid file's `/XRefStm` could not be read and was ignored.

## Why

Hybrid files carry a classic xref table for old readers and an xref stream for objects in object streams. An editor that moved bytes without updating `/XRefStm`, or a damaged stream, leaves the stream unreadable.

## How to fix

Nothing; the classic table is used.

## Example

```text
trailer << /Size 40 /Root 1 0 R /XRefStm 99999 >>
```

A hybrid-reference file (ISO 32000-2 §7.5.8.4) points `/XRefStm` at an offset with no stream. Lamina PDF ignores it and reads the classic table; objects only the stream listed may be missing.

## Data fields

`offset` (where `/XRefStm` points).
