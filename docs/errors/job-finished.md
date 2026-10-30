# `job-finished`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

`step` was called on a job that had already returned its result, failed or been cancelled.

## Why

A job ends with its first `Step::Done`, or with its first error (errors end the job, [ADR 0028](../decisions/0028-facade-rules-jobs-and-cancellation.md) §3). A step loop that keeps going after either reaches this.

## How to fix

Stop stepping once a step returns `Done` or an error. To run the work again, create a new job.

## Example

```rust
let mut job = doc.save_job(&lamina::SaveOptions::new());
while let lamina::Step::Progress { .. } = job.step(&mut doc, lamina::Budget::UNLIMITED)? {}
assert_eq!(job.step(&mut doc, lamina::Budget::UNLIMITED).unwrap_err().code(), "job-finished");
```

## Data fields

None.
