---
title: "no-transaction"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/no-transaction/"
---

# no-transaction

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

`apply`, `commit` or `rollback` was called without an open transaction.

## Why

Calling code skipped `begin`.

## How to fix

Call `begin` first, or use `transact`.

## Example

```rust
// laminapdf_ops::Document, without begin():
ops.commit()?; // no-transaction
```

## Data fields

None.
