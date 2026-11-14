---
title: "stream-length-recovered"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/stream-length-recovered/"
---

# stream-length-recovered

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A stream's `/Length` was wrong or unresolvable; its end was found by scanning for `endstream`.

## Why

Writers that edit streams without updating `/Length`.

## How to fix

Nothing; re-saving writes the correct length.

## Example

```text
4 0 obj << /Length 10 >>
stream
BT /F1 12 Tf 72 720 Td (Hello) Tj ET
endstream
```

`/Length` says 10 bytes but the data is longer. Lamina PDF finds `endstream`, uses the real length and raises the warning with the stream's `object`.

## Data fields

None.
