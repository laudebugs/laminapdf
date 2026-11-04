//! Extracts fuzzed files as blocks in reading order, by layout and by content order
//! (`extract_entry::extract`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::extract_entry::extract(data));
