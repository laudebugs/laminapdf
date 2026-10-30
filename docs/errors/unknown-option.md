# `unknown-option`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An options record passed to `@lamina/wasm` (`OpenOptionsInit`, `SaveOptionsInit`, `BuildOptionsInit`, …) has a key lamina doesn't know.

## Why

Records refuse unknown keys instead of ignoring them, so a misspelt option fails loudly rather than silently doing nothing ([ADR 0028](../decisions/0028-facade-rules-jobs-and-cancellation.md) F6, [ADR 0026](../decisions/0026-error-and-diagnostic-contract.md) §1).

## How to fix

Remove the key or correct its spelling. `didYouMean` lists the known keys within two edits of it.

## Example

```js
WasmDocument.openBytes(bytes, { pasword: "secret" }); // throws { code: "unknown-option", didYouMean: ["password"], … }
```

## Data fields

- `argument`: the parameter whose record had the key (`options`, `metadata`, …).
- `option`: the key given (untrusted: caller text, never in the message).
