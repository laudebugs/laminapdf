//! What reading a document by reopening its file costs, against reading one held handle: the
//! desktop app's `FileSource` reads the first way on Windows (ADR 0008 §11), the second elsewhere.
//!
//! ```sh
//! cargo xtask fixtures large-pdf
//! cargo run --release -p laminapdf-desktop-os --example reopen_cost -- target/fixtures/1gb.pdf
//! ```
//!
//! Reads the file in 256 KiB blocks, the engine's block size, both ways: every block in order (what
//! opening the 1 GB fixture reads) and 1,000 blocks spread through the file (a scroll). Each way
//! runs three times, alternating, and the fastest run counts. Prints a Markdown table.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    reason = "a measuring tool: a panic is a failed run, and its counts are small"
)]

use std::fs::File;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use laminapdf_desktop_os::file_identity;

const BLOCK: u64 = 256 * 1024;

/// `path` opened as `FileSource` opens it: read-only, and on Windows sharing read, write and delete.
fn open_shared(path: &Path) -> io::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        options.share_mode(0x1 | 0x2 | 0x4);
    }
    options.open(path)
}

fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    #[cfg(unix)]
    return std::os::unix::fs::FileExt::read_at(file, buf, offset);
    #[cfg(windows)]
    return std::os::windows::fs::FileExt::seek_read(file, buf, offset);
}

/// Reads `offsets` from one handle.
fn held(path: &Path, offsets: &[u64], buf: &mut [u8]) -> Duration {
    let start = Instant::now();
    let file = open_shared(path).unwrap();
    for &offset in offsets {
        read_at(&file, buf, offset).unwrap();
    }
    start.elapsed()
}

/// Reads `offsets` as `FileSource` does on Windows: open, check identity and length, read, close.
fn reopened(path: &Path, offsets: &[u64], buf: &mut [u8]) -> Duration {
    let first = open_shared(path).unwrap();
    let (identity, len) = (file_identity(&first).unwrap(), first.metadata().unwrap().len());
    drop(first);
    let start = Instant::now();
    for &offset in offsets {
        let file = open_shared(path).unwrap();
        assert!(file_identity(&file).unwrap() == identity && file.metadata().unwrap().len() == len);
        read_at(&file, buf, offset).unwrap();
    }
    start.elapsed()
}

fn fastest(runs: impl Iterator<Item = Duration>) -> Duration {
    runs.min().unwrap_or_default()
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn main() {
    let path = std::env::args_os().nth(1).expect("usage: reopen_cost <file>");
    let path = Path::new(&path);
    let len = std::fs::metadata(path).unwrap().len();
    let blocks = len.div_ceil(BLOCK);
    let in_order: Vec<u64> = (0..blocks).map(|b| b * BLOCK).collect();
    let spread: Vec<u64> = (0..1000_u64).map(|i| (i * 7919 % blocks) * BLOCK).collect();
    let mut buf = vec![0_u8; usize::try_from(BLOCK).unwrap()];
    println!("| Reads | Held handle | Reopened per read | Difference | Per read |");
    println!("| --- | --- | --- | --- | --- |");
    for (name, offsets) in [
        ("blocks in order", &in_order),
        ("blocks spread through the file", &spread),
    ] {
        let mut h = Vec::new();
        let mut r = Vec::new();
        for _ in 0..3 {
            h.push(held(path, offsets, &mut buf));
            r.push(reopened(path, offsets, &mut buf));
        }
        let (h, r) = (fastest(h.into_iter()), fastest(r.into_iter()));
        let per = (ms(r) - ms(h)) * 1000.0 / offsets.len() as f64;
        println!(
            "| {} {name} | {:.1} ms | {:.1} ms | {:+.1} % | {per:+.1} µs |",
            offsets.len(),
            ms(h),
            ms(r),
            (ms(r) / ms(h) - 1.0) * 100.0
        );
    }
}
