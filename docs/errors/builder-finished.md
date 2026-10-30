# `builder-finished`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A `WasmBuilder` method was called after the builder's `finish`, or a `WasmPageBuilder` was drawn on or finished again after its own `finish` (or after its builder's).

## Why

`finish` hands the work over: a builder's becomes a `WasmDocument`, a page's is written into its builder. Neither holds anything to draw on afterwards (the binding's owned pages, [ADR 0016](../decisions/0016-package-design.md) §7).

## How to fix

Draw everything on a page before finishing it, and finish every page before the builder. To build more, create a new page or builder.

## Example

```js
const b = new WasmBuilder(undefined);
const page = b.page(595, 842);
page.finish();
page.save(); // throws { code: "builder-finished", … }
```

## Data fields

- `argument`: always `builder`.
