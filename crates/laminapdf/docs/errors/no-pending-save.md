---
title: "no-pending-save"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/no-pending-save/"
---

# no-pending-save

- Type: error
- Kind: `state` (the call isn't valid in the current state; it may succeed after a state change)

## What happened

`rebase` was called, but no save is waiting for it.

## Why

Rebasing onto other bytes would not match the undo history, and unsaved changes would be lost.

## How to fix

Call `rebase` only after a save that left the document `SavedNotRebased`.

## Example

```rust
doc.rebase(bytes)?; // no-pending-save: no save is waiting
```

## Data fields

None.
