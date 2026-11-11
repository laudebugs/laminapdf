//! Save a copy (`Save::save_copy`): the copy is what a save would write, the save-time stamp
//! included, and the document is left as it was: not rebased, its changes pending, so a later
//! plan still sees them (ADR 0007).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use common::save::{editable, set_title};
use laminapdf_model::PdfDate;
use laminapdf_ops::SaveState;
use laminapdf_save::{ConstraintKind, IncrementalOutput, Save, SaveError, SaveMode, SaveOptions, SavePlanner};

fn title(bytes: &[u8]) -> Option<String> {
    editable(bytes).info().unwrap().and_then(|i| i.title().unwrap())
}

fn mod_date(bytes: &[u8]) -> Option<String> {
    editable(bytes)
        .info()
        .unwrap()
        .and_then(|i| i.mod_date().unwrap())
        .map(|d| d.raw)
}

const AT: &str = "D:20261111120000Z";

#[test]
fn a_copy_holds_the_change_and_the_document_keeps_it_pending() {
    let base = fixture("corpus/hello.pdf");
    let mut doc = editable(&base);
    set_title(&mut doc, "Copied");
    let mut out = Vec::new();
    let report = doc.save_copy(&mut out, &SaveOptions::new()).unwrap();
    assert_eq!(
        (
            report.mode(),
            out.starts_with(&base),
            title(&out).as_deref(),
            doc.save_state(),
            doc.has_unsaved_changes(),
            doc.cos().source().len(),
        ),
        (
            SaveMode::Incremental,
            true,
            Some("Copied"),
            SaveState::Dirty,
            true,
            u64::try_from(base.len()).unwrap(),
        )
    );
}

#[test]
fn the_appended_revision_alone_completes_the_base() {
    let base = fixture("corpus/hello.pdf");
    let mut doc = editable(&base);
    set_title(&mut doc, "Tail");
    let mut tail = Vec::new();
    let options = SaveOptions::new().with_output(IncrementalOutput::AppendedOnly);
    doc.save_copy(&mut tail, &options).unwrap();
    let whole = [base.as_slice(), tail.as_slice()].concat();
    assert_eq!(title(&whole).as_deref(), Some("Tail"));
}

#[test]
fn a_second_copy_still_holds_every_change_since_the_last_save() {
    let mut doc = editable(&fixture("corpus/hello.pdf"));
    set_title(&mut doc, "One");
    doc.save_copy(&mut Vec::new(), &SaveOptions::new()).unwrap();
    set_title(&mut doc, "Two");
    let mut out = Vec::new();
    doc.save_copy(&mut out, &SaveOptions::new()).unwrap();
    // One revision over the base, with the newest title: not a revision per copy.
    assert_eq!((title(&out).as_deref(), editable(&out).revisions()), (Some("Two"), 2));
}

#[test]
fn the_stamp_goes_into_the_copy_only() {
    let mut doc = editable(&fixture("golden/metadata.pdf"));
    set_title(&mut doc, "Stamped");
    let date: PdfDate = AT.parse().unwrap();
    let mut out = Vec::new();
    let report = doc
        .save_copy(&mut out, &SaveOptions::new().with_metadata_date(date))
        .unwrap();
    let before = mod_date(&fixture("golden/metadata.pdf"));
    let mut now = Vec::new();
    // The document's own date is still the file's: a copy of it with no stamp shows it.
    doc.save_copy(&mut now, &SaveOptions::new()).unwrap();
    assert_eq!(
        (report.metadata_updated(), mod_date(&out).as_deref(), mod_date(&now)),
        (true, Some(AT), before)
    );
}

#[test]
fn a_full_rewrite_copy_leaves_the_document_saveable() {
    let mut doc = editable(&fixture("corpus/hello.pdf"));
    set_title(&mut doc, "Rewritten");
    let mut out = Vec::new();
    let options = SaveOptions::new().with_mode(SaveMode::FullRewrite);
    let report = doc.save_copy(&mut out, &options).unwrap();
    // Not SavedNotRebased: no rebase is pending, and a real save still works.
    assert_eq!(
        (report.mode(), title(&out).as_deref(), doc.save_state()),
        (SaveMode::FullRewrite, Some("Rewritten"), SaveState::Dirty)
    );
    doc.save(&mut Vec::new(), &SaveOptions::new()).unwrap();
}

#[test]
fn a_violation_stays_pending_after_a_copy_that_overrides_it() {
    // Certified P1 allows no change: the title edit is a conflict. A copy that overrides it
    // is written; the document still has the conflict, and a save without the override is
    // still refused.
    let mut doc = editable(&fixture("signed/certified-p1.pdf"));
    set_title(&mut doc, "Changed");
    let overridden = SaveOptions::new().with_invalidate_signatures(true);
    doc.save_copy(&mut Vec::new(), &overridden).unwrap();
    let kind = SavePlanner::new(&doc).unwrap().constraint().kind();
    let refused = doc.save_copy(&mut Vec::new(), &SaveOptions::new()).unwrap_err();
    assert_eq!(
        (kind, matches!(refused, SaveError::WouldInvalidateSignatures { .. })),
        (ConstraintKind::Conflict, true)
    );
}

#[test]
fn the_report_says_when_a_save_emptied_the_undo_history() {
    let save = |mode: SaveMode, copy: bool| {
        let mut doc = editable(&fixture("corpus/hello.pdf"));
        set_title(&mut doc, "History");
        let options = SaveOptions::new().with_mode(mode);
        let report = if copy {
            doc.save_copy(&mut Vec::new(), &options).unwrap()
        } else {
            doc.save_and_rebase(Vec::new(), &options).unwrap()
        };
        report.history_cleared()
    };
    assert_eq!(
        [
            save(SaveMode::Incremental, false),
            save(SaveMode::FullRewrite, false),
            save(SaveMode::FullRewrite, true),
        ],
        [false, true, false]
    );
}
