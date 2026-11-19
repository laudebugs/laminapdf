//! Reads arbitrary bytes as form data JSON: one that reads writes JSON that reads back to the same
//! fields, and writing that again gives the same bytes (`interchange_entry::form_json`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::interchange_entry::form_json(data));
