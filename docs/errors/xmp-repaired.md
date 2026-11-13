# `xmp-repaired`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The document's XMP metadata has damage that ExifTool and pdf.js read past. Lamina PDF reads past it the same way, so it reads the values as the writer meant them.

## Why

`reason` says which damage:

- `header-encoding`: the packet's `<?xpacket … ?>` header holds bytes that aren't UTF-8, most often a mangled byte-order mark. XML doesn't read inside that header, so Lamina PDF reads each such byte as `?`.
- `undeclared-prefix`: the packet uses one of XMP's standard prefixes without declaring it, such as `dc:title` with no `xmlns:dc`. The prefixes read this way are `dc`, `xmp`, `pdf`, `xmpMM`, `pdfaid`, `pdfuaid`, and the older `xap` and `xapMM`, each in its standard namespace.

## How to fix

Nothing: Lamina PDF reads the file. In strict mode the warning is an error, so a strict reader refuses such a packet.

An edit repairs the prefixes it touches. A metadata edit declares the prefix of each property it writes, and a packet rewritten whole declares the prefix of every property. A prefix used only inside a property's value, such as an `xmp:` field of an `xmpMM:History` entry, stays as the file wrote it.

## Example

```ts
const doc = Document.open(bytes); // a packet that writes <dc:title> with no xmlns:dc
doc.metadata().title; // read as written
doc.warnings(); // xmp-repaired, reason "undeclared-prefix"
```

## Data fields

- `reason`: `header-encoding` or `undeclared-prefix`.

## For contributors

The decisions and todos behind this code:

- [`xmp-packets-other-readers-accept`](../bugs/xmp-packets-other-readers-accept/detail.md): the packets that needed it
- [ADR 0003](../decisions/0003-reader-tolerance-and-limits.md): every repair is a warning
