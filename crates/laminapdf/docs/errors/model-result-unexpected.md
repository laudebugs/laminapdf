---
title: "model-result-unexpected"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/model-result-unexpected/"
---

# model-result-unexpected

- Type: error
- Kind: `usage` (the call itself is wrong)

## What happened

A model result was handed to an extraction job (`provide`) that wasn't waiting for one, or that was waiting for the other model's result: layout regions when it asked for OCR words, or the reverse.

## Why

A job whose models the host runs stops at each page that needs a result and says which one (`request()`). It takes exactly that result and nothing else, so results can't land on the wrong page.

## How to fix

Call `provide` only while `request()` returns a request, with the result its kind asks for: layout regions for a layout request, OCR words for an OCR request.

## Example

```rust
while let laminapdf::Step::Progress { .. } = job.step(&doc, laminapdf::Budget::UNLIMITED)? {
    if let Some(request) = job.request().cloned() {
        job.provide(run_my_model(&request))?;
    }
}
```

## Data fields

None.
