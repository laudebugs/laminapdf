//! Inflates a stream and runs the filter pipeline with predictor parameters from the input's
//! last four bytes (`fuzz_entry::flate`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::fuzz_entry::flate(data));
