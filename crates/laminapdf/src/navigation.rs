//! Moving through a document: its outline (bookmarks, ISO 32000-2 §12.3.3) and its named
//! destinations (§12.3.2.4), each resolved to a page index and a view in page space (ADR 0027:
//! points from the displayed page's top-left, y down), so a viewer goes there without reading
//! PDF user space. Page labels (§12.4.2) are on [`crate::PageInfo::label`].

use laminapdf_model::{PageFrame, UserSpacePoint, View};

use crate::document::Document;
use crate::error::Result;

/// One outline item.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct OutlineNode {
    /// Its title (document text).
    pub title: String,
    /// Whether it opens with its children shown (`/Count` positive).
    pub open: bool,
    /// Where it goes, when its destination names a page of this document.
    pub target: Option<Target>,
    /// Its children, in order.
    pub children: Vec<OutlineNode>,
}

/// A place in the document: a page, and how to show it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Target {
    /// The page's 0-based index.
    pub page: u32,
    /// How to show it (§12.3.2.2, Table 149), in page space.
    pub view: TargetView,
}

/// How a target shows its page, in page space (points from the displayed page's top-left, y
/// down). A coordinate the file leaves unset is `None`: the viewer keeps its own.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum TargetView {
    /// The point (`x`, `y`) at the viewport's top-left, at `zoom` (1 is 100 %) when given
    /// (`/XYZ`; a missing or unknown view is this with nothing set: the page's top).
    At {
        /// Left edge.
        x: Option<f64>,
        /// Top edge.
        y: Option<f64>,
        /// Zoom factor.
        zoom: Option<f64>,
    },
    /// The whole page in the viewport (`/Fit`, `/FitB`).
    Fit,
    /// The page's width fits the viewport, with `y` at its top (`/FitH`, `/FitBH`).
    FitWidth {
        /// Top edge.
        y: Option<f64>,
    },
    /// The page's height fits the viewport, with `x` at its left (`/FitV`, `/FitBV`).
    FitHeight {
        /// Left edge.
        x: Option<f64>,
    },
    /// The rectangle fits the viewport (`/FitR`).
    FitRect {
        /// Left.
        x0: f64,
        /// Top.
        y0: f64,
        /// Right.
        x1: f64,
        /// Bottom.
        y1: f64,
    },
}

impl Document {
    /// The outline's top-level items in order, each with its children; empty for a document
    /// without one. An item whose destination names no page of this document (another file,
    /// a broken reference) has no [`OutlineNode::target`]. The model bounds the walk (cycles,
    /// depth), so a broken outline yields what can be read.
    ///
    /// # Errors
    /// Reading errors (the outline, its destinations, the page tree).
    pub fn outline(&self) -> Result<Vec<OutlineNode>> {
        let outline = self.ops().outline()?;
        let frames = self.frames()?;
        outline.roots().map(|item| node(&item, &frames)).collect()
    }

    /// The named destination `name` (§12.3.2.4), from the `/Names /Dests` tree or the catalog's
    /// `/Dests`: `None` when the document has no such name, or its target names no page here.
    ///
    /// # Errors
    /// Reading errors (the names, the page tree).
    pub fn named_destination(&self, name: &str) -> Result<Option<Target>> {
        let outline = self.ops().outline()?;
        let Some(dest) = outline.named_destination(name.as_bytes())? else {
            return Ok(None);
        };
        Ok(target(dest.page_index, &dest.view, &self.frames()?))
    }

    /// Every page's frame, in order, for moving destinations into page space.
    fn frames(&self) -> Result<Vec<PageFrame>> {
        let pages = self.ops().pages()?;
        pages.iter().map(|page| Ok(page.frame()?)).collect()
    }
}

fn node(item: &laminapdf_model::OutlineItem<'_, '_>, frames: &[PageFrame]) -> Result<OutlineNode> {
    let target = match item.target()? {
        Some(dest) => target(dest.page_index, &dest.view, frames),
        None => None,
    };
    Ok(OutlineNode {
        title: item.title()?,
        open: item.is_open()?,
        target,
        children: item
            .children()
            .map(|child| node(&child, frames))
            .collect::<Result<_>>()?,
    })
}

/// A destination's page and view in page space, or `None` without a page of this document.
fn target(page_index: Option<usize>, view: &View, frames: &[PageFrame]) -> Option<Target> {
    let index = page_index?;
    let frame = frames.get(index)?;
    Some(Target {
        page: u32::try_from(index).ok()?,
        view: page_view(view, frame),
    })
}

/// `view` in page space. On a page turned a quarter, user space's x runs down the page and its
/// y across it, so an `/XYZ` left or top sets the other page axis. `/FitH` and `/FitV` still fit
/// the displayed width and height there, as pdf.js and Lamina PDF's builder read them, but their
/// operand names no displayed top or left, so it is dropped.
fn page_view(view: &View, frame: &PageFrame) -> TargetView {
    let turned = matches!(frame.rotate().degrees(), 90 | 270);
    // (x, y) in page space of a user-space point whose unset coordinates stay unset.
    let point = |left: Option<f64>, top: Option<f64>| {
        let p = frame.to_page(UserSpacePoint::new(left.unwrap_or(0.0), top.unwrap_or(0.0)));
        let (x, y) = (p.x.get(), p.y.get());
        if turned {
            (top.map(|_| x), left.map(|_| y))
        } else {
            (left.map(|_| x), top.map(|_| y))
        }
    };
    match *view {
        View::Xyz { left, top, zoom } => {
            let (x, y) = point(left, top);
            TargetView::At {
                x,
                y,
                zoom: zoom.filter(|z| *z > 0.0),
            }
        }
        View::Fit | View::FitB => TargetView::Fit,
        View::FitH { top } | View::FitBH { top } => {
            let (_, y) = point(None, top.filter(|_| !turned));
            TargetView::FitWidth { y }
        }
        View::FitV { left } | View::FitBV { left } => {
            let (x, _) = point(left.filter(|_| !turned), None);
            TargetView::FitHeight { x }
        }
        View::FitR(rect) => {
            let a = frame.to_page(UserSpacePoint::new(rect.x0, rect.y0));
            let b = frame.to_page(UserSpacePoint::new(rect.x1, rect.y1));
            TargetView::FitRect {
                x0: a.x.get().min(b.x.get()),
                y0: a.y.get().min(b.y.get()),
                x1: a.x.get().max(b.x.get()),
                y1: a.y.get().max(b.y.get()),
            }
        }
        _ => TargetView::At {
            x: None,
            y: None,
            zoom: None,
        },
    }
}
