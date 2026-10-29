//! Interprets fuzzed content streams: whole files page by page, anything else as the page
//! content of a small file with every kind of resource (`fuzz_entry::content`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| lamina_fuzz::content_entry::content(data));
