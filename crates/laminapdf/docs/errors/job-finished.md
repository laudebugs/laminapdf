---
title: "job-finished"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/job-finished/"
---

# job-finished

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
