//! Shared test code: the `extract` fuzz target's entry point, which `fuzz/src/lib.rs` includes.
#![allow(
    unreachable_pub,
    reason = "pub so the fuzz crate can call it; one test crate uses it"
)]

pub mod fuzz_entry;
