# `missing-header`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

No `%PDF-x.y` header was found in the first 1,024 bytes; parsing went ahead.

## Why

Files are often truncated at the front or prefixed with other data.

## How to fix

Nothing, if the document reads correctly. Open in strict mode to refuse such files, or strip whatever precedes `%PDF` before opening.

## Example

```text
offset 0      From: mailer@example.org
              …(1,200 bytes of mail headers and a MIME boundary)…
offset 1,212  %PDF-1.7
              1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
```

Lamina PDF looks for `%PDF-` in the first 1,024 bytes only. Here it starts at byte 1,212, so `Document::open_bytes` warns `missing-header` and reads the objects anyway. With `OpenOptions::default().with_strict(true)` the same open fails with [`strict`](strict.md).

## Data fields

None.
