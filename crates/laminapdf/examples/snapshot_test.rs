//! Checks that a PDF your code makes is byte for byte the reference PDF you approved.
//!
//! Build the document with `test_options()`, then assert on the answer in a test:
//!
//! ```ignore
//! let bytes = invoice(&order, test_options()?)?;
//! assert_eq!(compare_with_reference(&bytes, Path::new("references/invoice.pdf"), false)?, Outcome::Same);
//! ```
//!
//! The first run fails with `Created` and writes the reference: open it, and commit it once it
//! looks right. When the bytes change later, the new PDF is written beside the reference for
//! review; pass `update: true` to accept it. Run on its own, the example compares two files:
//!
//! ```sh
//! cargo run -p laminapdf --example snapshot_test -- new.pdf reference.pdf
//! ```

use std::error::Error;
use std::path::{Path, PathBuf};

use laminapdf::BuildOptions;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(new), Some(reference)) = (args.next(), args.next()) else {
        return Err("usage: snapshot_test <new.pdf> <reference.pdf>".into());
    };
    let outcome = compare_with_reference(&std::fs::read(new)?, Path::new(&reference), false)?;
    println!("{outcome:?}");
    Ok(())
}

/// What a test passes in place of the clock and a random file ID. With these fixed, the same code
/// and data give the same bytes on every run, machine and runtime.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "your tests call it; `main` only compares two files")
)]
pub(crate) fn test_options() -> Result<BuildOptions, Box<dyn Error>> {
    /// The file ID: any 16 bytes, as long as they stay the same.
    const FILE_ID: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
    ];
    /// The Info dictionary's `/Producer`: your app's name, without a version that changes.
    const PRODUCER: &str = "my-app tests";
    /// The Info dictionary's `/CreationDate`, in PDF date form.
    const CREATED: &str = "D:20260101000000Z";
    Ok(BuildOptions::new()
        .with_id(FILE_ID)
        .with_producer(PRODUCER)
        .with_creation_date(CREATED.parse()?))
}

/// `Same`: the bytes match. `Created`: there was no reference, so one was written. `Changed`:
/// they differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Same,
    Created,
    Changed,
}

/// Compares `bytes` with the reference PDF at `reference`. A changed PDF is written to
/// `<name>.new.pdf` next to the reference, or replaces it when `update` is set.
pub(crate) fn compare_with_reference(bytes: &[u8], reference: &Path, update: bool) -> std::io::Result<Outcome> {
    let pending = pending_path(reference);
    if !reference.exists() {
        if let Some(folder) = reference.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(reference, bytes)?;
        return Ok(Outcome::Created);
    }
    if std::fs::read(reference)? == bytes {
        remove_if_present(&pending)?;
        return Ok(Outcome::Same);
    }
    if update {
        std::fs::write(reference, bytes)?;
        remove_if_present(&pending)?;
    } else {
        std::fs::write(&pending, bytes)?;
    }
    Ok(Outcome::Changed)
}

/// `invoice.pdf` → `invoice.new.pdf`, in the same folder.
fn pending_path(reference: &Path) -> PathBuf {
    let stem = reference.file_stem().unwrap_or_default().to_string_lossy();
    reference.with_file_name(format!("{stem}.new.pdf"))
}

fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}
