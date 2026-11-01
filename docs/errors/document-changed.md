# `document-changed`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A job's `step` was given a document other than the one the job was created from, or that document changed since: a committed transaction, an undo, a redo, a save or a rebase. Every step checks the document's instance and revision before any work, and the job is now finished.

## Why

Jobs don't borrow the document, so other calls can run between two steps. A save or a read that went on after an edit would mix two states of the document, so the job stops instead.

## How to fix

Pass the document the job was created from, don't change it while the job runs, and create a new job after a change. A failed save left the document as it was.

## Example

```rust
let mut job = doc.save_job(&lamina::SaveOptions::new());
doc.apply(vec![lamina::Op::from(set_info)])?; // the revision moves on
assert_eq!(job.step(&mut doc, lamina::Budget::UNLIMITED).unwrap_err().code(), "document-changed");
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [ADR 0028](../decisions/0028-facade-rules-jobs-and-cancellation.md) §3
