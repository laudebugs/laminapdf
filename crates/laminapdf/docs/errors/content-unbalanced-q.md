---
title: "content-unbalanced-q"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/content-unbalanced-q/"
---

# content-unbalanced-q

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A `Q` had no matching `q` (it was ignored), or a content stream ended with saved graphics states still open (they were restored).

## Why

Producers that split `q`/`Q` pairs across streams, or damaged content.

## How to fix

Nothing to do; each stream balances on its own.

## Example

```text
q 1 0 0 1 72 72 cm 0 0 10 10 re f Q Q
```

The second `Q` has no matching `q` and is ignored.

## Data fields

- `offset`: the `Q`'s byte offset, or the end of the content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
