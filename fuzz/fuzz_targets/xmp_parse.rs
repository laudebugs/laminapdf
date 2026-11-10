//! Reads arbitrary bytes as an XMP packet: one that parses writes back unchanged, and after a
//! title, authors and date edit writes a packet that parses again with those values
//! (`fuzz_entry::xmp_parse`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::fuzz_entry::xmp_parse(data));
