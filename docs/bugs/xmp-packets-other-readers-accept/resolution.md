# Resolution: XMP packets other readers accept were read as malformed

[Diagnosis](detail.md).

## The fix

The XMP reader (`laminapdf_model::xmp`) now reads both kinds of damage, and reports each:

- **The header.** Bytes that aren't UTF-8 are still refused, except inside the leading `<?xpacket … ?>` header, where each is read as `?`. One byte stands for one, so error offsets and the packet's length are those of the bytes given. An unchanged packet still writes back byte for byte; an edited one writes `?` in their place.
- **Prefixes.** An undeclared prefix that is one of XMP's standard ones names its standard namespace: `dc`, `xmp`, `pdf`, `xmpMM`, `pdfaid`, `pdfuaid`, and the older `xap` and `xapMM`. Only the parser falls back; a declaration always wins, and any other undeclared prefix is still refused. An edit declares such a prefix where it writes the property, on the replaced element or on the description of a replaced attribute, and a canonical rewrite declares it with the other namespaces. Properties an in-place edit leaves alone stay as the file wrote them.
- **The warning.** ADR 0003 says a repair is never silent. `Packet::repairs` says which damage a packet was read through, and `Document::xmp` raises the new warning `xmp-repaired` once for each, with `reason` `header-encoding` or `undeclared-prefix`. Strict mode refuses such a packet, as it refuses a malformed one.

The 13 files now parse, and their fields equal ExifTool's. The other 7 files' 8 differences are `[[known]]` entries in `fixtures/differential/xmp-exceptions.toml`, each with its reason, tracked by [#527](https://github.com/laudebugs/laminapdf/issues/527):

- 4 files ExifTool reads no XMP from: a wrong `startxref`, object streams over several revisions, and a `/Crypt` filter on encrypted metadata;
- a property written twice, where Lamina PDF reads the first and ExifTool the last;
- an empty `rdf:Seq`;
- two files whose dates aren't ISO 8601.

## How it was checked

- `crates/laminapdf-model/src/xmp/tests.rs`:
  - `a_header_whose_byte_order_mark_was_mangled_is_read` (and its edited packet parses, at the same length);
  - `offsets_after_a_repaired_header_are_those_of_the_bytes_given`;
  - `bytes_that_are_not_utf8_outside_the_header_are_still_refused`;
  - `undeclared_standard_prefixes_name_their_standard_namespaces` (both pdf.js cases);
  - `an_undeclared_unknown_prefix_is_still_refused_and_a_declaration_wins`;
  - `an_edit_declares_the_standard_prefix_the_packet_left_undeclared`, edited in place and rewritten canonically.
- `crates/laminapdf-model/tests/xmp.rs`: `a_repaired_packet_is_read_and_raises_xmp_repaired_for_each_repair` and `strict_mode_refuses_a_repaired_packet`.
- The `xmp-facts` example over the 20 files reads, for each field the differential compared, the value ExifTool printed (or an excused difference).

## Left for later

- A packet whose bytes outside the header aren't UTF-8 is still malformed.
- A structure's fields that use an undeclared standard prefix are read the same way, but an edit declares only the prefix of the property it writes.
