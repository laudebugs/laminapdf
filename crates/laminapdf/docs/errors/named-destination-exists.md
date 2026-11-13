---
title: "named-destination-exists"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/named-destination-exists/"
---

# named-destination-exists

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A rename gives a destination a name that another destination of the file already has.

## Why

Each name points at one place, so two destinations can't share it. Renaming onto a taken name would lose the destination that holds it.

## How to fix

Choose another name, or delete the other destination first if it is no longer needed.

## Example

```rust
// The file defines both chapter-2 and appendix.
doc.apply(vec![Op::from(RenameNamedDestination::new("chapter-2", "appendix"))])?; // named-destination-exists
```

## Data fields

None.
