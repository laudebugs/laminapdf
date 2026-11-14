# `filter-characters-skipped`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

ASCIIHex or ASCII85 data held characters outside the filter's alphabet; they were skipped.

## Why

A damaged stream, or one written by a tool that put other characters in its text-encoded data.

## How to fix

Nothing, if the content looks right; if it doesn't, the stream is damaged and another copy of the file may read better.

## Example

```text
5 0 obj << /Filter /ASCIIHexDecode /Length 14 >> stream 48 6G5 6C6C6F> endstream
```

`G` isn't a hex digit. Lamina PDF skips it and decodes the rest, as other PDF readers do, so the stream reads as `Hello`. The warning carries the stream's `object`.

## Data fields

None.
