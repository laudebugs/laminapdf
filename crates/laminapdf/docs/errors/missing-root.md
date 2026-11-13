---
title: "missing-root"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/missing-root/"
---

# missing-root

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

No document catalog (`/Root`) was found, even after repair.

## Why

The file is damaged beyond repair.

## How to fix

Use another copy of the file.

## Example

```text
trailer << /Size 3 >>      % no /Root, and no object with /Type /Catalog in the file
```

Repair found no catalog to use, so `open` fails.

## Data fields

None.
