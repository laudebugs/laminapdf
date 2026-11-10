---
title: "model-needs-host"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/model-needs-host/"
---

# model-needs-host

- Type: error
- Kind: `usage` (the call itself is wrong)

## What happened

An extraction in one call (`extract`) was given a model whose results only the host can give: a model named as the host's (`ModelSlot::Host`), or a plug-in model with no page renderer to draw the pages it reads.

## Why

A one-call extraction runs to the end without stopping, so it can only use models it can call itself, on pages its renderer draws. A host's model answers through the extraction job, which stops at each page that needs a result.

## How to fix

Step an extraction job and hand it each result: `request()` says which page and resolution it needs, `provide(result)` gives it back. Or give the plug-in model a page renderer, so the extraction calls it between pages.

## Example

```rust
let mut job = doc.extract_job(&options)?;
loop {
    if let laminapdf::Step::Done(extraction) = job.step(&doc, laminapdf::Budget::UNLIMITED)? {
        break extraction;
    }
    if let Some(request) = job.request().cloned() {
        job.provide(run_my_model(&request))?;
    }
}
```

## Data fields

None.
