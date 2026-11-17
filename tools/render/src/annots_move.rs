//! The corpus edit test's edit: every annotation of a file moved by [`MOVE`] through the façade,
//! as a user dragging it would, and saved incrementally; then Lamina PDF reopens the saved file
//! and must find each moved annotation, and its Popup, exactly [`MOVE`] further on in user space,
//! and every other annotation where it was. `cargo xtask annots-parity --corpus` then checks the
//! saved file with `qpdf --check` and has PDFium, pdf.js and hayro draw it beside the original
//! ([`crate::annots_compare::compare_saved`]).
//!
//! Only a file Lamina PDF may save incrementally as it is (`Free`: unsigned, unencrypted, not
//! repaired) is edited; any other gets the [`Outcome`] that says why. An annotation the edit
//! refuses because it is locked or not editable (a widget, a direct dictionary) is counted, not
//! moved. A Popup moves with the annotation it belongs to, so it is checked, never moved itself.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use laminapdf::{ConstraintKind, Document, Offset, Op, OpenOptions, SaveMode, SaveOptions, UpdateAnnot};
use serde::Serialize;

/// The move, in user space: one unit right and one up. `/Rect` is in user space, so a moved
/// annotation's rectangle changes by exactly this, whatever the page's turn or `/UserUnit`.
pub const MOVE: (f64, f64) = (1.0, 1.0);
/// How far a moved coordinate may be from the expected one: the writer rounds reals to five
/// decimals.
const TOLERANCE: f64 = 1e-4;

/// What happened to one file.
///
/// `annots-parity move` writes these into its `report.json`, which `cargo xtask annots-parity`
/// (another workspace, so it can't share the type) reads by these names and compares with the
/// outcomes pinned in `fixtures/render-golden/annots-parity-corpus.toml`: rename them in all
/// three places together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Every annotation it could move moved, and the save reopened as it should.
    Moved,
    /// Skipped: only a full rewrite may save it (a repaired or encrypted file).
    RequiresRewrite,
    /// Skipped: signed, so only an incremental save that keeps the signatures may save it.
    AppendOnly,
    /// Skipped: its signatures' restrictions already conflict.
    Conflict,
    /// Skipped: every annotation is a widget or a direct dictionary, which the edit refuses.
    WidgetsOnly,
    /// Skipped: every annotation is locked.
    AllLocked,
    /// Skipped: no annotation at all, or locked ones and widgets only.
    NothingMovable,
    /// Lamina PDF couldn't open, edit, save or reopen it as it should: a failure.
    Failed,
}

/// What happened to one file, with the counts behind it.
#[derive(Debug, Serialize)]
pub struct Report {
    /// The outcome.
    pub outcome: Outcome,
    /// Annotations moved.
    pub moved: usize,
    /// Popups that moved with them.
    pub popups: usize,
    /// Annotations the edit refused as `Locked`.
    pub locked: usize,
    /// Annotations the edit refused as not editable: widgets, direct dictionaries.
    pub not_editable: usize,
    /// What went wrong.
    pub problems: Vec<String>,
}

/// Every annotation's user-space rectangle by ID.
type Rects = BTreeMap<String, Option<[f64; 4]>>;

/// Moves every annotation of `bytes`: what happened, and the saved file when there is one.
pub fn move_all(bytes: &[u8]) -> (Report, Option<Vec<u8>>) {
    let mut report = Report {
        outcome: Outcome::Failed,
        moved: 0,
        popups: 0,
        locked: 0,
        not_editable: 0,
        problems: Vec::new(),
    };
    let edited = edit(bytes, &mut report);
    let saved = match edited {
        Ok(Some((saved, moved))) => {
            if let Err(e) = check(bytes, &saved, &moved) {
                report.problems.push(format!("{e:#}"));
            }
            Some(saved)
        }
        Ok(None) => None,
        Err(e) => {
            report.problems.push(format!("{e:#}"));
            None
        }
    };
    if !report.problems.is_empty() {
        report.outcome = Outcome::Failed;
    }
    (report, saved)
}

/// Opens, moves and saves: the saved bytes and the IDs of what moved (Popups included), or
/// `None` when the file is skipped. Sets `report.outcome`.
fn edit(bytes: &[u8], report: &mut Report) -> Result<Option<(Vec<u8>, Vec<String>)>> {
    let mut doc = Document::open_bytes(bytes.to_vec(), OpenOptions::new()).context("opening")?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    report.outcome = match doc.plan(&options)?.constraint {
        ConstraintKind::Free => Outcome::Moved,
        ConstraintKind::RequiresRewrite => Outcome::RequiresRewrite,
        ConstraintKind::AppendOnly => Outcome::AppendOnly,
        ConstraintKind::Conflict => Outcome::Conflict,
        other => bail!("a save constraint this test doesn't know: {other:?}"),
    };
    if report.outcome != Outcome::Moved {
        return Ok(None);
    }
    let offset = Offset::new(MOVE.0, MOVE.1).context("a finite move")?;
    let mut moved = Vec::new();
    for page in 0..doc.page_count()? {
        let annots = doc.page_annotations(page)?;
        for annot in &annots {
            if annot.parent.is_some() {
                continue;
            }
            let Ok(id) = annot.id.parse() else {
                report.not_editable += 1;
                continue;
            };
            let mut update = UpdateAnnot::new(id);
            update.translate = Some(offset);
            match doc.apply(vec![Op::from(update)]) {
                Ok(_) => {
                    report.moved += 1;
                    moved.push(annot.id.clone());
                    if let Some(popup) = annot.popup.as_ref().filter(|p| annots.iter().any(|a| a.id == **p)) {
                        report.popups += 1;
                        moved.push(popup.clone());
                    }
                }
                Err(e) if e.code() == "annotation-locked" => report.locked += 1,
                Err(e) if e.code() == "annotation-not-editable" => report.not_editable += 1,
                Err(e) => report.problems.push(format!("page {page} {}: {}", annot.id, e.code())),
            }
        }
    }
    if moved.is_empty() {
        report.outcome = match (report.locked, report.not_editable) {
            (0, n) if n > 0 => Outcome::WidgetsOnly,
            (n, 0) if n > 0 => Outcome::AllLocked,
            _ => Outcome::NothingMovable,
        };
        return Ok(None);
    }
    Ok(Some((doc.save(&options)?.bytes, moved)))
}

/// Lamina PDF reopens `saved` with the same annotations, each one in `moved` exactly [`MOVE`]
/// further on and every other where it was.
fn check(original: &[u8], saved: &[u8], moved: &[String]) -> Result<()> {
    let before = user_rects(original)?;
    let after = user_rects(saved).context("reopening the saved file")?;
    let mut problems = Vec::new();
    if before.keys().ne(after.keys()) {
        problems.push(format!(
            "the annotations changed: {:?} became {:?}",
            before.keys().collect::<Vec<_>>(),
            after.keys().collect::<Vec<_>>()
        ));
    }
    for (id, rect) in &before {
        let shift = if moved.contains(id) { MOVE } else { (0.0, 0.0) };
        let got = after.get(id).copied().flatten();
        if !shifted_by(*rect, got, shift) {
            problems.push(format!("{id}: {rect:?} became {got:?}, expected a move by {shift:?}"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        bail!("{}", problems.join("; "))
    }
}

/// Every annotation's `/Rect` in user space, read through Lamina PDF's model (the façade's
/// records are in page space, which a turn or `/UserUnit` changes).
fn user_rects(bytes: &[u8]) -> Result<Rects> {
    let doc = Document::open_bytes(bytes.to_vec(), OpenOptions::new())?.into_ops();
    let mut rects = Rects::new();
    for page in doc.pages()?.iter() {
        for annot in doc.annotations(&page)? {
            rects.insert(annot.id.to_string(), annot.rect.map(|r| [r.x0, r.y0, r.x1, r.y1]));
        }
    }
    Ok(rects)
}

/// Whether `after` is `before` moved by exactly `(dx, dy)`, sign included.
fn shifted_by(before: Option<[f64; 4]>, after: Option<[f64; 4]>, (dx, dy): (f64, f64)) -> bool {
    match (before, after) {
        (None, None) => true,
        (Some(b), Some(a)) => b
            .iter()
            .zip(a)
            .zip([dx, dy, dx, dy])
            .all(|((b, a), d)| (a - b - d).abs() <= TOLERANCE),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEFORE: Option<[f64; 4]> = Some([10.0, 20.0, 30.0, 40.0]);

    /// fixtures/pages/boxes.pdf: four pages turned 0°, 90°, 180° and 270°, one with `/UserUnit` 2.
    fn boxes_with_a_note_a_page() -> Vec<u8> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/pages/boxes.pdf");
        let mut doc = Document::open_bytes(std::fs::read(path).unwrap(), OpenOptions::new()).unwrap();
        for page in 0..doc.page_count().unwrap() {
            let note = doc
                .note_at(
                    page,
                    laminapdf::PointPt::new(laminapdf::Pt::new(40.0), laminapdf::Pt::new(40.0)),
                    "A note",
                    laminapdf::NoteIcon::Comment,
                )
                .unwrap();
            doc.apply(vec![Op::from(note)]).unwrap();
        }
        doc.save(&SaveOptions::new()).unwrap().bytes
    }

    #[test]
    fn a_page_with_a_user_unit_moves_by_exactly_the_move() {
        let (report, saved) = move_all(&boxes_with_a_note_a_page());
        assert_eq!(
            (report.outcome, report.problems.len(), saved.is_some()),
            (Outcome::Moved, 0, true)
        );
    }

    #[test]
    fn a_rectangle_moved_by_the_move_counts_as_moved() {
        assert!(shifted_by(BEFORE, Some([11.0, 21.0, 31.0, 41.0]), MOVE));
    }

    #[test]
    fn a_rectangle_moved_the_other_way_does_not() {
        assert!(!shifted_by(BEFORE, Some([9.0, 19.0, 29.0, 39.0]), MOVE));
    }

    #[test]
    fn a_rectangle_moved_on_one_axis_only_does_not() {
        assert!(!shifted_by(BEFORE, Some([11.0, 20.0, 31.0, 40.0]), MOVE));
    }

    #[test]
    fn a_rectangle_that_stayed_does_not() {
        assert!(!shifted_by(BEFORE, BEFORE, MOVE));
    }

    #[test]
    fn a_lost_rectangle_does_not() {
        assert!(!shifted_by(BEFORE, None, MOVE));
    }

    #[test]
    fn the_matrix_moves_and_reopens_without_a_problem() {
        let (report, _) = move_all(&crate::annots_matrix::generate().unwrap());
        assert_eq!(
            (report.outcome, report.problems),
            (Outcome::Moved, Vec::<String>::new())
        );
    }

    #[test]
    fn every_mark_of_the_matrix_moves() {
        let (report, _) = move_all(&crate::annots_matrix::generate().unwrap());
        assert_eq!(
            report.moved,
            crate::annots_matrix::PAGES * crate::annots_matrix::SUBTYPES.len()
        );
    }

    #[test]
    fn a_file_lamina_cant_open_fails() {
        let (report, saved) = move_all(b"not a PDF");
        assert_eq!((report.outcome, saved.is_none()), (Outcome::Failed, true));
    }
}
