---
title: "direct-page"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/direct-page/"
---

# direct-page

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

The page is a direct dictionary, which ops can't edit yet.

## Why

Non-standard files.

## How to fix

Rewrite the file so every page is an indirect object (any full rewrite by another tool does), or wait for Lamina PDF's page-structure ops.

## Example

```text
2 0 obj << /Type /Pages /Kids [<< /Type /Page /MediaBox [0 0 612 792] >>] /Count 1 >>
```

The page is written inline in `/Kids`. Reading works (its ID is a path ID such as `p2.0/Kids/0`), but a `rotatePage` op on it fails with `direct-page`.

## Data fields

None.
