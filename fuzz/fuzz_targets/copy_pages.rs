//! Copies the pages of a fuzzed PDF into an empty document and into `outline.pdf`, then saves
//! and reopens each (`save_entry::copy_pages`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::save_entry::copy_pages(data));
