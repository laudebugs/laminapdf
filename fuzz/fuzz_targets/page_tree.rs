//! Walks fuzzed page trees: whole files, or `/Pages` bodies wrapped in a small file
//! (`fuzz_entry::page_tree`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::fuzz_entry::page_tree(data));
