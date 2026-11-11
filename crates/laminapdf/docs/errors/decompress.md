---
title: "decompress"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/decompress/"
---

# decompress

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

Compressed data was corrupt and produced no output.

## Why

Damaged streams.

## How to fix

Use another copy of the file.

## Example

```text
5 0 obj << /Filter /FlateDecode /Length 40 >> stream (40 bytes that are not zlib data) endstream
```

Inflating fails before producing any output, so the stream has no data.

## Data fields

None.
