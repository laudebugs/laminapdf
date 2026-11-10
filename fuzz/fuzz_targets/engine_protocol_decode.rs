//! Reads arbitrary bytes as the frames between the desktop app's main process and its engine,
//! in both directions: each frame decodes or fails with a protocol error, and a decoded message
//! encodes again; never a panic (`protocol_entry::engine_protocol_decode`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::protocol_entry::engine_protocol_decode(data));
