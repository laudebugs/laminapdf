---
title: "saved-not-rebased"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/saved-not-rebased/"
---

# saved-not-rebased

- Type: error
- Kind: `state` (the call isn't valid in the current state; it may succeed after a state change)

## What happened

The document was saved to a sink Lamina PDF can't read back, and it waits for `rebase`: ops, undo, redo and saves are refused until then.

## Why

`save_full` to a write-only sink leaves the document `SavedNotRebased`: the bytes it would continue from are not available to it.

## How to fix

Call `rebase` with the saved bytes, then retry.

## Example

```rust
ops.save(&mut write_only_sink, &options)?;  // full rewrite; the document waits for rebase
ops.apply(op);                               // saved-not-rebased
ops.rebase(saved_bytes)?;                    // then edits work again
```

## Data fields

None.
