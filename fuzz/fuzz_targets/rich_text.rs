//! Reads arbitrary bytes as a FreeText annotation's `/RC` rich text: the reader never panics, and
//! the subset writes what it read and reads it back unchanged (`rich_text_entry::rich_text`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::rich_text_entry::rich_text(data));
