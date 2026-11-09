---
title: "invalid-rotate"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/invalid-rotate/"
---

# invalid-rotate

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A page's `/Rotate` is not a multiple of 90; it reads as 0.

## Why

A writer bug, or a viewer setting stored as an arbitrary angle.

## How to fix

Nothing to do if the page displays upright. Otherwise set the rotation you want with the `rotatePage` op.

## Example

```text
3 0 obj << /Type /Page /Rotate 45 … >>
```

`/Rotate` must be a multiple of 90 (ISO 32000-2 §7.7.3.3); the page reads as unrotated.

## Data fields

None.
