---
title: "invalid-xmp-property"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/invalid-xmp-property/"
---

# invalid-xmp-property

- Type: error
- Kind: `usage` (the call was made wrongly)

## What happened

`setXmpProperty` or `removeXmpProperty` named a property, or gave a value, that can't be written to the document's XMP metadata. Nothing changed.

## Why

An XMP property is named by a namespace URI and a local name, and written with a prefix for its namespace. A value can't be written when:

- the namespace is empty, or is one the packet's own structure uses (RDF, `xml:`, `x:`, or the PDF/A extension schemas);
- the prefix or name isn't an XML name without a colon, or the prefix starts with `xml`;
- an array or a language alternative has no entries, an array mixes types or holds other arrays, or a language appears twice;
- the property is one the document information dictionary mirrors (`dc:title`, `dc:creator`, `pdf:Producer`, the dates, …) and the value has the wrong shape for it.

## How to fix

Change the part `reason` names. For a property the information dictionary mirrors, give the title and subject as text, the authors (`dc:creator`) as an array of text, and the dates as dates. To write a structure, or the title in other languages, replace the whole packet with `replaceXmpPacket`.

## Example

```ts
doc.setXmpProperty("http://purl.org/dc/elements/1.1/", "dc", "title", { type: "integer", value: 3 });
// invalid-xmp-property: reason "shape", the title is text
```

## Data fields

- `reason`: the part that can't be written: `namespace`, `prefix`, `name`, `value` (an empty, mixed or nested array, or a repeated language) or `shape` (the wrong shape for a mirrored property).
