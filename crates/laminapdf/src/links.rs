//! A page's links (ISO 32000-2 §12.5.6.5) in page space (ADR 0027), each with what following
//! it does, so a viewer can draw them over the page and act on them without reading PDF
//! objects. Nothing here runs an action: a `Launch`, `JavaScript` or `SubmitForm` link is
//! reported by its type, for the viewer to say it won't run it.

use laminapdf_model::{AnnotId, Link, LinkAction, PageFrame, PointPt, RectPt};
use laminapdf_ops::{AddLink, OutlineTarget};

use crate::document::Document;
use crate::error::{Error, Result};
use crate::navigation::{PageTarget, target};

/// One link on a page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PageLink {
    /// Its ID, for the link edits; `None` for a link stored inside the page's list, which
    /// can't be edited.
    pub id: Option<AnnotId>,
    /// Its rectangle, in page space.
    pub rect: RectPt,
    /// The areas that follow it, four corners each in page space, when the file gives them
    /// (`/QuadPoints`); otherwise empty and the rectangle is the area.
    pub quads: Vec<[PointPt; 4]>,
    /// Its description (`/Contents`), document text.
    pub contents: Option<String>,
    /// What following it does.
    pub to: LinkTo,
}

/// What following a link does.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LinkTo {
    /// A place in this document: `target` when it names one of its pages, `name` for a named
    /// destination (§12.3.2.4).
    Destination {
        /// The named destination's name.
        name: Option<String>,
        /// Where it goes; `None` for a destination that names no page here.
        target: Option<PageTarget>,
    },
    /// A web address or other URI, as the file stores it (document text; not checked).
    Uri(String),
    /// A place in another file (`GoToR`), named by that file when it can be read.
    OtherFile(Option<String>),
    /// A named action (`NextPage`, `PrevPage`, `FirstPage`, `LastPage`, or another name).
    NamedAction(String),
    /// An action a reader never runs (`Launch`, `JavaScript`, `SubmitForm`…), by its type.
    Unsupported(String),
}

impl Document {
    /// The visible links on page `index`, in the page's annotation order. Hidden links, links
    /// without a rectangle and links with no action are left out.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the annotations, named destinations, the
    /// page tree).
    ///
    /// Added in 0.2.0.
    pub fn page_links(&self, index: u32) -> Result<Vec<PageLink>> {
        let pages = self.ops().pages()?;
        let page = usize::try_from(index)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or(Error::PageIndexOutOfRange {
                index,
                count: u32::try_from(pages.len()).unwrap_or(u32::MAX),
            })?;
        let frame = page.frame()?;
        let mut out = Vec::new();
        for link in pages.links(&page)? {
            let Link {
                id,
                rect,
                quads,
                contents,
                action,
                ..
            } = link;
            let to = match action {
                LinkAction::GoTo(dest) => {
                    let goes_to = match dest.page_index.and_then(|i| pages.get(i)) {
                        Some(to) => target(&dest, &to.frame()?),
                        None => None,
                    };
                    LinkTo::Destination {
                        name: dest.name.as_deref().map(|n| String::from_utf8_lossy(n).into_owned()),
                        target: goes_to,
                    }
                }
                LinkAction::Uri(uri) => LinkTo::Uri(String::from_utf8_lossy(&uri).into_owned()),
                LinkAction::OtherFile(file) => LinkTo::OtherFile(file),
                LinkAction::Named(name) => LinkTo::NamedAction(String::from_utf8_lossy(&name).into_owned()),
                LinkAction::Other(kind) => LinkTo::Unsupported(kind.as_name().into_owned()),
                _ => continue,
            };
            out.push(PageLink {
                id,
                rect: frame.rect_to_page(&rect),
                quads: quads.iter().map(|q| q.map(|p| frame.to_page(p))).collect(),
                contents,
                to,
            });
        }
        Ok(out)
    }
}

impl Document {
    /// The edit that adds a link going to `target` on `area`, a box the user drew in page space,
    /// of page `page` (0-based). Apply it with [`Document::apply`]; its description and
    /// `/NM` are the caller's to set.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree).
    ///
    /// Added in 0.3.0.
    pub fn link_on_area(&self, page: u32, area: RectPt, target: OutlineTarget) -> Result<AddLink> {
        let (id, frame) = self.page_frame(page)?;
        Ok(AddLink::new(id, user_rect(&frame, &[area]), target))
    }

    /// The edit that adds a link on text of page `page` (0-based): `lines` are the boxes of
    /// the text, one per line, in page space, as a selection gives them. Each line becomes one
    /// of the link's quads. In a tagged document, the MCIDs of the glyphs inside them place the
    /// link in the structure tree beside its text ([`AddLink::marked`]).
    ///
    /// # Errors
    /// `invalid-op` for no lines; `page-index-out-of-range`; the reading errors of
    /// [`Document::page_text`].
    ///
    /// Added in 0.3.0.
    #[cfg(feature = "extract")]
    pub fn link_on_text(&self, page: u32, lines: &[RectPt], target: OutlineTarget) -> Result<AddLink> {
        if lines.is_empty() {
            return Err(laminapdf_ops::Error::InvalidOp("a link on text needs one line or more").into());
        }
        let (id, frame) = self.page_frame(page)?;
        let marked = self.text_under(page, lines)?.marked;
        let mut op = AddLink::new(id, user_rect(&frame, lines), target);
        op.quads = lines
            .iter()
            .map(|l| {
                [
                    PointPt::new(l.x0, l.y0),
                    PointPt::new(l.x1, l.y0),
                    PointPt::new(l.x0, l.y1),
                    PointPt::new(l.x1, l.y1),
                ]
                .map(|p| frame.to_user_space(p))
            })
            .collect();
        op.marked = marked;
        Ok(op)
    }

    /// Page `page`'s ID and frame.
    pub(crate) fn page_frame(&self, page: u32) -> Result<(laminapdf_model::PageId, PageFrame)> {
        let pages = self.ops().pages()?;
        let p = usize::try_from(page)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or(Error::PageIndexOutOfRange {
                index: page,
                count: u32::try_from(pages.len()).unwrap_or(u32::MAX),
            })?;
        Ok((p.id().clone(), p.frame()?))
    }
}

/// The user-space rectangle that bounds `areas`, given in page space.
pub(crate) fn user_rect(frame: &PageFrame, areas: &[RectPt]) -> laminapdf_model::UserSpaceRect {
    let bounds = areas
        .iter()
        .skip(1)
        .fold(areas.first().copied().unwrap_or_default(), |r, a| r.union(a));
    let a = frame.to_user_space(PointPt::new(bounds.x0, bounds.y0));
    let b = frame.to_user_space(PointPt::new(bounds.x1, bounds.y1));
    laminapdf_model::UserSpaceRect::from_corners(a.x, a.y, b.x, b.y)
}
