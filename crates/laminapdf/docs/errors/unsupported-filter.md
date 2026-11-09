---
title: "unsupported-filter"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/unsupported-filter/"
---

# unsupported-filter

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

A stream uses a filter Lamina PDF doesn't implement; only decoding that stream fails.

## Why

Rare or proprietary filters.

## How to fix

Use a tool that supports the filter.

## Example

```text
5 0 obj << /Filter /JBIG2Decode /Length 2048 >> stream …
```

Decoding this stream fails; the rest of the document reads normally. `data.filter` is the filter's name as read from the file.

## Data fields

`filter` (**untrusted**: read from the file). The message never includes it.
