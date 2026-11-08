//! The flow-layout spike (T4a): the same documents laid out three ways. Throwaway code.

pub mod inputs;
pub mod probe;
#[cfg(feature = "own")]
pub mod own;
#[cfg(feature = "frames")]
pub mod typst_frames;
#[cfg(feature = "typst")]
pub mod typst_pdf;
#[cfg(feature = "typst")]
pub mod typst_world;

use std::path::PathBuf;

/// The repository's root, from this crate's manifest directory.
pub fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The font files the spike uses, read from `fixtures/fonts/`.
pub fn fonts() -> Vec<Vec<u8>> {
    ["NotoSans-Regular.ttf", "NotoSans-RegularBold.ttc", "NotoSansArabic-Regular.ttf", "NotoSansDevanagari-Regular.ttf"]
        .iter()
        .map(|name| std::fs::read(repo().join("fixtures/fonts").join(name)).unwrap_or_else(|e| panic!("{name}: {e}")))
        .collect()
}
