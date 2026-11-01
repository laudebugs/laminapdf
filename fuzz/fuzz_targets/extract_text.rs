//! Extracts the text of fuzzed files page by page, and of anything else as the content and the
//! ToUnicode CMap of a small file with a simple and a Type 0 font (`text_entry::extract_text`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::text_entry::extract_text(data));
