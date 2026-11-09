//! A page's links (ISO 32000-2 §12.5.6.5) in page space (ADR 0027), each with what following
//! it does, so a viewer can draw them over the page and act on them without reading PDF
//! objects. Nothing here runs an action: a `Launch`, `JavaScript` or `SubmitForm` link is
//! reported by its type, for the viewer to say it won't run it.

use laminapdf_model::{Link, LinkAction, PointPt, RectPt};

use crate::document::Document;
use crate::error::{Error, Result};
use crate::navigation::{PageTarget, target};

/// One link on a page.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PageLink {
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
                rect: frame.rect_to_page(&rect),
                quads: quads.iter().map(|q| q.map(|p| frame.to_page(p))).collect(),
                contents,
                to,
            });
        }
        Ok(out)
    }
}
