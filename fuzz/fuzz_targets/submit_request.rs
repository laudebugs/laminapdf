//! Reads arbitrary bytes as a SubmitForm action's flags and address: a request that is built goes to
//! an `https:` host or a `mailto:` address, and one that isn't either is refused
//! (`interchange_entry::submit_request`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| laminapdf_fuzz::interchange_entry::submit_request(data));
