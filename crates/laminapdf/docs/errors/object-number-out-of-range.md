---
title: "object-number-out-of-range"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/object-number-out-of-range/"
---

# object-number-out-of-range

- Type: warning
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

An xref entry above `Limits::max_object_number` was dropped.

## Why

Corrupt or hostile xref data.

## How to fix

Raise `max_object_number` only for trusted input.

## Example

```text
xref
0 1
0000000000 65535 f
9999999 1
0000000015 00000 n
```

Object 9,999,999 is above `Limits::max_object_number`; its entry is dropped so a hostile xref can't make Lamina PDF allocate for it. References to it resolve to `null`.

## Data fields

None.
