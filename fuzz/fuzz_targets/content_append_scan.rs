//! The append API's `q`/`Q` scan and wrap (`laminapdf-content`'s `scan_balance` and
//! `PageContentWriter::wrap_existing`) on content streams split at zero bytes
//! (`append_entry::content_append_scan`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::append_entry::content_append_scan(data));
