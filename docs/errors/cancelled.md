# `cancelled`

- Type: error
- Kind: `cancelled` (a `CancelToken` stopped the work)

## What happened

The `CancelToken` passed to a job, or to a run-to-completion call such as `Document::save`, was cancelled. The call returned at its next check: the start of a step, a budget check, or the next object or page of a long loop.

## Why

The caller (or a host on its behalf: an `AbortSignal`, Ctrl-C, a task cancellation) asked for the work to stop.

## How to fix

Nothing: this is the outcome that was asked for. The job is finished, a cancelled save left the document as it was (`Dirty`, nothing written), and a cancelled read returned no partial result. Start a new job with a fresh token to try again.

## Example

```rust
let cancel = laminapdf::CancelToken::new();
cancel.cancel();
let err = doc.save_with(&laminapdf::SaveOptions::new(), &cancel).unwrap_err();
assert_eq!(err.code(), "cancelled");
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [ADR 0028](../decisions/0028-facade-rules-jobs-and-cancellation.md) §5
