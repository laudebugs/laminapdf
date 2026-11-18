//! Reads a fuzzed interactive form: the field walk, inheritance, the reconciliation with the
//! pages and the tab order (`fuzz_entry::form_tree`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::fuzz_entry::form_tree(data));
