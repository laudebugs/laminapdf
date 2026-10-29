//! Opens the PDF after eight control bytes, applies the ops and save options they name, saves
//! and reopens the output (`save_entry::save_roundtrip`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| lamina_fuzz::save_entry::save_roundtrip(data));
