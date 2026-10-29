//! A sink that fails (ADR 0005 §3): the save returns the error and the document stays
//! `Dirty` with its overlay untouched, so the user can save elsewhere.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::save::{FailingSink, editable, facts, set_title, two_pages};
use lamina_ops::SaveState;
use lamina_save::{FullWriter, IoWrite, ReadableSink, Save, SaveError, SaveOptions, SinkError, WriteSink};

fn dirty() -> lamina_ops::Document {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "Pending");
    doc
}

fn overlay(doc: &lamina_ops::Document) -> Vec<(u32, lamina_cos::OverlayEntry)> {
    doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect()
}

#[test]
fn every_save_that_hits_a_full_sink_leaves_the_document_dirty() {
    for room in [0, 10, 500] {
        for which in 0..2 {
            let mut doc = dirty();
            let before = overlay(&doc);
            let mut sink = FailingSink::new(room);
            let result = match which {
                0 => doc.save_incremental(&mut sink, &SaveOptions::new()),
                _ => doc.save_full(&mut sink, &SaveOptions::new()),
            };
            assert_eq!(
                (
                    result.map_err(|e| e.code()).err(),
                    doc.save_state(),
                    overlay(&doc) == before,
                    doc.has_unsaved_changes(),
                    facts(&doc).title.as_deref(),
                ),
                (Some("sink"), SaveState::Dirty, true, true, Some("Pending")),
                "room {room}, writer {which}"
            );
        }
    }
}

/// A readable sink whose read-back fails.
struct Unreadable(Vec<u8>);

impl WriteSink for Unreadable {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError> {
        self.0.write_all(bytes)
    }
}

impl ReadableSink for Unreadable {
    fn into_source(self) -> Result<std::sync::Arc<dyn lamina_cos::ByteSource>, SinkError> {
        Err(SinkError::Failed("spool lost".into()))
    }
}

#[test]
fn a_sink_that_cannot_give_the_bytes_back_leaves_the_document_dirty() {
    let mut doc = dirty();
    let result = doc.save_full_and_rebase(Unreadable(Vec::new()), &SaveOptions::new());
    assert_eq!(
        (result.err(), doc.save_state(), doc.has_unsaved_changes()),
        (
            Some(SaveError::Sink(SinkError::Failed("spool lost".into()))),
            SaveState::Dirty,
            true
        )
    );
}

/// A readable sink that gives back something other than what was written.
struct Garbled(Vec<u8>);

impl WriteSink for Garbled {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError> {
        self.0.write_all(bytes)
    }
}

impl ReadableSink for Garbled {
    fn into_source(self) -> Result<std::sync::Arc<dyn lamina_cos::ByteSource>, SinkError> {
        Ok(std::sync::Arc::new(b"not a pdf".to_vec()))
    }
}

#[test]
fn a_failed_rebase_hands_the_output_back_for_a_retry() {
    let mut doc = dirty();
    let mut good = Vec::new();
    FullWriter::new(doc.cos(), SaveOptions::new()).write(&mut good).unwrap();
    let result = doc.save_full_and_rebase(Garbled(Vec::new()), &SaveOptions::new());
    let Err(SaveError::NotRebased { saved, .. }) = result else {
        panic!("{result:?}");
    };
    let stuck = doc.save_state();
    let wrong_again = doc.rebase(saved.source()).is_err();
    doc.rebase(good).unwrap();
    assert_eq!(
        (stuck, wrong_again, doc.save_state(), facts(&doc).title.as_deref()),
        (SaveState::SavedNotRebased, true, SaveState::Clean, Some("Pending"))
    );
}

/// A writer that refuses everything.
struct Broken;

impl std::io::Write for Broken {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(std::io::ErrorKind::StorageFull, "no space"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn io_write_reports_the_error_kind() {
    let mut doc = dirty();
    let result = doc.save_incremental(&mut IoWrite::new(Broken), &SaveOptions::new());
    assert!(matches!(
        result,
        Err(SaveError::Sink(SinkError::Io {
            kind: std::io::ErrorKind::StorageFull,
            ..
        }))
    ));
}
