# `view-not-finite`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A destination's view has a coordinate or zoom that isn't a finite number.

## Why

A PDF file can only hold finite numbers, so a view of infinity or NaN can't be written.

## How to fix

Pass finite numbers, in the page's user space, or `null` for a coordinate the viewer should keep as it is.

## Example

```rust
let view = DestinationView::Xyz { left: Some(f64::NAN), top: None, zoom: None };
doc.apply(vec![Op::from(SetNamedDestination::new("here", ExplicitDestination::new(page, view)))])?; // view-not-finite
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3b](../todos/done/E3b-bookmarks-and-destinations.md): the bookmark editor and named destinations
