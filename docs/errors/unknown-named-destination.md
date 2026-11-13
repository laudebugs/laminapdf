# `unknown-named-destination`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A rename or delete names a destination that the file doesn't define.

## Why

The name is spelled differently, or an earlier op renamed or deleted it. Names are case-sensitive.

## How to fix

Call `namedDestinations()` (`named_destinations()` in Rust) and use one of its names. To create a name, use `setNamedDestination`.

## Example

```rust
doc.apply(vec![Op::from(DeleteNamedDestination::new("no-such-name"))])?; // unknown-named-destination
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3b](../todos/done/E3b-bookmarks-and-destinations.md): the bookmark editor and named destinations
