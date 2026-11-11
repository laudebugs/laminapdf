---
title: "invalid-op-log"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/invalid-op-log/"
---

# invalid-op-log

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An op log does not parse: an unknown op or field, a bad value, or not JSON.

## Why

Hand-written or corrupted op logs.

## How to fix

Validate the log against the op schema; the samples in `fixtures/ops/v1/` show the format.

## Example

```rust
doc.apply_log(r#"{ "v": 1, "ops": [{ "op": "rotate" }] }"#)?;
// invalid-op-log: "rotate" is not an op (it is "rotatePage"); data.detail has serde's message
```

## Data fields

`detail` (**untrusted**: may quote the log; the message never includes it).
