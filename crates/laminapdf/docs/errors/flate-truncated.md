---
title: "flate-truncated"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/flate-truncated/"
---

# flate-truncated

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

Flate data was truncated or had a bad checksum; the partial output was kept.

## Why

Truncated downloads or damaged streams.

## How to fix

Nothing, if the content looks right; part of the stream may be missing.

## Example

```text
5 0 obj << /Filter /FlateDecode /Length 900 >> stream …(the zlib data stops early)… endstream
```

Lamina PDF keeps what inflated before the data ended or the Adler-32 checksum failed; the content may be cut short. The warning carries the stream's `object`.

## Data fields

None.
