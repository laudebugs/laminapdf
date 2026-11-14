---
title: "stale-generation"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/stale-generation/"
---

# stale-generation

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A reference's generation number did not match its xref entry, so it resolved to `null` (ISO 32000-2 §7.3.10).

## Why

An editor deleted and reused an object number (bumping its generation) but left a reference with the old generation behind.

## How to fix

Nothing; the reference is treated as missing.

## Example

```text
/Contents 7 1 R      % but the xref entry for object 7 has generation 0
```

The reference resolves to `null` (ISO 32000-2 §7.3.10), as if object 7 were missing, and the warning carries `generation` 1.

## Data fields

`generation`.
