---
title: "not-rebased"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/not-rebased/"
---

# not-rebased

- Type: error
- Kind: `state` (the call isn't valid in the current state; it may succeed after a state change)

## What happened

The save wrote its complete output, but rebasing the document onto it failed. The document is `SavedNotRebased`.

## Why

Reading the output back failed (for example a limit, or a source that can't deliver the bytes yet).

## How to fix

Retry `rebase` with the output the error carries (`saved.source()`; in the `laminapdf` façade, `Error::saved_output()` and `Document::rebase`).

## Example

```rust
match doc.save_with(&options, &cancel) {
    Err(e) if e.code() == "not-rebased" => doc.rebase(e.saved_output().unwrap())?,
    other => { other?; }
}
```

## Data fields

- `error`: the rebase's own error (its code, kind and location).
