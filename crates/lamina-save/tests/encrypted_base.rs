//! Both writers refuse an encrypted base before writing anything (ADR 0007 §2): there is no
//! encryption on write yet (E11) and removing it is B3c's explicit option. With nothing to
//! write, an incremental save is a no-op and allowed.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::sync::{Arc, Mutex};

use common::fixture;
use common::save::{corpus_doc, set_title};
use lamina_ops::SaveState;
use lamina_save::{
    IncrementalOutput, ReadableSink, Renumber, Save, SaveError, SaveOptions, SinkError, WriteSink, XrefKind,
};

fn dirty_encrypted() -> lamina_ops::Document {
    let mut doc = corpus_doc("enc.pdf");
    assert!(doc.is_encrypted());
    set_title(&mut doc, "Changed");
    doc
}

/// A readable sink whose bytes the test can still see after the save consumed it.
#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);

impl WriteSink for Shared {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(())
    }
}

impl ReadableSink for Shared {
    fn into_source(self) -> Result<Arc<dyn lamina_cos::ByteSource>, SinkError> {
        Ok(Arc::new(self.0.lock().unwrap().clone()))
    }
}

#[test]
fn every_save_of_an_encrypted_base_fails_and_writes_nothing() {
    let options = [
        SaveOptions::new(),
        SaveOptions::new().with_xref(XrefKind::Classic),
        SaveOptions::new().with_renumber(Renumber::Compact),
    ];
    for (i, options) in options.iter().enumerate() {
        for which in 0..3 {
            let mut doc = dirty_encrypted();
            let before: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
            let sink = Shared::default();
            let result = match which {
                0 => doc.save_incremental(&mut sink.clone(), options),
                1 => doc.save_full(&mut sink.clone(), options),
                _ => doc.save_full_and_rebase(sink.clone(), options),
            };
            let after: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
            let written = sink.0.lock().unwrap().len();
            assert_eq!(
                (result.err(), written, doc.save_state(), after == before),
                (
                    Some(SaveError::Cos(lamina_cos::Error::EncryptedBase)),
                    0,
                    SaveState::Dirty,
                    true
                ),
                "options {i}, writer {which}"
            );
        }
    }
}

#[test]
fn an_incremental_save_of_a_clean_encrypted_document_writes_the_base_unchanged() {
    let mut doc = corpus_doc("enc.pdf");
    let mut out = Vec::new();
    let report = doc.save_incremental(&mut out, &SaveOptions::new()).unwrap();
    assert_eq!((out == fixture("corpus/enc.pdf"), report.append_offset()), (true, None));
}

#[test]
fn an_appended_only_save_of_a_clean_document_reports_no_revision() {
    let mut doc = corpus_doc("hello.pdf");
    let report = doc
        .save_incremental(
            &mut Vec::new(),
            &SaveOptions::new().with_output(IncrementalOutput::AppendedOnly),
        )
        .unwrap();
    assert_eq!((report.bytes_written(), report.append_offset()), (0, None));
}

#[test]
fn the_refusal_has_the_cos_code() {
    assert_eq!(
        SaveError::from(lamina_cos::Error::EncryptedBase).code(),
        "encrypted-base"
    );
}
