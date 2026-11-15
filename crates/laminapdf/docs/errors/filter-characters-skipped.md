---
title: "filter-characters-skipped"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/filter-characters-skipped/"
---

# filter-characters-skipped

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A stream stored as text, as hex digits (`ASCIIHexDecode`) or as ASCII85 characters (`ASCII85Decode`), held characters that its encoding doesn't use. Lamina PDF skipped them.

## Why

The stream is damaged, or the tool that wrote it put other characters in its encoded data.

## How to fix

Nothing, if the content looks right; if it doesn't, the stream is damaged and another copy of the file may read better. In strict mode the warning is an error, so the file doesn't open.

## Example

```text
5 0 obj << /Filter /ASCIIHexDecode /Length 14 >> stream 48 6G5 6C6C6F> endstream
```

`G` isn't a hex digit. Lamina PDF skips it and decodes the rest, as other PDF readers do, so the stream reads as `Hello`. The warning carries the stream's `object`.

## Data fields

None.
