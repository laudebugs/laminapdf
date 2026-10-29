//! Tokenises arbitrary bytes until the end or the first error (`fuzz_entry::lex`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| lamina_fuzz::fuzz_entry::lex(data));
