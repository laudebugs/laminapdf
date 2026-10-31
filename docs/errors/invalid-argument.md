# `invalid-argument`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A value passed from JavaScript to `@laminapdf/wasm` is not of the type its declaration describes: a record with a field of the wrong type, a string that names no known value (a blend mode, a fill rule, a standard 14 font, an op), a path in the flat encoding cut short, a colour with the wrong number of components, an ID that isn't 32 hex digits.

## Why

The `.d.ts` types describe each argument, but JavaScript (and code written without the types) can pass anything. The binding checks each value as it converts it ([ADR 0016](../decisions/0016-package-design.md) §7, [ADR 0028](../decisions/0028-facade-rules-jobs-and-cancellation.md) F6).

## How to fix

Pass what the declaration says. When the value names an unknown string, `didYouMean` lists the closest allowed ones; `data.detail` says what the conversion found.

## Example

```js
page.blendMode("multiplly"); // throws { code: "invalid-argument", didYouMean: ["multiply"], … }
```

## Data fields

- `argument`: the parameter that was wrong (`options`, `path`, `mode`, …).
- `detail`: what the conversion said (untrusted: it may quote the value passed).
