//! Parses arbitrary bytes as a direct and an indirect object (`fuzz_entry::parse_object`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::fuzz_entry::parse_object(data));
