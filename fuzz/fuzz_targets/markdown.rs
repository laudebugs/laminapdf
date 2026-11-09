//! Lays arbitrary text out as Markdown, in the standard 14 fonts and in Noto Sans with a
//! contents page and an image: a document or a coded error, never a panic
//! (`markdown_entry::markdown`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::markdown_entry::markdown(data));
