# `invalid-xmp-packet`

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

`replaceXmpPacket` was given XML that isn't an XMP packet Lamina PDF can read, so the document's packet was left as it was.

## Why

The new packet is read before it replaces the old one, with the same rules as a packet in a file. It must be well-formed XML in UTF-8 or UTF-16 with an `rdf:RDF` element, every prefix declared. It must not declare a DTD or use entities other than XML's five predefined ones, and it must stay within the limits on depth and attributes.

## How to fix

Fix the XML near the `offset` in the error's data; `reason` says what is wrong. Then replace the packet again.

## Example

```ts
doc.replaceXmpPacket("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF>");
// invalid-xmp-packet: reason "prefix", the rdf: prefix isn't declared
```

## Data fields

- `reason`: what is wrong: `encoding` (neither UTF-8 nor UTF-16), `dtd`, `entity` (an undefined entity), `prefix` (an undeclared prefix), `depth` (elements nested more than 64 deep), `limit` (too many attributes or namespaces), `syntax` (not well-formed XML) or `no-rdf` (no `rdf:RDF` element).
- `offset`: the byte in the packet where the problem is, when there is one place.

## For contributors

The decisions and todos behind this code:

- [E3e](../todos/ongoing/E3e-xmp-editor.md): the general XMP editor
