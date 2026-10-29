//! The differential suite's fixed-seed mutants of `fixtures/corpus` (ADR 0018 tier 6).
//!
//! Two sets: [`all`], four per corpus file, which the fast lane checks, and [`full`], the
//! full lane's larger set (`cargo xtask differential --mutants 500`). Shared by
//! `tests/differential.rs` (which compares lamina with the precomputed pypdf facts) and
//! `examples/differential-inputs.rs` (which writes the same files for
//! `cargo xtask differential --update`), so both always see the same bytes.

use super::Rng;

/// The seed of the mutant set; printed by the differential test.
pub const SEED: u64 = 0xB2C0_2026;

/// The seed of the full lane's mutant set ([`full`]), independent of [`SEED`] so the two
/// sets don't share their first mutants.
pub const FULL_SEED: u64 = 0x0F04_2026;

/// Mutants per corpus file in the fast set.
pub const PER_FILE: usize = 4;

/// The fast set's mutants of one corpus file, named `<stem>-m<i>.pdf`, deterministic for its name.
pub fn of(name: &str, bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    series(name, bytes, SEED, 'm', PER_FILE)
}

/// `count` mutants of one file, named `<stem>-<tag><i>.pdf`; the RNG is salted with the
/// file name, so a file's mutants don't change when other corpus files come or go.
fn series(name: &str, bytes: &[u8], seed: u64, tag: char, count: usize) -> Vec<(String, Vec<u8>)> {
    let stem = name.strip_suffix(".pdf").unwrap_or(name);
    let salt = name.bytes().fold(seed, |acc, b| acc.rotate_left(5) ^ u64::from(b));
    let mut rng = Rng::new(salt);
    (0..count)
        .map(|i| (format!("{stem}-{tag}{i}.pdf"), mutate(&mut rng, bytes, i)))
        .collect()
}

/// One mutation per index: truncate, flip bytes, delete a range, duplicate a range.
fn mutate(rng: &mut Rng, bytes: &[u8], kind: usize) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let len = out.len().max(1);
    match kind % 4 {
        0 => out.truncate(len / 2 + rng.below(len / 2)),
        1 => {
            for _ in 0..=rng.below(4) {
                let at = rng.below(out.len());
                if let Some(b) = out.get_mut(at) {
                    *b ^= u8::try_from(1 + rng.below(255)).unwrap();
                }
            }
        }
        2 => {
            let at = rng.below(out.len());
            let n = 1 + rng.below(16);
            out.drain(at..(at + n).min(out.len()));
        }
        _ => {
            let at = rng.below(out.len());
            let n = 1 + rng.below(32);
            let copy: Vec<u8> = out.get(at..(at + n).min(out.len())).unwrap().to_vec();
            let insert = rng.below(out.len());
            out.splice(insert..insert, copy);
        }
    }
    out
}

/// Every mutant of every corpus file, sorted by name.
pub fn all() -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<_> = super::corpus()
        .iter()
        .flat_map(|(name, bytes)| of(name, bytes))
        .collect();
    out.sort();
    out
}

/// The full lane's `n` mutants, from [`FULL_SEED`], named `<stem>-f<i>.pdf` and sorted by
/// name: `n` split evenly over the corpus files, the first `n % files` (by name) taking one
/// more. Every file cycles through the four mutation kinds.
pub fn full(n: usize) -> Vec<(String, Vec<u8>)> {
    let corpus = super::corpus();
    let files = corpus.len().max(1);
    let mut out: Vec<_> = corpus
        .iter()
        .enumerate()
        .flat_map(|(i, (name, bytes))| {
            let count = n / files + usize::from(i < n % files);
            series(name, bytes, FULL_SEED, 'f', count)
        })
        .collect();
    out.sort();
    out
}
