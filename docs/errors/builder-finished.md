# `builder-finished`

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A `WasmBuilder` method, or `finish` on one of its `WasmPageBuilder`s, was called after the builder's `finish`.

## Why

`finish` writes the document and hands it over as a `WasmDocument`; the builder holds nothing afterwards. A page started before `finish` and finished after it has no builder left to write into (the binding's owned pages, [ADR 0016](../decisions/0016-package-design.md) §7).

## How to fix

Finish every page before the builder. To build another document, create a new builder.

## Example

```js
const b = new WasmBuilder(undefined);
const page = b.page(595, 842);
b.finish();
page.finish(); // throws { code: "builder-finished", … }
```

## Data fields

- `argument`: always `builder`.
