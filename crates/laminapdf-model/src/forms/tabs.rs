//! The order Tab visits a page's widgets in: the page's `/Tabs` (ISO 32000-2 §12.5.1, Table
//! 31): row, column, structure, annotations-array or widget order.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use laminapdf_cos::Result;

use crate::document::Document;
use crate::geometry::RectPt;
use crate::id::PageId;
use crate::page::read_rect;
use crate::structure::{StructElem, StructKid};
use crate::values::read_name;
use crate::widgets::PageWidget;

/// A page's `/Tabs`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Tabs {
    /// `/R`: row by row from the top, each row left to right (on the page as shown).
    Row,
    /// `/C`: column by column from the left, each column top to bottom.
    Column,
    /// `/S`: the structure tree's order (§14.7), which PDF/UA asks for.
    Structure,
    /// `/A` (PDF 2.0): the order of the page's `/Annots`.
    Annotations,
    /// `/W` (PDF 2.0): widget order. Widgets come in `/Annots` order; it differs from
    /// [`Tabs::Annotations`] only in where other annotations go, and the tab order lists
    /// widgets alone.
    Widgets,
}

impl Tabs {
    /// The value `/Tabs` names, when this reader knows it.
    fn from_name(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"R" => Self::Row,
            b"C" => Self::Column,
            b"S" => Self::Structure,
            b"A" => Self::Annotations,
            b"W" => Self::Widgets,
            _ => return None,
        })
    }
}

/// The widgets of a page in the order Tab visits them.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TabOrder {
    /// The page's `/Tabs`; `None` when it has none, or one this reader doesn't know, and the
    /// widgets come in `/Annots` order, as viewers show them.
    pub tabs: Option<Tabs>,
    /// The page's widgets, each with its field, in tab order.
    pub widgets: Vec<PageWidget>,
}

impl Document {
    /// The order Tab visits `page`'s widgets in ([`TabOrder`]). In row and column order, a
    /// widget joins a row (or column) when its top (or left) edge is within the first widget's
    /// upper (or left) half; widgets without a usable `/Rect` come last. In structure order, the
    /// widgets the structure tree doesn't reach follow the others, in `/Annots` order.
    ///
    /// # Errors
    /// Unavailable input, hard limits and strict-mode refusals reading the page, its widgets or
    /// the structure tree.
    ///
    /// Added in 0.6.0.
    pub fn tab_order(&self, page: &PageId) -> Result<TabOrder> {
        let widgets = self.page_widgets(page)?;
        let pages = self.pages()?;
        let Some(p) = pages.by_id(page) else {
            return Ok(TabOrder { tabs: None, widgets });
        };
        let tabs = read_name(self, p.dict().get("Tabs"))?.and_then(|n| Tabs::from_name(&n));
        let widgets = match tabs {
            Some(Tabs::Row | Tabs::Column) => {
                let frame = p.frame()?;
                let mut placed = Vec::with_capacity(widgets.len());
                for w in widgets {
                    let rect = match self.read(&laminapdf_cos::Object::Ref(w.widget))?.as_deref() {
                        Some(laminapdf_cos::Object::Dict(d)) => match d.get("Rect") {
                            Some(r) => read_rect(self, r)?.map(|r| frame.rect_to_page(&r)),
                            None => None,
                        },
                        _ => None,
                    };
                    placed.push((w, rect.filter(RectPt::is_finite)));
                }
                by_position(placed, tabs == Some(Tabs::Column))
            }
            Some(Tabs::Structure) => match self.structure()? {
                Some(tree) => by_structure(widgets, &tree.roots),
                None => widgets,
            },
            Some(Tabs::Annotations | Tabs::Widgets) | None => widgets,
        };
        Ok(TabOrder { tabs, widgets })
    }
}

/// A widget with its sort key: (along the line, across the lines, half its size across).
type Keyed = (PageWidget, (f64, f64, f64));

/// Widgets in row order (`columns` false) or column order, those without a rectangle last.
fn by_position(placed: Vec<(PageWidget, Option<RectPt>)>, columns: bool) -> Vec<PageWidget> {
    // (along, across, half): rows run along x and stack down y; columns the other way.
    let key = |r: &RectPt| {
        if columns {
            (r.y0.get(), r.x0.get(), (r.x1.get() - r.x0.get()) / 2.0)
        } else {
            (r.x0.get(), r.y0.get(), (r.y1.get() - r.y0.get()) / 2.0)
        }
    };
    let (mut boxed, unplaced): (Vec<_>, Vec<_>) = placed.into_iter().partition(|(_, r)| r.is_some());
    let mut keyed: Vec<Keyed> = boxed.drain(..).filter_map(|(w, r)| r.map(|r| (w, key(&r)))).collect();
    keyed.sort_by(|a, b| a.1.1.partial_cmp(&b.1.1).unwrap_or(Ordering::Equal));
    let mut lines: Vec<Vec<Keyed>> = Vec::new();
    for item in keyed {
        match lines.last_mut() {
            Some(line) if line.first().is_some_and(|(_, first)| item.1.1 < first.1 + first.2) => line.push(item),
            _ => lines.push(vec![item]),
        }
    }
    let mut out = Vec::new();
    for mut line in lines {
        line.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap_or(Ordering::Equal));
        out.extend(line.into_iter().map(|(w, _)| w));
    }
    out.extend(unplaced.into_iter().map(|(w, _)| w));
    out
}

/// Widgets in the order the structure tree's object references (`OBJR`) reach them, depth
/// first; the rest after, in `/Annots` order.
fn by_structure(widgets: Vec<PageWidget>, roots: &[StructElem]) -> Vec<PageWidget> {
    let mut order: BTreeMap<u32, usize> = BTreeMap::new();
    let mut stack: Vec<&StructKid> = roots.iter().rev().flat_map(|e| e.kids.iter().rev()).collect();
    while let Some(kid) = stack.pop() {
        match kid {
            StructKid::Elem(e) => stack.extend(e.kids.iter().rev()),
            StructKid::Object { object, .. } => {
                let next = order.len();
                order.entry(object.num).or_insert(next);
            }
            StructKid::Mcid { .. } => {}
        }
    }
    let (mut tagged, rest): (Vec<_>, Vec<_>) = widgets.into_iter().partition(|w| order.contains_key(&w.widget.num));
    tagged.sort_by_key(|w| order.get(&w.widget.num).copied());
    tagged.extend(rest);
    tagged
}
