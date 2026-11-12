# Resolution: XMP packets other readers accept were read as malformed

[Diagnosis](detail.md).

## The fix

The XMP reader (`laminapdf_model::xmp`) now reads both kinds of damage:

- **The header.** Bytes that aren't UTF-8 are still refused, except inside the leading `<?xpacket … ?>` header, which is read with U+FFFD in their place. An unchanged packet still writes back byte for byte; an edited one is valid UTF-8.
- **Prefixes.** An undeclared prefix that is one of XMP's standard ones names its standard namespace: `dc`, `xmp`, `pdf`, `xmpMM`, `pdfaid`, `pdfuaid`, and the older `xap` and `xapMM`. Only the parser falls back. A declaration always wins, any other undeclared prefix is still refused, and the writer declares every prefix it writes.

The 13 files now parse, and their fields equal ExifTool's. The other 7 files' 8 differences are `[[known]]` entries in `fixtures/differential/xmp-exceptions.toml`, each with its reason, tracked by [#527](https://github.com/laudebugs/laminapdf/issues/527):

- 4 files ExifTool reads no XMP from: a wrong `startxref`, object streams over several revisions, and a `/Crypt` filter on encrypted metadata;
- a property written twice, where Lamina PDF reads the first and ExifTool the last;
- an empty `rdf:Seq`;
- two files whose dates aren't ISO 8601.

## How it was checked

- `crates/laminapdf-model/src/xmp/tests.rs`:
  - `a_header_whose_byte_order_mark_was_mangled_is_read` (and that the edited packet parses);
  - `bytes_that_are_not_utf8_outside_the_header_are_still_refused`;
  - `undeclared_standard_prefixes_name_their_standard_namespaces` (both pdf.js cases);
  - `an_undeclared_unknown_prefix_is_still_refused_and_a_declaration_wins`.
- The `xmp-facts` example over the 20 files reads, for each field the differential compared, the value ExifTool printed (or an excused difference).

## Left for later

- The undeclared-prefix fallback raises no warning: the packet is read as the writer meant it, and nothing else would tell the user.
- A packet whose bytes outside the header aren't UTF-8 is still malformed.
