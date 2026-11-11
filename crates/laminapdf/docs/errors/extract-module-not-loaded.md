---
title: "extract-module-not-loaded"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/extract-module-not-loaded/"
---

# extract-module-not-loaded

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

`page.text()` of `@laminapdf/sdk` was called on a page of a document opened in the base module, which has no text extraction.

## Why

`@laminapdf/sdk` ships three WebAssembly modules: base (open, read, edit, save), extract (base plus text extraction) and text (base plus fonts, shaping and extraction). A document lives in the module it was opened in, and opening uses the richest one loaded at that moment: text, then extract, then base. A document opened before the extract or text module was loaded stays in the base module.

## How to fix

Load the extract module (about half the size of the text module) before opening the document: `await init({ extract: true })` or `await loadExtract()`. A document already open in the base module can be moved by saving it and opening the bytes again.

## Example

```js
await init();
const doc = Document.open(bytes);
doc.page(0).text(); // throws { code: "extract-module-not-loaded", … }

await loadExtract();
Document.open(bytes).page(0).text().text(); // works
```

## Data fields

None.
