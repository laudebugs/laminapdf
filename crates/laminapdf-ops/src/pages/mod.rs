//! The page-structure ops (E2 plan §2.1): insert blank pages, delete, move, rotate and reverse,
//! over the model's page-tree editor (ISO 32000-2 §7.7.3). Each keeps the page labels right
//! (`PageLabels::remap`) and reports the form fields whose widgets it touched, for the save
//! planner's field-level checks (ADR 0007 §11).

mod cleanup;
mod delete;
mod insert;
mod order;
mod rotate;
mod structure;

use std::collections::BTreeSet;

use laminapdf_model::{PageId, ReferenceKind};

pub use cleanup::{ReferenceChange, ReferenceOutcome};
pub use delete::{DeletePages, ReferencePolicy};
pub use insert::{InsertBlankPages, PageSize};
pub use order::{MovePages, ReversePages};
pub use rotate::{RotateMode, RotatePages};

use crate::class::ModificationClass;
use crate::error::{Error, Result};
use crate::widgets::{TouchedWidget, WidgetChange};

/// The pages in order.
pub(crate) fn page_ids(doc: &laminapdf_model::Document) -> Result<Vec<PageId>> {
    Ok(doc.pages()?.iter().map(|p| p.id().clone()).collect())
}

/// The pages in order before a page op, after making the tree readable: a page dictionary
/// listed twice is a cycle to the page reader, so the editor first copies it to a new object
/// for its second listing (raising `invalid-entry`), as any page-tree edit would.
pub(crate) fn pages_before(doc: &mut laminapdf_model::Document) -> Result<Vec<PageId>> {
    let editor = doc.edit_page_tree()?;
    let ids = editor.page_ids();
    let mut seen = BTreeSet::new();
    let listed_twice = !ids.iter().all(|id| seen.insert(id));
    if listed_twice {
        editor.finish()?;
        page_ids(doc)
    } else {
        Ok(ids)
    }
}

/// Every page of `pages` in the document, once each, in the order given; an unknown page is
/// [`Error::UnknownPage`].
pub(crate) fn checked(all: &[PageId], pages: &[PageId]) -> Result<Vec<PageId>> {
    let known: BTreeSet<&PageId> = all.iter().collect();
    let mut seen: BTreeSet<&PageId> = BTreeSet::new();
    let mut out: Vec<PageId> = Vec::with_capacity(pages.len());
    for page in pages {
        if !known.contains(page) {
            return Err(Error::UnknownPage(page.clone()));
        }
        if seen.insert(page) {
            out.push(page.clone());
        }
    }
    Ok(out)
}

/// Writes the page labels for the page list `after`, from those that described `before`
/// (every surviving page keeps its label; new pages continue the range they land in). A
/// document without labels is left alone.
pub(crate) fn relabel(doc: &mut laminapdf_model::Document, before: &[PageId], after: &[PageId]) -> Result<()> {
    let labels = doc.page_labels()?;
    if labels.is_empty() {
        return Ok(());
    }
    let remapped = labels.remap(before, after);
    if remapped != labels {
        remapped.write(&mut doc.cos_mut().edit())?;
    }
    Ok(())
}

/// The form fields whose widgets are on `pages`, each once, as changed by `change`.
pub(crate) fn widgets_on(
    doc: &laminapdf_model::Document,
    pages: &[PageId],
    change: WidgetChange,
    class: ModificationClass,
) -> Result<Vec<TouchedWidget>> {
    let mut out: Vec<TouchedWidget> = Vec::new();
    for entry in doc.page_references(pages)?.entries() {
        if let ReferenceKind::Widget { field, .. } = &entry.kind {
            let touched = TouchedWidget::new(laminapdf_model::FieldId::object(*field), change, class);
            if !out.contains(&touched) {
                out.push(touched);
            }
        }
    }
    Ok(out)
}
