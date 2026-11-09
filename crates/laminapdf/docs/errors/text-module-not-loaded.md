---
title: "text-module-not-loaded"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/text-module-not-loaded/"
---

# text-module-not-loaded

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

A font or text method (`Font` use, `surface.text`, `surface.textBox`, `builder.font`) was called on a `Builder` created while only the base WebAssembly module was loaded, or `new Builder({ text: true })` was called before the text module was loaded.

## Why

`@laminapdf/sdk` ships two modules: the base module (open, edit, save, vector and image pages) and the larger text module (all of the base plus fonts, shaping and text layout). A document that draws text is built entirely in the text module, and loading it is asynchronous, so it can't happen inside a synchronous drawing call. A builder created without the text module can't gain it later.

## How to fix

Load the text module before creating the builder that draws text: `await init({ text: true })`, or `await loadText()` after `init()`. Builders created after that use the text module.

## Example

```js
await init();
const b = new Builder();
b.page("A4").surface().text("Hi", { font: Font.standard("Helvetica"), size: 12, at: [72, 72] });
// throws { code: "text-module-not-loaded", … }
await loadText(); // then create a new Builder
```

## Data fields

None.
