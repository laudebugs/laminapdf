//! Reads a fuzzed form script as a field's keystroke, format, validate and calculate action:
//! the recogniser, the native helpers and the inventory (`fuzz_entry::af_script`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::fuzz_entry::af_script(data));
