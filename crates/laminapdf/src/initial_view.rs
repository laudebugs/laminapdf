//! How a document asks to be shown when it opens (ISO 32000-2 §12.2, §7.7.2) and its article
//! threads (§12.4.3), with pages as indices and places in page space (ADR 0027), so a viewer
//! applies them without reading PDF user space. [`SetInitialView`](crate::SetInitialView)
//! changes the first.

use laminapdf_model::{OpenAction, PageFrame, PageLayout, PageMode, WindowPreferences};

use crate::RectPt;
use crate::document::Document;
use crate::error::Result;
use crate::navigation::{PageTarget, target};

/// How a document asks to be shown when it opens: [`Document::initial_view`].
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct OpeningView {
    /// Where it opens.
    pub open: OpenTarget,
    /// The panel to show (`/PageMode`); `None` when the file leaves it to the reader.
    pub page_mode: Option<PageMode>,
    /// How to lay the pages out (`/PageLayout`); `None` when the file leaves it to the reader.
    pub page_layout: Option<PageLayout>,
    /// The window's viewer preferences.
    pub preferences: WindowPreferences,
}

/// Where a document opens.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OpenTarget {
    /// Its first page, at the reader's zoom: no open destination, or one that names no page
    /// of this document.
    ReaderDefault,
    /// A page and how to show it.
    Page(PageTarget),
    /// An action other than going to a page, by its `/S` type (`JavaScript`, `Named`, `URI`, …).
    /// Lamina PDF never runs it, and [`SetInitialView`](crate::SetInitialView) never replaces it.
    Action(String),
}

/// An article thread: the parts of one article spread over pages (ISO 32000-2 §12.4.3).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ArticleThread {
    /// Its title, from the thread's information dictionary (document text).
    pub title: Option<String>,
    /// Its parts in reading order.
    pub parts: Vec<ThreadPart>,
}

/// One part of an article thread: a rectangle on a page.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ThreadPart {
    /// The page's 0-based index; `None` when the part names a page the document lacks.
    pub page: Option<u32>,
    /// The part's rectangle in page space (points from the displayed page's top-left, y down).
    pub rect: Option<RectPt>,
}

impl Document {
    /// How the document asks to be shown when it opens: the page and zoom (`/OpenAction`), the
    /// panel (`/PageMode`), the page layout (`/PageLayout`) and the window preferences
    /// (`/ViewerPreferences`). A value of the wrong type or an unknown name counts as absent,
    /// with an `invalid-entry` warning for the former.
    ///
    /// # Errors
    /// Reading errors (the catalog, the page tree, named destinations).
    ///
    /// Added in 0.3.0.
    pub fn initial_view(&self) -> Result<OpeningView> {
        let view = self.ops().initial_view()?;
        let open = match view.open {
            OpenAction::Destination(dest) => match dest.page_index {
                Some(index) => {
                    let pages = self.ops().pages()?;
                    match pages.get(index) {
                        Some(page) => target(&dest, &page.frame()?).map_or(OpenTarget::ReaderDefault, OpenTarget::Page),
                        None => OpenTarget::ReaderDefault,
                    }
                }
                None => OpenTarget::ReaderDefault,
            },
            OpenAction::Action(kind) => OpenTarget::Action(kind.as_name().into_owned()),
            _ => OpenTarget::ReaderDefault,
        };
        Ok(OpeningView {
            open,
            page_mode: view.page_mode,
            page_layout: view.page_layout,
            preferences: view.preferences,
        })
    }

    /// The document's article threads, in the catalog's order, each with its parts in reading
    /// order; empty for a document without any. A part whose page isn't in the document has
    /// no page and no rectangle.
    ///
    /// # Errors
    /// Reading errors (the catalog, the page tree).
    ///
    /// Added in 0.3.0.
    pub fn threads(&self) -> Result<Vec<ArticleThread>> {
        let threads = self.ops().threads()?;
        if threads.is_empty() {
            return Ok(Vec::new());
        }
        let pages = self.ops().pages()?;
        let frames: Vec<PageFrame> = pages.iter().map(|p| p.frame()).collect::<laminapdf_cos::Result<_>>()?;
        Ok(threads
            .into_iter()
            .map(|thread| ArticleThread {
                title: thread.title,
                parts: thread
                    .beads
                    .iter()
                    .map(|bead| {
                        let frame = bead.page_index.and_then(|i| frames.get(i));
                        ThreadPart {
                            page: frame.and(bead.page_index).and_then(|i| u32::try_from(i).ok()),
                            rect: frame.zip(bead.rect).map(|(f, r)| f.rect_to_page(&r)),
                        }
                    })
                    .collect(),
            })
            .collect())
    }
}
