---
title: "reference-cycle"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/reference-cycle/"
---

# reference-cycle

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A reference chain or the page tree loops back on itself.

## Why

Damaged or hostile files.

## How to fix

Use another copy of the file.

## Example

```text
5 0 obj 6 0 R endobj
6 0 obj 5 0 R endobj
```

Resolving object 5 leads back to itself; the call fails instead of looping. A page tree whose `/Kids` lead back to an ancestor fails the same way.

## Data fields

None.
