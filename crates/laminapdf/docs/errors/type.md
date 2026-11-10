---
title: "type"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/type/"
---

# type

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An object had a different type than the structure requires.

## Why

Malformed input: an object of the wrong type where the structure requires a specific one.

## How to fix

Use another copy of the file.

## Example

```text
2 0 obj << /Type /Pages /Kids 5 >>
```

`/Kids` must be an array; reading the page tree fails with `type`, and `data` names what was expected and where (`expected`, `context`, from Lamina PDF's own vocabulary).

## Data fields

`expected`, `context` (from Lamina PDF's fixed list).
