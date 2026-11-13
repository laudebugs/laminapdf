//! `MovePages` and `ReversePages`: the page order (ISO 32000-2 §7.7.3.2), every page keeping
//! its object and [`PageId`] (ADR 0005 §5).

use std::collections::BTreeSet;

use laminapdf_model::{PageId, PagePosition};

use super::{checked, page_ids, pages_before, relabel, widgets_on};
use crate::class::ModificationClass;
use crate::error::Result;
use crate::op::Effect;
use crate::widgets::WidgetChange;
use crate::wire;

/// Moves `pages` to `to`. They keep their order among themselves (the order given, each page
/// once) and their IDs; an anchor that is itself moved is replaced by the nearest page that
/// stays.
///
/// On the wire: `{ "op": "movePages", "args": { "pages": ["p7.0", "p8.0"], "to": "start" } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct MovePages {
    /// The pages, in the order they take at `to`.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// Where they go.
    #[serde(with = "wire::position")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "\"start\" | \"end\" | { before: string } | { after: string }")
    )]
    pub to: PagePosition,
}

impl MovePages {
    /// The op that moves `pages` to `to`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(pages: Vec<PageId>, to: PagePosition) -> Self {
        Self { pages, to }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let before = pages_before(doc)?;
        let pages = checked(&before, &self.pages)?;
        if pages.is_empty() {
            return Ok(Effect::default());
        }
        {
            let mut editor = doc.edit_page_tree()?;
            editor.move_pages(&pages, &self.to)?;
            editor.finish()?;
        }
        finish_reorder(doc, &before, &pages)
    }
}

/// Reverses the order of `pages` within the places they hold: the first of them (in page
/// order) takes the last one's place, and so on; the other pages don't move.
///
/// On the wire: `{ "op": "reversePages", "args": { "pages": ["p3.0", "p4.0", "p5.0"] } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct ReversePages {
    /// The pages; their order here doesn't matter.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
}

impl ReversePages {
    /// The op that reverses `pages` within their places.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(pages: Vec<PageId>) -> Self {
        Self { pages }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let before = pages_before(doc)?;
        let pages = checked(&before, &self.pages)?;
        let desired = reversed(&before, &pages);
        let reversing: BTreeSet<&PageId> = pages.iter().collect();
        if desired == before {
            return Ok(Effect::default());
        }
        {
            let mut editor = doc.edit_page_tree()?;
            // Each reversed page goes right after the page it follows in the new order. Pages
            // that don't move keep their order, so this reaches the new order after one pass,
            // moving only the reversed pages.
            for (i, page) in desired.iter().enumerate() {
                if !reversing.contains(page) {
                    continue;
                }
                let to = match i.checked_sub(1).and_then(|p| desired.get(p)) {
                    Some(previous) => PagePosition::After(previous.clone()),
                    None => PagePosition::Start,
                };
                editor.move_pages(std::slice::from_ref(page), &to)?;
            }
            editor.finish()?;
        }
        finish_reorder(doc, &before, &pages)
    }
}

/// `order` with the places of `pages` filled by those pages in reverse page order.
fn reversed(order: &[PageId], pages: &[PageId]) -> Vec<PageId> {
    let pages: BTreeSet<&PageId> = pages.iter().collect();
    let mut flipped = order.iter().filter(|p| pages.contains(p)).rev();
    order
        .iter()
        .map(|p| {
            if pages.contains(p) {
                flipped.next().cloned().unwrap_or_else(|| p.clone())
            } else {
                p.clone()
            }
        })
        .collect()
}

/// After a reorder: the labels for the new order, and the effect, with the widgets on the
/// moved pages (their pages moved, so they are shown as they were).
fn finish_reorder(doc: &mut laminapdf_model::Document, before: &[PageId], moved: &[PageId]) -> Result<Effect> {
    let after = page_ids(doc)?;
    if after == before {
        return Ok(Effect::default());
    }
    relabel(doc, before, &after)?;
    let widgets = widgets_on(doc, moved, WidgetChange::Moved, ModificationClass::PageStructure)?;
    // The moved pages are listed as touched: a viewer that redraws touched pages sees the
    // order change, and so does undo, which reports the same pages.
    Ok(Effect {
        pages: moved.to_vec(),
        page_order: Some(after),
        widgets,
        ..Effect::default()
    })
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::ObjRef;

    use super::*;

    fn ids(nums: &[u32]) -> Vec<PageId> {
        nums.iter().map(|&n| PageId::object(ObjRef::new(n, 0))).collect()
    }

    #[test]
    fn reversed_fills_the_places_of_the_pages_in_reverse() {
        assert_eq!(
            reversed(&ids(&[1, 2, 3, 4, 5]), &ids(&[4, 2, 5])),
            ids(&[1, 5, 3, 4, 2])
        );
    }
}
