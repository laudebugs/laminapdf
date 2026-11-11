//! Fills arbitrary JSON as a document template from arbitrary JSON data (the two separated by a
//! zero byte) and lays it out: a document or a coded error, never a panic
//! (`document_entry::document`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::document_entry::document(data));
