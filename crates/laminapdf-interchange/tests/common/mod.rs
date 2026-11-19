//! What the crate's integration tests share: the fuzz targets' entry points, which
//! `fuzz/src/lib.rs` includes too.
#![allow(
    unreachable_pub,
    reason = "the fuzz crate includes these files and reaches them as public items"
)]

pub mod fuzz_entry;
