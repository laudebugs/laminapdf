# `template-too-large`

- Type: error
- Kind: `usage` (the caller passed something wrong)

## What happened

Filling a document template would have made more than 100,000 values, or nested more than 64 levels deep. Filling stopped, and nothing was laid out.

## Why

`$each` repeats its block once per item, and an `$each` inside another repeats it once per item of each. A few levels over long lists multiply into millions of blocks, which no document needs. So filling counts what it makes and stops at a limit far beyond a real document.

## How to fix

`data.limit` is the limit that filling reached. Repeat fewer items, or nest fewer `$each` placeholders inside each other. For a very long report, split it into several documents.

## Example

```rust
let document = flow::document::fill(&template, &data)?; // fails when nested $each placeholders multiply past the limit
```

## Data fields

- `limit`: the limit passed: 100,000 values filled in, or 64 levels of nesting.
