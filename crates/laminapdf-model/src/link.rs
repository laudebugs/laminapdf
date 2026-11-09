//! Link annotations (ISO 32000-2 §12.5.6.5): where each link on a page sits and what it does,
//! read without running anything. A link goes to a destination (`/Dest`, or a `GoTo` action's
//! `/D`, §12.6.4.2), a URI (§12.6.4.8), another file (`GoToR`, §12.6.4.3) or a named action
//! (§12.6.4.11); any other action is reported by its type so a viewer can say it won't run it.
//!
//! Reading is tolerant (ADR 0003): a link without a usable `/Rect` is left out, a malformed
//! `/QuadPoints` falls back to the rectangle, and an unreadable action leaves the link with
//! [`LinkAction::None`].

use laminapdf_cos::{Dict, Object, Result};

use crate::document::Document;
use crate::geometry::{UserSpacePoint, UserSpaceRect};
use crate::outline::{ActionKind, Destination, Outline};
use crate::page::{HIDDEN_ANNOT_FLAGS, Page, number, read_rect};
use crate::values::{file_spec_name, read_name, text_of};

/// One link annotation on a page.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Link {
    /// `/Rect`, in user space.
    pub rect: UserSpaceRect,
    /// `/QuadPoints` (§12.5.6.5, Table 179), four corners each, in user space: the areas that
    /// follow the link when the file gives them; empty when absent or malformed.
    pub quads: Vec<[UserSpacePoint; 4]>,
    /// `/Contents`, decoded (§7.9.2.2): the link's alternate description.
    pub contents: Option<String>,
    /// What following the link does.
    pub action: LinkAction,
}

/// What following a link does.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LinkAction {
    /// A destination in this document (`/Dest` or `GoTo`), resolved as far as the file allows:
    /// [`Destination::page_index`] is `None` for a broken one.
    GoTo(Destination),
    /// `URI`: the address as stored (§12.6.4.8: 7-bit ASCII), not resolved against the
    /// catalog's `/URI /Base`.
    Uri(Vec<u8>),
    /// `GoToR`: a destination in another file, named by its file specification (`/UF`, else
    /// `/F`), when readable.
    OtherFile(Option<String>),
    /// `Named`: the action's name as stored (`NextPage`, `PrevPage`, `FirstPage`, `LastPage`
    /// are the standard ones, Table 207).
    Named(Vec<u8>),
    /// Any other action (`Launch`, `JavaScript`, `SubmitForm`…), which a reader never runs.
    Other(ActionKind),
    /// No action, or one that can't be read.
    None,
}

impl Outline<'_> {
    /// The visible link annotations of `page` ([`crate::Pages::links`]), destinations resolved
    /// through this outline's named destinations.
    pub(crate) fn links(&self, page: &Page<'_>) -> Result<Vec<Link>> {
        let doc = self.doc();
        let Some(annots) = page.dict().get("Annots") else {
            return Ok(Vec::new());
        };
        let Some(annots) = doc.read(annots)? else {
            return Ok(Vec::new());
        };
        let mut links = Vec::new();
        for entry in annots.as_array().unwrap_or_default() {
            let Some(annot) = doc.read(entry)? else { continue };
            let Some(annot) = annot.as_dict() else { continue };
            if let Some(link) = self.link(annot)? {
                links.push(link);
            }
        }
        Ok(links)
    }

    fn link(&self, annot: &Dict) -> Result<Option<Link>> {
        let doc = self.doc();
        if !matches!(read_name(doc, annot.get("Subtype"))?.as_deref(), Some(b"Link")) {
            return Ok(None);
        }
        let flags = match annot.get("F") {
            Some(f) => doc.read(f)?.and_then(|f| f.as_int()).unwrap_or(0),
            None => 0,
        };
        if flags & HIDDEN_ANNOT_FLAGS != 0 {
            return Ok(None);
        }
        let Some(rect) = (match annot.get("Rect") {
            Some(rect) => read_rect(doc, rect)?,
            None => None,
        }) else {
            return Ok(None);
        };
        let contents = match annot.get("Contents") {
            Some(c) => doc
                .read(c)?
                .as_deref()
                .and_then(text_of)
                .filter(|s| !s.trim().is_empty()),
            None => None,
        };
        Ok(Some(Link {
            rect,
            quads: quads(doc, annot.get("QuadPoints"))?,
            contents,
            action: self.link_action(annot)?,
        }))
    }

    /// `/Dest` first, as for outline items (§12.5.6.5: `/Dest` and `/A` are exclusive, and a
    /// `/Dest` that can't be read counts as absent), then `/A`.
    fn link_action(&self, annot: &Dict) -> Result<LinkAction> {
        let doc = self.doc();
        if let Some(dest) = annot.get("Dest")
            && let Some(dest) = self.destination(dest)?
        {
            return Ok(LinkAction::GoTo(dest));
        }
        let Some(action) = annot.get("A") else {
            return Ok(LinkAction::None);
        };
        let Some(action) = doc.read(action)? else {
            return Ok(LinkAction::None);
        };
        let Some(action) = action.as_dict() else {
            return Ok(LinkAction::None);
        };
        let Some(kind) = read_name(doc, action.get("S"))? else {
            return Ok(LinkAction::None);
        };
        Ok(match ActionKind::from_name(&kind) {
            ActionKind::GoTo => match action.get("D") {
                Some(d) => self.destination(d)?.map_or(LinkAction::None, LinkAction::GoTo),
                None => LinkAction::None,
            },
            ActionKind::Uri => match read_bytes(doc, action.get("URI"))? {
                Some(uri) => LinkAction::Uri(uri),
                None => LinkAction::None,
            },
            ActionKind::GoToR => LinkAction::OtherFile(file_name(doc, action.get("F"))?),
            ActionKind::Named => match read_name(doc, action.get("N"))? {
                Some(name) => LinkAction::Named(name),
                None => LinkAction::None,
            },
            other => LinkAction::Other(other),
        })
    }
}

/// A string's bytes (some writers store a URI as a name, which is read too).
fn read_bytes(doc: &Document, value: Option<&Object>) -> Result<Option<Vec<u8>>> {
    let Some(value) = value else { return Ok(None) };
    Ok(match doc.read(value)?.as_deref() {
        Some(Object::String(s)) => Some(s.bytes.clone()),
        Some(Object::Name(n)) => Some(n.as_bytes().to_vec()),
        _ => None,
    })
}

/// The file a file specification (§7.11) names: a string, or a dictionary's `/UF`, else `/F`.
fn file_name(doc: &Document, value: Option<&Object>) -> Result<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    let Some(spec) = doc.read(value)? else { return Ok(None) };
    let name = match &*spec {
        Object::Dict(d) => file_spec_name(doc, d)?,
        other => text_of(other),
    };
    Ok(name.filter(|n| !n.is_empty()))
}

/// `/QuadPoints`: groups of eight numbers, each the four corners of one area. Anything else
/// (a count not a multiple of eight, a value that isn't a number) gives none, so the link
/// falls back to its rectangle.
fn quads(doc: &Document, value: Option<&Object>) -> Result<Vec<[UserSpacePoint; 4]>> {
    let Some(value) = value else { return Ok(Vec::new()) };
    let Some(array) = doc.read(value)? else {
        return Ok(Vec::new());
    };
    let Some(items) = array.as_array() else {
        return Ok(Vec::new());
    };
    if items.is_empty() || items.len() % 8 != 0 {
        return Ok(Vec::new());
    }
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        match number(doc, item)? {
            Some(v) => values.push(v),
            None => return Ok(Vec::new()),
        }
    }
    let (groups, _) = values.as_chunks::<8>();
    Ok(groups
        .iter()
        .map(|&[x0, y0, x1, y1, x2, y2, x3, y3]| {
            [
                UserSpacePoint::new(x0, y0),
                UserSpacePoint::new(x1, y1),
                UserSpacePoint::new(x2, y2),
                UserSpacePoint::new(x3, y3),
            ]
        })
        .collect())
}
