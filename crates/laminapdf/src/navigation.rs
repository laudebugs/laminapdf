//! Moving through a document: its outline (bookmarks, ISO 32000-2 §12.3.3) and its named
//! destinations (§12.3.2.4), each resolved to a page index and a view in page space (ADR 0027:
//! points from the displayed page's top-left, y down), so a viewer goes there without reading
//! PDF user space. Page labels (§12.4.2) are on [`crate::PageInfo::label`].

use laminapdf_model::{ActionKind, Destination, OutlineItemId, PageFrame, PointPt, Pt, UserSpacePoint, View};
use laminapdf_ops::{DestinationView, ExplicitDestination, Rgb};

use crate::document::Document;
use crate::error::Result;

/// One outline item.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct OutlineNode {
    /// Its ID, which the outline ops ([`crate::UpdateOutlineItem`], …) take.
    ///
    /// Added in 0.2.0.
    pub id: OutlineItemId,
    /// Its title (document text).
    pub title: String,
    /// Whether it opens with its children shown (`/Count` positive).
    pub open: bool,
    /// Where it goes, when its destination names a page of this document.
    pub target: Option<PageTarget>,
    /// The named destination it goes to (`/Dest (name)` or a `GoTo` naming one), when it names
    /// one.
    ///
    /// Added in 0.2.0.
    pub destination_name: Option<String>,
    /// The web address it opens (a `URI` action), when it has one.
    ///
    /// Added in 0.2.0.
    pub uri: Option<String>,
    /// The type of its action when that is neither a `GoTo` nor a `URI` (`JavaScript`,
    /// `Launch`, …): an edit replaces it only when it sets a new target.
    ///
    /// Added in 0.2.0.
    pub other_action: Option<String>,
    /// Its colour (`/C`), when it isn't the default black.
    ///
    /// Added in 0.2.0.
    pub color: Option<Rgb>,
    /// Shown in bold (`/F` bit 2).
    ///
    /// Added in 0.2.0.
    pub bold: bool,
    /// Shown in italic (`/F` bit 1).
    ///
    /// Added in 0.2.0.
    pub italic: bool,
    /// Its children, in order.
    pub children: Vec<OutlineNode>,
}

/// One named destination: a name links can jump to, and where it goes.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NamedDestination {
    /// The name (names that aren't UTF-8 are left out of the list).
    pub name: String,
    /// Where it goes, when it names a page of this document.
    pub target: Option<PageTarget>,
}

/// A place in the document: a page, and how to show it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct PageTarget {
    /// The page's 0-based index.
    pub page: u32,
    /// How to show it (§12.3.2.2, Table 149), in page space.
    pub view: PageTargetView,
}

/// How a target shows its page, in page space (points from the displayed page's top-left, y
/// down). A coordinate the file leaves unset is `None`: the viewer keeps its own.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum PageTargetView {
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
    ///
    /// Added in 0.2.0.
    pub fn outline(&self) -> Result<Vec<OutlineNode>> {
        let outline = self.ops().outline()?;
        let frames = self.frames()?;
        outline
            .roots()
            .filter_map(|item| node(&item, &frames).transpose())
            .collect()
    }

    /// The named destination `name` (§12.3.2.4), from the `/Names /Dests` tree or the catalog's
    /// `/Dests`: `None` when the document has no such name, or its target names no page here.
    ///
    /// # Errors
    /// Reading errors (the names, the page tree).
    ///
    /// Added in 0.2.0.
    pub fn named_destination(&self, name: &str) -> Result<Option<PageTarget>> {
        let outline = self.ops().outline()?;
        let Some(dest) = outline.named_destination(name.as_bytes())? else {
            return Ok(None);
        };
        // One page's frame, not every page's.
        let Some(index) = dest.page_index else { return Ok(None) };
        let pages = self.ops().pages()?;
        let Some(page) = pages.get(index) else { return Ok(None) };
        Ok(target(&dest, &page.frame()?))
    }

    /// Every named destination, sorted by name: the `/Names /Dests` tree's and the catalog's
    /// `/Dests` dictionary's (a name in both is listed once, as the tree resolves it).
    ///
    /// # Errors
    /// Reading errors (the names, the page tree).
    ///
    /// Added in 0.2.0.
    pub fn named_destinations(&self) -> Result<Vec<NamedDestination>> {
        let outline = self.ops().outline()?;
        let frames = self.frames()?;
        let mut out = Vec::new();
        for name in outline.destination_names()? {
            let target = outline
                .named_destination(&name)?
                .and_then(|dest| target(&dest, frames.get(dest.page_index?)?));
            // A name that isn't UTF-8 can't be named in an edit: it is left out of the list.
            let Ok(name) = String::from_utf8(name) else { continue };
            out.push(NamedDestination { name, target });
        }
        Ok(out)
    }

    /// `view` on the page at `page` (0-based), from page space (ADR 0027) into the page's user
    /// space: the destination the outline and named-destination ops take. `PageTargetView::At`
    /// with `x` and `y` is how "the current view" becomes a destination.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree).
    ///
    /// Added in 0.2.0.
    pub fn destination_at(&self, page: u32, view: PageTargetView) -> Result<ExplicitDestination> {
        let pages = self.ops().pages()?;
        let index = usize::try_from(page).unwrap_or(usize::MAX);
        let Some(p) = pages.get(index) else {
            return Err(crate::Error::PageIndexOutOfRange {
                index: page,
                count: u32::try_from(pages.len()).unwrap_or(u32::MAX),
            });
        };
        let frame = p.frame()?;
        Ok(ExplicitDestination::new(p.id().clone(), user_view(view, &frame)))
    }

    /// Every page's frame, in order, for moving destinations into page space.
    fn frames(&self) -> Result<Vec<PageFrame>> {
        let pages = self.ops().pages()?;
        pages.iter().map(|page| Ok(page.frame()?)).collect()
    }
}

/// `/F`'s italic and bold bits (§12.3.3, Table 152).
const ITALIC: i64 = 1;
const BOLD: i64 = 2;

/// The item as an `OutlineNode`; `None`, with its children, for one without an ID (the walk gives
/// every item one).
fn node(item: &laminapdf_model::OutlineItem<'_, '_>, frames: &[PageFrame]) -> Result<Option<OutlineNode>> {
    let Some(id) = item.id().cloned() else { return Ok(None) };
    let dest = item.target()?;
    let target = dest
        .as_ref()
        .and_then(|dest| target(dest, frames.get(dest.page_index?)?));
    let name = dest.and_then(|dest| dest.name).and_then(|n| String::from_utf8(n).ok());
    let other_action = item
        .action()?
        .filter(|a| !matches!(a, ActionKind::GoTo | ActionKind::Uri))
        .map(|a| a.as_name().into_owned());
    let flags = item.flags()?;
    Ok(Some(OutlineNode {
        id,
        title: item.title()?,
        open: item.is_open()?,
        target,
        destination_name: name,
        uri: item.uri()?,
        other_action,
        color: item
            .color()?
            .and_then(|c| Rgb::from_pdf(&c.map(laminapdf_cos::Object::Real))),
        bold: flags & BOLD != 0,
        italic: flags & ITALIC != 0,
        children: item
            .children()
            .filter_map(|child| node(&child, frames).transpose())
            .collect::<Result<_>>()?,
    }))
}

/// `dest` in page space on its page, whose frame is `frame` (`None` without a page index, or past
/// `u32`).
pub(crate) fn target(dest: &Destination, frame: &PageFrame) -> Option<PageTarget> {
    let index = dest.page_index?;
    Some(PageTarget {
        page: u32::try_from(index).ok()?,
        view: page_view(&dest.view, frame),
    })
}

/// `view` in page space. On a page turned a quarter, user space's x runs down the page and its
/// y across it, so an `/XYZ` left or top sets the other page axis. `/FitH` and `/FitV` still fit
/// the displayed width and height there, as pdf.js does. Their operand names no displayed top or
/// left there, so it is dropped, as Lamina PDF's builder writes it (`null`); pdf.js instead maps
/// it as a point, which moves the other axis.
fn page_view(view: &View, frame: &PageFrame) -> PageTargetView {
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
            PageTargetView::At {
                x,
                y,
                zoom: zoom.filter(|z| *z > 0.0),
            }
        }
        View::Fit | View::FitB => PageTargetView::Fit,
        View::FitH { top } | View::FitBH { top } => {
            let (_, y) = point(None, top.filter(|_| !turned));
            PageTargetView::FitWidth { y }
        }
        View::FitV { left } | View::FitBV { left } => {
            let (x, _) = point(left.filter(|_| !turned), None);
            PageTargetView::FitHeight { x }
        }
        View::FitR(rect) => {
            let a = frame.to_page(UserSpacePoint::new(rect.x0, rect.y0));
            let b = frame.to_page(UserSpacePoint::new(rect.x1, rect.y1));
            PageTargetView::FitRect {
                x0: a.x.get().min(b.x.get()),
                y0: a.y.get().min(b.y.get()),
                x1: a.x.get().max(b.x.get()),
                y1: a.y.get().max(b.y.get()),
            }
        }
        _ => PageTargetView::At {
            x: None,
            y: None,
            zoom: None,
        },
    }
}

/// `view` (page space) in the user space of the page `frame` shows: the inverse of
/// [`page_view`]. On a page turned a quarter the displayed top or left names the other user-space
/// coordinate, which `/FitH` and `/FitV` can't hold, so it is dropped there, as [`page_view`]
/// drops it and the builder writes it.
fn user_view(view: PageTargetView, frame: &PageFrame) -> DestinationView {
    let upright = frame.rotate().degrees().is_multiple_of(180);
    let user = |x: Option<f64>, y: Option<f64>| frame.to_user_space_unset(x.map(Pt::new), y.map(Pt::new));
    match view {
        PageTargetView::At { x, y, zoom } => {
            let (left, top) = user(x, y);
            DestinationView::Xyz { left, top, zoom }
        }
        PageTargetView::FitWidth { y } => DestinationView::FitH {
            top: user(None, y).1.filter(|_| upright),
        },
        PageTargetView::FitHeight { x } => DestinationView::FitV {
            left: user(x, None).0.filter(|_| upright),
        },
        PageTargetView::FitRect { x0, y0, x1, y1 } => {
            let a = frame.to_user_space(PointPt::new(Pt::new(x0), Pt::new(y0)));
            let b = frame.to_user_space(PointPt::new(Pt::new(x1), Pt::new(y1)));
            DestinationView::FitR {
                left: a.x.min(b.x),
                bottom: a.y.min(b.y),
                right: a.x.max(b.x),
                top: a.y.max(b.y),
            }
        }
        _ => DestinationView::Fit,
    }
}
