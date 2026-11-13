//! The annotation views and the annotation ops (`UpdateAnnot`, `DeleteAnnot`) on any input
//! (`annot_entry::annots`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::annot_entry::annots(data));
