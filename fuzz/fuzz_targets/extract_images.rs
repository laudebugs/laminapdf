//! Extracts the images of fuzzed files, and of anything else as the stream of one image whose
//! filter, colour space, depth and size its first bytes choose (`extract_entry::extract_images`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::extract_entry::extract_images(data));
