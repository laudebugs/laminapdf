//! Reads arbitrary bytes as an op log and applies it to two corpus files: saved and reopened,
//! and undone op by op back to the unchanged base (`save_entry::ops_json`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| lamina_fuzz::save_entry::ops_json(data));
