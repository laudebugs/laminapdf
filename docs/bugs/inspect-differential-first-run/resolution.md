# Resolution: the inspection differential failed on its first nightly

[Diagnosis](detail.md).

## The fix

The inspector now reads font dictionaries as ISO 32000-2 and the text extraction do, the differential reads pdffonts' rows from their end, and the differences in what the tools report are excused by value, not by file.

- **Font names** (`laminapdf-model`'s `FontResource::name`): `/BaseFont` through a reference, else the font descriptor's `/FontName`, else, for a Type 3 font only, its `/Name`. The subset tag is read from that name. A simple font's `/Name` is a resource label, so it is never a name.
- **ToUnicode** (`FontResource::to_unicode`): only a `/ToUnicode` stream counts.
- **Encoding** (`FontResource::encoding`): a simple font's encoding dictionary with `/Differences` is `Custom`, one without is its `/BaseEncoding` (none when it names none); a stream is a Type 0 font's CMap (`Custom`) and declares nothing for a simple font. The façade's `FontSummary` and the WebAssembly binding's docs, which the SDK's types carry, say the same.
- **One reading for both.** `laminapdf-model`'s `SimpleEncoding::read` and `to_unicode_cmap` are now the only readers of a simple font's `/Encoding` and of `/ToUnicode`, for the inspector and for `laminapdf-text`'s extraction. Each caller passes its own way of resolving a reference. The extraction used to read an indirect `/BaseEncoding` as absent, so it fell back to StandardEncoding where the inspector reported WinAnsi.
- **pdffonts' rows** (`tools/xtask/src/live/inspect.rs`): read from the end (the object ID, the three flags, the encoding word, then the longest of poppler's twelve type names), so a long or lossily decoded name keeps the other columns in place. The `sub` column is now pdffonts' own.
- **Exceptions by value** (`tools/xtask/src/live/inspect/differences.rs`): the comparison pairs objects by number and fonts by object (a direct font by type and name), and a `[[class]]` excuses one object or font by its `relation`, the `fields` that differ, the values each side holds, the fields that are `null` on each side, and lamina's facts about the whole file. Every class names its `rule`. A `[[known]]` entry excuses what classes leave of a fact in its files, and a class counts as used only when the mismatch it matched is excused. The `inspect-facts` example adds `appearance_fonts`, the fonts reached only through annotations' appearances, and `annotations_without_appearance`, so classes can match them. A pdffonts row the parser can't read marks the reader as broken, so its font can't pass for one only lamina lists.

`fixtures/differential/inspect-exceptions.toml` holds 16 classes and 7 `[[known]]` entries:

- **Fonts by value:** a TrueType font without an `/Encoding` it can use (§9.6.5.4), a simple font's encoding name §9.6.5.1 doesn't define, a font without `/Subtype`, a type poppler takes from the font program, a direct font's ToUnicode, every appearance state's fonts (§12.5.5), and poppler's own appearance for a free text annotation without `/AP`.
- **Fonts in one file, matched by their values too:** fonts only a Type 3 font's resources reach, direct pages, a page poppler refuses for its 32,768-entry limit, and a Type 0 font whose descendant is `null`.
- **Objects by value:** a cross-reference stream no section lists, a referenced number no section defines, and the `/Type /Catalog` qpdf adds to a catalog without one or with a misspelt one. In one file, the entries beyond a wrong `/Size`.
- **`[[known]]`:** the six files whose damaged object Lamina PDF's reader reads as `null` where qpdf or poppler recovers it ([#722](https://github.com/laudebugs/laminapdf/issues/722)), and the BrotliDecode file, a support decision ([#724](https://github.com/laudebugs/laminapdf/issues/724)).

Two of the six break a rule Lamina PDF already has. ADR 0003 §1 says a wrong `/Length` is recovered by scanning for `endstream` (`stream-length-recovered`). Yet in `bug_972518.pdf` and `bad24.pdf`, object 4's `/Length` runs past its `endstream`, and the reader fails with `unexpected-eof`. #722 records that.

## How it was checked

- `crates/laminapdf-model/tests/fonts.rs`: `an_indirect_base_font_should_name_the_font`, `a_simple_font_without_base_font_should_take_its_descriptors_font_name`, `a_type0_font_without_base_font_should_take_its_descendants_font_name`, `an_encoding_dictionary_with_neither_entry_should_declare_nothing`, `a_type3_font_should_take_its_descriptors_font_name`, `a_type3_font_without_a_descriptor_should_take_its_name_entry`, `a_simple_font_should_not_take_its_name_entry`, `a_tounicode_name_should_not_count_as_a_cmap`, `a_tounicode_stream_should_count_as_a_cmap`, `an_encoding_dictionary_without_differences_should_be_its_base_encoding`, `an_encoding_dictionary_with_differences_should_be_custom`, `a_simple_fonts_encoding_stream_should_declare_nothing`, `a_type0_fonts_cmap_stream_should_be_custom`.
- `crates/laminapdf-text/tests/extract_fonts.rs`: `the_inspector_and_extraction_should_read_an_indirect_base_encoding_alike`.
- `tools/xtask/src/live/inspect.rs` and `inspect/differences.rs`: `pdffonts_rows_should_read_from_their_end`, `pdffonts_rows_should_read_an_empty_encoding_and_no_name`, `pdffonts_rows_that_cant_be_read_should_be_counted`, `lamina_file_should_match_a_fact_of_the_whole_file`, `a_class_should_excuse_its_fonts_and_a_known_entry_the_rest`, `a_class_that_excused_nothing_should_be_stale`, `a_class_should_excuse_only_the_values_it_names`, `appearance_should_mark_the_fonts_lamina_reaches_through_appearances`, `null_should_match_only_null_fields`, `a_class_should_refuse_fields_without_differ`.
- `crates/laminapdf/tests/inspect_differential.rs` (the PR lane's committed facts) still passes.
- `cargo xtask differential --live --inspect` over the pinned corpus with qpdf 12.4.2 and pdffonts 26.10.0: fonts match on 1,945 of 2,131 files (1,899 before), and every difference is excused, with no stale entry.

## Left for later

- The reader's recovery of damaged objects, which six `[[known]]` entries wait on (#722): a stream whose `/Length` runs past `endstream` (ADR 0003 §1's own rule), and a dictionary without `<<` or `>>` or with a bare `"`, which need an ADR 0003 rule.
- Whether to support BrotliDecode (#724).
- A Type 0 font's `/Encoding`, which the inspector and the extraction still each read themselves.
