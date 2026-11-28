//! Reads a fuzzed XFA part, as one stream and as an array of packets: the packets, and static
//! or dynamic (`fuzz_entry::xfa_packets`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::fuzz_entry::xfa_packets(data));
