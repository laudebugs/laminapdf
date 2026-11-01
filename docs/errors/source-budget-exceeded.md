# `source-budget-exceeded`

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

A sparse source (`WasmSparseSource` in `@laminapdf/wasm`, or a `@laminapdf/sdk` range document) was handed bytes that would make it hold more than the byte budget it was created with. The bytes were not added.

## Why

A range source keeps every byte it has fetched: the document may already have read them, so nothing is evicted. The budget bounds how much of a remote or lazily read file one document may hold in memory.

## How to fix

Create the source with a larger budget, or fetch less per round: smaller blocks and no readahead. A file that must be read whole (a full rewrite, or a repaired file with no usable xref) needs a budget of at least its size.

## Example

```js
const source = new WasmSparseSource(10_000_000, 1_000_000);
source.supply(0, new Uint8Array(2_000_000)); // throws { code: "source-budget-exceeded", … }
```

## Data fields

- `argument`: `bytes`.
- `detail`: how many bytes the source would hold and its budget (Lamina PDF's own numbers).

## For contributors

The decisions and todos behind this code:

- [L2c](../todos/done/L2c-range-byte-source.md): the range byte source
- [ADR 0005](../decisions/0005-document-model.md) §2
