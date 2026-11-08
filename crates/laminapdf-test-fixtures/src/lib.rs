//! Test helpers shared by Lamina PDF's crates (docs/guidelines/code-organization.md §6): the
//! repository's `fixtures/` directory, hand-built classic files, a seeded PRNG, a byte-slice
//! read for `ByteSource` fakes, the `fixtures/codes.json` loader, and PNG bitmaps with the SSIM
//! comparison render goldens use.
//!
//! It depends on no Lamina PDF crate, so every crate can take it as a dev-dependency, unit
//! tests included, without a dependency cycle that would build a second copy of the crate
//! under test. A helper that needs a crate's own types, such as one that opens a document or
//! uses the spike-era writer, stays in that crate's `tests/common/`.
#![expect(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers: a panic here is a test failure"
)]

mod classic;
pub mod code_groups;
pub mod codes;
pub mod image;

use std::path::{Path, PathBuf};

pub use classic::{classic, classic_with_root, stream_body};

/// The repository's shared `fixtures/` directory.
#[must_use]
pub fn fixtures_dir() -> PathBuf {
    // This crate sits two levels below the repository root, like every crate.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// The bytes of `fixtures/<rel>`.
///
/// # Panics
/// When the file can't be read.
#[must_use]
pub fn fixture(rel: &str) -> Vec<u8> {
    let path = fixtures_dir().join(rel);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// `fixtures/<rel>` as UTF-8 text.
///
/// # Panics
/// When the file can't be read or isn't UTF-8.
#[must_use]
pub fn fixture_text(rel: &str) -> String {
    String::from_utf8(fixture(rel)).unwrap()
}

/// `fixtures/<rel>` parsed as JSON.
///
/// # Panics
/// When the file can't be read or isn't JSON.
#[must_use]
pub fn json(rel: &str) -> serde_json::Value {
    serde_json::from_slice(&fixture(rel)).unwrap()
}

/// Every `fixtures/corpus/*.pdf` as `(file name, bytes)`, sorted by name.
///
/// # Panics
/// When the directory or a file can't be read.
#[must_use]
pub fn corpus() -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(fixtures_dir().join("corpus"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

/// Copies `data[offset..]` into `buf` as far as both go and returns the count: the body of a
/// `ByteSource::read_at` fake over a byte slice. Returns 0 when `offset` is past the end.
#[must_use]
pub fn read_at(data: &[u8], offset: u64, buf: &mut [u8]) -> usize {
    let tail = usize::try_from(offset)
        .ok()
        .and_then(|start| data.get(start..))
        .unwrap_or_default();
    let n = tail.len().min(buf.len());
    buf[..n].copy_from_slice(&tail[..n]);
    n
}

/// `bytes` as lowercase hex, two digits each: how the tests print a SHA-256 digest.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// xorshift64*: a deterministic PRNG, so a failure found with a seed is reproducible.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// The generator for `seed` (0 counts as 1, which xorshift needs).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..n` (`0..1` when `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        let n = u64::try_from(n.max(1)).unwrap_or(u64::MAX);
        usize::try_from(self.next_u64() % n).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Rng, fixture, read_at};

    #[test]
    fn read_at_copies_what_is_there_and_stops_at_the_end() {
        let mut buf = [0u8; 4];
        let counts = [read_at(b"abcdef", 4, &mut buf), read_at(b"abcdef", 9, &mut buf)];
        assert_eq!((counts, &buf[..2]), ([2, 0], &b"ef"[..]));
    }

    #[test]
    fn rng_is_deterministic_for_a_seed() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        assert_eq!((a.next_u64(), a.below(10)), (b.next_u64(), b.below(10)));
    }

    #[test]
    fn fixture_reads_from_the_shared_directory() {
        assert!(fixture("codes.json").starts_with(b"{"));
    }
}
