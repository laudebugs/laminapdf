//! The differential suite's fixed-seed mutants of `fixtures/corpus` (ADR 0018 tier 6).
//!
//! Shared by `tests/differential.rs` (which compares lamina with the precomputed pypdf facts)
//! and `examples/differential-mutants.rs` (which writes the same files for
//! `cargo xtask differential --update`), so both always see the same bytes.

use super::Rng;

/// The seed of the mutant set; printed by the differential test.
pub const SEED: u64 = 0xB2C0_2026;

/// Mutants per corpus file.
pub const PER_FILE: usize = 4;

/// The mutants of one corpus file, named `<stem>-m<i>.pdf`, deterministic for its name.
pub fn of(name: &str, bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let stem = name.strip_suffix(".pdf").unwrap_or(name);
    let salt = name.bytes().fold(SEED, |acc, b| acc.rotate_left(5) ^ u64::from(b));
    let mut rng = Rng::new(salt);
    (0..PER_FILE)
        .map(|i| (format!("{stem}-m{i}.pdf"), mutate(&mut rng, bytes, i)))
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
