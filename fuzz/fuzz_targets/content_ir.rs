//! The editable content IR (ADR 0022) on fuzzed content: written back unedited, edited, written
//! and read again, recorded and hit-tested (`ir_fuzz_entry::content_ir`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::ir_entry::ir_fuzz_entry::content_ir(data));
