---
title: "unknown-option"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/unknown-option/"
---

# unknown-option

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An options record passed to `@laminapdf/wasm` (`OpenOptionsInit`, `SaveOptionsInit`, `BuildOptionsInit`, …) has a key Lamina PDF doesn't know.

## Why

Records refuse unknown keys instead of ignoring them, so a misspelt option fails loudly rather than silently doing nothing.

## How to fix

Remove the key or correct its spelling. `didYouMean` lists the known keys within two edits of it.

## Example

```js
WasmDocument.openBytes(bytes, { pasword: "secret" }); // throws { code: "unknown-option", didYouMean: ["password"], … }
```

## Data fields

- `argument`: the parameter whose record had the key (`options`, `metadata`, …).
- `option`: the key given (untrusted: caller text, never in the message).
