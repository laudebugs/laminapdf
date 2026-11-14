# Resolution: the nightly differential failed on files the new filters decode

[Diagnosis](detail.md).

## The fix

Three reading rules change, each recorded in ADR 0003 §1 after the owner chose it:

- **ASCIIHex and ASCII85.** A character outside the filter's alphabet, and an ASCII85 last group of one character, are skipped and the rest decodes, as pdf.js and poppler do. The reader raises the new warning `filter-characters-skipped`.
- **Dictionary keys.** Something other than a name where a key belongs (a number, string or keyword, a whole array or dictionary, a stray `]`, an unbalanced `)` or `>`) is skipped and the rest of the dictionary is read, as pdf.js and PDFium do, with the new warning `dictionary-key-skipped`. An object keyword (`obj`, `endobj`, `stream`, `endstream`) is still an error. An `/Encrypt` or catalog read by skipping counts as damaged, so repair still prefers an intact older definition. Strict mode refuses it.
- **PDFDocEncoding.** A text string's undefined byte keeps its own scalar (U+0000–U+00FF), as pdf.js, qpdf and pypdf read it, instead of U+FFFD.

An AES-256 file whose `/OE` or `/UE` is damaged now opens with a password that doesn't need it; it fails only when no password works without it.

The differential's exception files follow:

- `issue18529.pdf` is a `[[known]]` XMP entry under #527, with the other files ExifTool can't read.
- Four pages of `poppler-90-0-fuzzed.pdf` still differ, because Lamina PDF's content interpreter keeps reading past content errors that make pdf.js stop the page. They are a `[[known]]` text entry tracked by #597.
- The `[[known]]` entries of #37 are gone: both mutants open, the outline titles agree, and `issue22011.pdf` reads 57 pages as PDFium, qpdf and pypdf do. The pinned pdf.js 6.4.299 still reads 40, a bug it fixed in 6.5, so that file is an `[[exception]]` for pdf.js alone.
- Two PR-lane and 25 full-lane exceptions are no longer needed and are removed. Three full-lane mutants now read where pypdf fails, and are excused under the dictionary-key rule.

## How it was checked

- `crates/laminapdf-syntax/src/filters/text.rs`: `ascii_hex_skips_a_character_that_is_not_a_hex_digit_and_says_so`, `ascii85_skips_a_character_outside_its_alphabet_and_says_so`, `ascii85_drops_a_last_group_of_one_character_and_says_so`.
- `crates/laminapdf-syntax/src/text.rs`: `decode_text_string_keeps_an_undefined_pdfdoc_code_as_its_own_scalar`.
- `crates/laminapdf-syntax/src/parser.rs`: `parse_object_skips_what_is_not_a_name_where_a_key_belongs`, `an_object_keyword_where_a_key_belongs_still_fails`, `a_well_formed_dictionary_skips_no_keys`.
- `crates/laminapdf-cos/tests/warnings.rs` and `encryption.rs`: the warning on resolve, in a trailer and in strict mode; an intact older definition winning in repair; a short `/OE` and a short `/UE`.
- The `text-facts`, `xmp-facts` and `differential-facts` examples over the pinned corpus files: page 9 of the fuzzed file now starts as pdf.js's does, and the two outline-title files read as pdf.js does.
- `cargo xtask differential` and `cargo xtask differential --full` pass.

## Left for later

- Whether a page's text should stop at the first content error Lamina PDF can't repair, as pdf.js stops: #597.
- Bumping the pinned pdf.js to 6.5 or later, which would turn the `issue22011.pdf` exception into an agreement.
