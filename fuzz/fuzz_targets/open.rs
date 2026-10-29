//! Opens arbitrary bytes (empty password, then each line of `fuzz/seeds/open/passwords.txt`)
//! and drives every read path and model view (`fuzz_entry::open`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| lamina_fuzz::fuzz_entry::open(data));
