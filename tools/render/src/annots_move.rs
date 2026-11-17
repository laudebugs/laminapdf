//! The corpus edit test: every annotation of a file moved by [`MOVE`] through the façade, as a
//! user dragging it would, saved incrementally, then opened again by Lamina PDF, which must
//! find each moved annotation (and its Popup) 1 pt further on, and by hayro, which must draw
//! every page. `cargo xtask annots-parity --corpus` adds `qpdf --check` and PDFium.
//!
//! Only a file Lamina PDF may save incrementally as it is (`Free`: unsigned, unencrypted, not
//! repaired) is edited; any other is skipped with the reason. An annotation the edit refuses
//! because it is locked or not editable (a widget, a direct dictionary) is counted, not moved.
//! A Popup moves with the annotation it belongs to, so it is checked, never moved itself.

use std::panic::{AssertUnwindSafe, catch_unwind};

use anyhow::{Context, Result, bail};
use laminapdf::{ConstraintKind, Document, Offset, Op, OpenOptions, RectPt, SaveMode, SaveOptions, UpdateAnnot};
use serde::Serialize;

use crate::hayro_render;

/// The move, in user space: one point right and one up.
const MOVE: (f64, f64) = (1.0, 1.0);
/// How far a moved coordinate may be from the expected one: the writer rounds reals.
const TOLERANCE: f64 = 0.01;

/// What happened to one file.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// `moved`, `skipped` or `failed`.
    pub outcome: &'static str,
    /// Why it was skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
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

/// An annotation the edit moved (or a Popup that moves with one): its page, ID and rectangle
/// before the move.
struct Before {
    page: u32,
    id: String,
    rect: Option<RectPt>,
}

/// Moves every annotation of `bytes`: what happened, and the saved file when there is one.
pub fn move_all(bytes: Vec<u8>) -> (Report, Option<Vec<u8>>) {
    let mut report = Report::default();
    let (saved, before) = match edit(bytes, &mut report) {
        Ok(Some(edited)) => edited,
        Ok(None) => {
            report.outcome = "skipped";
            return (report, None);
        }
        Err(e) => {
            report.problems.push(format!("{e:#}"));
            report.outcome = "failed";
            return (report, None);
        }
    };
    if let Err(e) = check(&saved, &before) {
        report.problems.push(format!("{e:#}"));
    }
    report.outcome = if report.problems.is_empty() { "moved" } else { "failed" };
    (report, Some(saved))
}

/// Opens, moves and saves: the saved bytes and what moved, or `None` when the file is skipped
/// (`report.reason` says why).
fn edit(bytes: Vec<u8>, report: &mut Report) -> Result<Option<(Vec<u8>, Vec<Before>)>> {
    let mut doc = match Document::open_bytes(bytes, OpenOptions::new()) {
        Ok(doc) => doc,
        Err(e) => {
            report.reason = Some(format!("Lamina PDF can't open it ({})", e.code()));
            return Ok(None);
        }
    };
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    let plan = doc.plan(&options)?;
    if plan.constraint != ConstraintKind::Free {
        report.reason = Some(format!(
            "its save constraint is {:?} ({:?})",
            plan.constraint, plan.rewrite_reasons
        ));
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
                    let popup = annot.popup.as_ref().and_then(|p| annots.iter().find(|a| a.id == *p));
                    report.popups += usize::from(popup.is_some());
                    for a in std::iter::once(annot).chain(popup) {
                        moved.push(Before {
                            page,
                            id: a.id.clone(),
                            rect: a.rect,
                        });
                    }
                }
                Err(e) if e.code() == "annotation-locked" => report.locked += 1,
                Err(e) if e.code() == "annotation-not-editable" => report.not_editable += 1,
                Err(e) => report.problems.push(format!("page {page} {}: {}", annot.id, e.code())),
            }
        }
    }
    if moved.is_empty() {
        report.reason = Some(format!(
            "nothing to move ({} locked, {} not editable)",
            report.locked, report.not_editable
        ));
        return Ok(None);
    }
    Ok(Some((doc.save(&options)?.bytes, moved)))
}

/// Lamina PDF finds every moved annotation 1 pt further on; hayro opens the file and draws
/// every page.
fn check(saved: &[u8], moved: &[Before]) -> Result<()> {
    let doc = Document::open_bytes(saved.to_vec(), OpenOptions::new()).context("reopening")?;
    let mut problems = Vec::new();
    for Before { page, id, rect } in moved {
        let annots = doc.page_annotations(*page)?;
        match annots.iter().find(|a| a.id == *id) {
            None => problems.push(format!("page {page} {id}: gone after the save")),
            Some(after) if !moved_by_one(*rect, after.rect) => {
                problems.push(format!("page {page} {id}: {rect:?} became {:?}", after.rect));
            }
            Some(_) => {}
        }
    }
    let pages = catch_unwind(AssertUnwindSafe(|| -> Result<usize> {
        let hayro = hayro_render::Doc::load(saved.to_vec())?;
        for page in 0..hayro.page_count() {
            hayro.render(page, 72.0)?;
        }
        Ok(hayro.page_count())
    }));
    match pages {
        Ok(Ok(n)) if u32::try_from(n).ok() == Some(doc.page_count()?) => {}
        Ok(Ok(n)) => problems.push(format!("hayro counts {n} pages")),
        Ok(Err(e)) => problems.push(format!("hayro: {e:#}")),
        Err(_) => problems.push("hayro panicked".to_owned()),
    }
    if problems.is_empty() {
        Ok(())
    } else {
        bail!("{}", problems.join("; "))
    }
}

/// Whether `after` is `before` moved by one point on each axis, in whichever direction the
/// page's turn maps the user-space move to.
fn moved_by_one(before: Option<RectPt>, after: Option<RectPt>) -> bool {
    let (Some(b), Some(a)) = (before, after) else {
        return before.is_none() && after.is_none();
    };
    let near = |x: f64, y: f64| (x - y).abs() <= TOLERANCE;
    let (dx, dy) = (a.x0.get() - b.x0.get(), a.y0.get() - b.y0.get());
    near(dx.abs(), 1.0) && near(dy.abs(), 1.0) && near(a.x1.get() - b.x1.get(), dx) && near(a.y1.get() - b.y1.get(), dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use laminapdf::Pt;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
        RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
    }

    #[test]
    fn a_rectangle_moved_by_one_point_either_way_counts_as_moved() {
        let before = Some(rect(10.0, 20.0, 30.0, 40.0));
        assert!(moved_by_one(before, Some(rect(11.0, 19.0, 31.0, 39.0))));
        assert!(moved_by_one(before, Some(rect(9.0, 21.0, 29.0, 41.0))));
    }

    #[test]
    fn a_rectangle_moved_otherwise_does_not() {
        let before = Some(rect(10.0, 20.0, 30.0, 40.0));
        assert!(!moved_by_one(before, before));
        assert!(!moved_by_one(before, Some(rect(12.0, 21.0, 32.0, 41.0))));
        assert!(!moved_by_one(before, Some(rect(11.0, 21.0, 32.0, 41.0))));
        assert!(!moved_by_one(before, None));
    }

    #[test]
    fn the_matrix_moves_whole_and_reopens() {
        let (report, saved) = move_all(crate::annots_matrix::generate().unwrap());
        assert_eq!((report.outcome, report.problems.as_slice()), ("moved", &[][..]));
        assert_eq!(
            report.moved,
            crate::annots_matrix::PAGES * crate::annots_matrix::SUBTYPES.len()
        );
        assert!(saved.is_some());
    }
}
