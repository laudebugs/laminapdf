---
title: "nested-transaction"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/nested-transaction/"
---

# nested-transaction

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A transaction was started, or undo/redo called, while one is open.

## Why

Calling code opened a second transaction.

## How to fix

Commit or roll back first, or use a savepoint.

## Example

```ts
doc.transact((tx) => {
  doc.undo(); // nested-transaction: finish the transaction first
});
```

## Data fields

None.
