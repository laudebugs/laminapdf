---
title: "content-bad-operands"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/content-bad-operands/"
---

# content-bad-operands

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An operator had too few operands or operands of the wrong type (it was skipped), or extra operands (the last ones were used).

## Why

A producer bug, or damaged content.

## How to fix

Nothing to do.

## Example

```text
72 Td
```

`Td` takes two numbers; with one, the operator is skipped.

## Data fields

- `offset`: the operator's byte offset in the decoded content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
