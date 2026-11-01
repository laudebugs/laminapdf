# `content-path-unpainted`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A content stream ended with a path that no painting operator ended; it was dropped.

## Why

A producer bug, or truncated content.

## How to fix

Nothing to do.

## Example

```text
0 0 m 100 100 l
```

The content stream ends after building a path without `S`, `f` or `n`: the path is dropped, as the spec intends.

## Data fields

- `offset`: the byte offset of the path's first segment in the decoded content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.

## For contributors

The decisions and todos behind this code:

- [ADR 0023](../decisions/0023-content-stream-reading.md)
