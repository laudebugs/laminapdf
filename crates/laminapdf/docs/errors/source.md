---
title: "source"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/source/"
---

# source

- Type: error
- Kind: `io` (the host failed to supply bytes; often transient)

## What happened

The `ByteSource` failed to read.

## Why

A file-system or network error in the host.

## How to fix

Retry, or check the source.

## Example

A `ByteSource` reading from a network answered with an error for bytes 4096..+65536; the call fails with `source` and the same call can be retried once the connection is back.

## Data fields

None.
