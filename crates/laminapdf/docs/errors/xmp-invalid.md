---
title: "xmp-invalid"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/xmp-invalid/"
---

# xmp-invalid

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The document's XMP metadata packet (the catalog's `/Metadata` stream, ISO 32000-2 §14.3.2) isn't readable XMP, so it is treated as absent. The packet may not be well-formed XML. It may declare a DTD, which XMP doesn't allow, or refer to an entity other than XML's five predefined ones. It may use a prefix it never declares, nest elements more than 64 deep, have more than 256 namespace declarations in scope at once or give a tag more than 256 attributes, or have no `rdf:RDF` element.

Lamina PDF never expands entities, so a packet built to make a reader fetch files or fill its memory is refused.

## Why

A producer bug, a damaged or truncated stream, or a packet written by hand. A DTD in a packet is also the shape of an attack on XML readers.

## How to fix

Nothing to do when reading: the title, authors and dates still come from the Info dictionary. An edit that writes metadata asks before it replaces the packet, since whatever the packet held is lost when it is replaced. Regenerating the file with a current producer writes a valid packet.

## Example

```text
<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<!DOCTYPE x [ <!ENTITY e SYSTEM "file:///etc/passwd"> ]>
<x:xmpmeta xmlns:x="adobe:ns:meta/">…</x:xmpmeta>
```

The packet declares a DTD, so it is refused at the `<!DOCTYPE`; `data` is `{ "offset": 51, "reason": "dtd" }`.

## Data fields

- `offset`: about where reading stopped, in bytes of the stream's decoded data, so a UTF-16 packet counts two bytes a character. It is left out when no one place is at fault (`encoding` and `no-rdf`).
- `reason`: one of `encoding`, `dtd`, `entity`, `prefix`, `depth`, `limit`, `syntax` or `no-rdf`.

The `location` is the metadata stream's `object`, none for a direct stream; where in its data is the `offset` above.
