//! Reads arbitrary bytes as an XFDF file: one that reads writes XFDF that reads back to the same
//! comments, and writing that again gives the same bytes (`interchange_entry::xfdf`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::interchange_entry::xfdf(data));
