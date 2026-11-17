# Resolution: the inspection differential failed on its first nightly

[Diagnosis](detail.md).

## The fix

The inspector now reads font dictionaries as ISO 32000-2 and the text extraction do, the differential reads pdffonts' rows from their end, and the differences in what the tools report are excused by value, not by file.

- **Font names** (`laminapdf-model`'s `FontResource::name`): `/BaseFont` through a reference, else the font descriptor's `/FontName`, else, for a Type 3 font only, its `/Name`. The subset tag is read from that name. A simple font's `/Name` is a resource label, so it is never a name.
- **ToUnicode** (`FontResource::to_unicode`): only a `/ToUnicode` stream counts, as the text extraction reads it.
- **Encoding** (`FontResource::encoding`): a simple font's encoding dictionary with `/Differences` is `Custom`, one without is its `/BaseEncoding` (none when it names none); a stream is a Type 0 font's CMap (`Custom`) and declares nothing for a simple font. The façade's `FontSummary` docs say the same.
- **pdffonts' rows** (`tools/xtask/src/live/inspect.rs`): read from the end (the object ID, the three flags, the encoding word, then the longest of poppler's twelve type names), so a long or lossily decoded name keeps the other columns in place. The `sub` column is now pdffonts' own.
- **Exceptions by value** (`tools/xtask/src/live/inspect/differences.rs`): the comparison pairs objects by number and fonts by object (a direct font by type and name), and a `[[class]]` excuses one object or font by its `relation`, the `fields` that differ, the values each side holds and the fields that are `null`. Every class names its `rule`. A `[[known]]` entry excuses what classes leave of a fact in its files. The `inspect-facts` example adds `appearance_fonts`, the fonts reached only through annotations' appearances, so a class can match them.

`fixtures/differential/inspect-exceptions.toml` holds 16 classes and 6 `[[known]]` entries:

- **Fonts by value:** a TrueType font without `/Encoding` (§9.6.5.4), an encoding name §9.6.5.1 doesn't define, a font without `/Subtype`, a type poppler takes from the font program, a direct font's ToUnicode, every appearance state's fonts (§12.5.5), and poppler's own free text appearance.
- **Fonts in one file:** fonts only a Type 3 font's resources reach, direct pages, a page poppler refuses for its 32,768-entry limit, a file in BrotliDecode (not an ISO 32000-2 filter), and a Type 0 font whose descendant is `null`.
- **Objects by value:** a cross-reference stream no section lists, a referenced number no section defines, and the `/Type /Catalog` qpdf adds. In one file, the entries beyond a wrong `/Size`.
- **`[[known]]`, issue PENDING:** the six files whose damaged object Lamina PDF's reader reads as `null` where qpdf or poppler recovers it.

## How it was checked

- `crates/laminapdf-model/tests/fonts.rs`: `an_indirect_base_font_should_name_the_font`, `a_type3_font_should_take_its_descriptors_font_name`, `a_type3_font_without_a_descriptor_should_take_its_name_entry`, `a_simple_font_should_not_take_its_name_entry`, `a_tounicode_name_should_not_count_as_a_cmap`, `a_tounicode_stream_should_count_as_a_cmap`, `an_encoding_dictionary_without_differences_should_be_its_base_encoding`, `an_encoding_dictionary_with_differences_should_be_custom`, `a_simple_fonts_encoding_stream_should_declare_nothing`, `a_type0_fonts_cmap_stream_should_be_custom`.
- `tools/xtask/src/live/inspect.rs` and `inspect/differences.rs`: `pdffonts_rows_should_read_from_their_end`, `a_class_should_excuse_its_fonts_and_a_known_entry_the_rest`, `a_class_that_excused_nothing_should_be_stale`, `a_class_should_excuse_only_the_values_it_names`, `appearance_should_mark_the_fonts_lamina_reaches_through_appearances`, `null_should_match_only_null_fields`, `a_class_should_refuse_fields_without_differ`.
- `crates/laminapdf/tests/inspect_differential.rs` (the PR lane's committed facts) still passes.
- `cargo xtask differential --live --inspect` over the pinned corpus with qpdf 12.4.2 and pdffonts 26.10.0: fonts match on 1,945 of 2,131 files (1,899 before), and every difference is excused, with no stale entry.

## Left for later

- The reader's recovery of damaged objects (a stream whose `/Length` runs past `endstream`, a dictionary without `<<` or `>>`, a bare `"`), which the six `[[known]]` entries wait on. It needs its own issue and an ADR 0003 rule.
- Sharing one reading of `/Encoding` between the inspector and the text extraction, which now agree but each read the dictionary themselves.
