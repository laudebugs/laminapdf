//! `FileSource` and `MmapSource`: parity with an in-memory open, and a file truncated mid-read.
#![cfg(feature = "native")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests"
)]

mod common;

use std::fs::{File, OpenOptions as FileOptions};
use std::path::PathBuf;
use std::sync::Arc;

use common::{Once, facts, first_difference, in_memory_facts, inputs, multi_page, show};
use lamina::{ByteSource, Document, OpenOptions};
use lamina_cos::SourceError;
use lamina_source::FileSource;

/// A file in the temp folder, removed when dropped.
struct TempFile(PathBuf);

impl TempFile {
    fn with(name: &str, bytes: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!("lamina-source-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        Self(path)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn source_facts(source: Arc<dyn ByteSource>) -> Vec<String> {
    match Document::open_source(source, OpenOptions::default()) {
        Ok(mut doc) => facts(&mut doc, &mut Once),
        Err(e) => vec![show(&e)],
    }
}

#[test]
fn a_file_source_reads_every_input_like_an_in_memory_open() {
    let mut failures = Vec::new();
    for (i, (name, bytes)) in inputs().iter().enumerate() {
        let file = TempFile::with(&format!("file-{i}.pdf"), bytes);
        let ours = source_facts(Arc::new(FileSource::open(&file.0).unwrap()));
        failures.extend(first_difference(&ours, &in_memory_facts(bytes)).map(|d| format!("{name}: {d}")));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_file_truncated_after_open_answers_failed_not_a_crash() {
    let bytes = multi_page(false);
    let file = TempFile::with("truncated.pdf", bytes);
    let source = FileSource::open(&file.0).unwrap();
    let len = source.len();
    FileOptions::new()
        .write(true)
        .open(&file.0)
        .unwrap()
        .set_len(len / 2)
        .unwrap();
    let mut buf = vec![0; 1024];
    assert_eq!(source.read_at(0, &mut buf), Ok(1024), "bytes still there read");
    assert_eq!(
        source.read_at(len - 1024, &mut buf),
        Err(SourceError::Failed {
            offset: len - 1024,
            len: 1024
        })
    );
    // Through the reader: open needs the tail, which is gone.
    let err = Document::open_source(Arc::new(source), OpenOptions::default())
        .err()
        .unwrap();
    assert_eq!(err.code(), "source", "{err:?}");
}

#[test]
fn a_file_source_reads_the_end_short_and_past_it_empty() {
    let file = TempFile::with("short.bin", b"0123456789");
    let source = FileSource::from_file(File::open(&file.0).unwrap()).unwrap();
    let mut buf = [0; 8];
    assert_eq!(source.read_at(6, &mut buf), Ok(4));
    assert_eq!(&buf[..4], b"6789");
    assert_eq!(source.read_at(10, &mut buf), Ok(0));
}

#[cfg(feature = "mmap")]
mod mmap {
    use super::*;
    use lamina_source::MmapSource;

    #[test]
    fn a_mapped_file_reads_every_input_like_an_in_memory_open() {
        let mut failures = Vec::new();
        for (i, (name, bytes)) in inputs().iter().enumerate() {
            let file = TempFile::with(&format!("mmap-{i}.pdf"), bytes);
            let source = MmapSource::open(&file.0).unwrap();
            assert_eq!(
                source.as_slice(),
                Some(bytes.as_slice()),
                "{name}: the mapping is the file"
            );
            let ours = source_facts(Arc::new(source));
            failures.extend(first_difference(&ours, &in_memory_facts(bytes)).map(|d| format!("{name}: {d}")));
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn an_empty_file_maps_as_empty() {
        let file = TempFile::with("empty.bin", b"");
        let source = MmapSource::open(&file.0).unwrap();
        assert_eq!(source.len(), 0);
        assert_eq!(source.as_slice(), Some(&[][..]));
    }
}
