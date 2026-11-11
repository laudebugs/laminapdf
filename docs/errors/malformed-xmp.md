# `malformed-xmp`

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A metadata edit (`setMetadata` or `removeMetadata`) would have to write the document's XMP metadata packet, but the packet in the file can't be read (it raised `xmp-invalid` when the document was opened). Writing a new packet would throw away everything the old one held, so the edit was refused and nothing changed.

## Why

The packet may not be well-formed XML, may declare a DTD, or may be damaged. Lamina PDF reads such a packet as absent, and only replaces it when you say so.

## How to fix

Set `replaceMalformed` on the edit to replace the packet with a new one that holds the fields you set. Or leave the metadata as it is: the title, authors and dates still read from the Info dictionary.

## Example

```ts
doc.setMetadata({ title: "Minutes" }); // malformed-xmp: the packet can't be read
doc.setMetadata({ title: "Minutes" }, { replaceMalformed: true }); // replaces it
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E1c](../todos/ongoing/E1c-metadata-ops.md): the metadata ops
