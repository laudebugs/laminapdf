# `xmp-claim-read-only`

- Type: error
- Kind: `usage` (the call was made wrongly)

## What happened

An XMP edit tried to change the file's PDF/A or PDF/UA identification: the `pdfaid:` or `pdfuaid:` properties that say which standard the file claims to meet. Lamina PDF refused the edit, and nothing changed.

## Why

These properties are a promise that the whole file meets a standard. Only converting the file, or checking it against the standard, can make that promise. An edit to the metadata can't, so `setXmpProperty` and `removeXmpProperty` refuse these properties, and `replaceXmpPacket` refuses a packet whose `pdfaid:` or `pdfuaid:` properties differ from the file's.

## How to fix

Edit the other properties instead. When you replace the whole packet, copy the file's `pdfaid:` and `pdfuaid:` properties into the new packet unchanged, or leave them out of a file that has none.

## Example

```ts
doc.setXmpProperty("http://www.aiim.org/pdfa/ns/id/", "pdfaid", "part", { type: "integer", value: 3 });
// xmp-claim-read-only: the file's PDF/A part can't be edited
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3e](../todos/ongoing/E3e-xmp-editor.md): the general XMP editor
- [E3 plan §11](../plans/editor/e3-navigation-attachments-inspection.md#11-decisions-on-the-open-questions): why the identification is read-only
