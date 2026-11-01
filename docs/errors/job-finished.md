# `job-finished`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

`step` was called on a job that had already returned its result, failed or been cancelled.

## Why

A job ends with its first `Step::Done`, or with its first error: an error ends the job. A step loop that keeps going after either reaches this.

## How to fix

Stop stepping once a step returns `Done` or an error. To run the work again, create a new job.

## Example

```rust
let mut job = doc.save_job(&laminapdf::SaveOptions::new());
while let laminapdf::Step::Progress { .. } = job.step(&mut doc, laminapdf::Budget::UNLIMITED)? {}
assert_eq!(job.step(&mut doc, laminapdf::Budget::UNLIMITED).unwrap_err().code(), "job-finished");
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [ADR 0028](../decisions/0028-facade-rules-jobs-and-cancellation.md) §3
