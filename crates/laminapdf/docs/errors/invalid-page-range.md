---
title: "invalid-page-range"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/invalid-page-range/"
---

# invalid-page-range

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A page range doesn't fit the grammar, or it names a page the document doesn't have, such as page 9 of a 5-page file.

## Why

Ranges count pages from 1, as people do. A range that can't be read, or that reaches past the last page, fails before anything is written, so a command never works on the wrong pages.

## How to fix

Write pages as `1-5`, `z` for the last page, `r2` for the second from the end, `x3` to leave page 3 out, and `:odd` or `:even` at the end for every other page of the result. `data.position` is the character where the range went wrong.

## Example

```console
$ laminapdf pages rotate scan.pdf +90:1-9 -o out.pdf
error[invalid-page-range]: the document has 5 pages
  = hint: write pages as 1-5, z for the last page, …
$ echo $?
2
```

## Data fields

- `range`: the range given (untrusted: caller text, never in the message).
- `position`: the 1-based character of `range` where it went wrong.
