---
title: "outline-item-skipped"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/outline-item-skipped/"
---

# outline-item-skipped

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An outline item was missing, not a dictionary or too deep; it and its subtree were skipped.

## Why

Damaged outlines.

## How to fix

Nothing to do; the skipped item and its children are missing from the outline.

## Example

```text
10 0 obj << /Type /Outlines /First 11 0 R >>   % object 11 is missing
```

The first item can't be read, so it and everything under it are left out of `outline()`.

## Data fields

None.
