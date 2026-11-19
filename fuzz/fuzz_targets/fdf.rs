//! Reads arbitrary bytes as an FDF file: one that reads writes FDF that reads back to the same
//! comments, and writing that again gives the same bytes (`interchange_entry::fdf`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::interchange_entry::fdf(data));
