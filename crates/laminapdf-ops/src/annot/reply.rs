//! Replies and review states (ISO 32000-2 §12.5.6.2, §12.5.6.4): [`AddReply`] answers a comment
//! and [`SetReviewState`] records a status on one. Both are Text annotations of their own, with
//! `/IRT` naming what they answer, so a thread keeps its whole history, as Acrobat's threads do:
//! changing a status adds a state rather than editing one.
//!
//! Both take their parent's `/Rect` and page, carry an appearance that draws nothing and leave
//! every hiding flag clear. A viewer that draws annotations then draws no second icon over the
//! comment, and a file that claims PDF/A keeps every annotation printable with an appearance
//! (ISO 19005-2 §6.3.2). Editing a reply's text and deleting a reply or a state are
//! [`UpdateAnnot`](super::UpdateAnnot) and [`DeleteAnnot`](super::DeleteAnnot).

use laminapdf_cos::{Dict, ObjRef, Object, PdfString, Stream};
use laminapdf_model::{AnnotFlags, AnnotId, PdfDate};
use serde::{Deserialize, Serialize};

use super::{Found, every_annotation, set_text, tags};
use crate::error::{Error, Result};
use crate::link::append_annot;
use crate::op::Effect;
use crate::wire;

/// Replies to a comment: a Text annotation with `/IRT` naming the parent and `/RT /R`.
///
/// The parent is any markup annotation, a reply included. The reply goes on the parent's page,
/// over its `/Rect`; its ID is in
/// [`Applied::created_annotations`](crate::Applied::created_annotations). A parent that isn't a
/// markup annotation (a link, a widget, a Popup) is `invalid-op`; a direct dictionary is
/// `annotation-not-editable`.
///
/// On the wire: `{ "op": "addReply", "args": { "parent": "a10.0", "contents": "Agreed", "author":
/// "Ben", "created": "D:20261101110000Z" } }`.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct AddReply {
    /// The annotation it answers.
    #[serde(with = "wire::annot_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub parent: AnnotId,
    /// `/Contents`: the reply.
    pub contents: String,
    /// `/T`: the author.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub author: Option<String>,
    /// `/CreationDate`, and `/M` with it.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_date")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub created: Option<PdfDate>,
    /// `/NM`, a name for matching it in exchanged comments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub nm: Option<String>,
}

impl AddReply {
    /// The op that answers `parent` with `contents`, with no author, date or `/NM`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn new(parent: AnnotId, contents: impl Into<String>) -> Self {
        Self {
            parent,
            contents: contents.into(),
            author: None,
            created: None,
            nm: None,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let entries = Entries {
            parent: &self.parent,
            contents: &self.contents,
            author: self.author.as_ref(),
            created: self.created.as_ref(),
            nm: self.nm.as_ref(),
            state: None,
        };
        entries.write(doc)
    }
}

/// A review status (Table 175), with the state model it belongs to.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum ReviewState {
    /// `Review` `None`: the status cleared.
    None,
    /// `Review` `Accepted`.
    Accepted,
    /// `Review` `Rejected`.
    Rejected,
    /// `Review` `Cancelled`.
    Cancelled,
    /// `Review` `Completed`.
    Completed,
    /// `Marked` `Marked`: a reviewer's check mark.
    Marked,
    /// `Marked` `Unmarked`: the check mark taken off.
    Unmarked,
}

impl ReviewState {
    /// `/StateModel`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn model(self) -> &'static str {
        match self {
            Self::None | Self::Accepted | Self::Rejected | Self::Cancelled | Self::Completed => "Review",
            Self::Marked | Self::Unmarked => "Marked",
        }
    }

    /// `/State`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Accepted => "Accepted",
            Self::Rejected => "Rejected",
            Self::Cancelled => "Cancelled",
            Self::Completed => "Completed",
            Self::Marked => "Marked",
            Self::Unmarked => "Unmarked",
        }
    }
}

/// Sets a comment's review status: a new Text annotation with `/IRT` naming the target and
/// `/StateModel` and `/State`. Earlier states stay, so the thread keeps its history; the latest
/// counts ([`CommentThread::latest_state`](laminapdf_model::CommentThread::latest_state)).
///
/// Its `/Contents` says what was set, as Acrobat writes it ("Accepted set by Ben"), so a screen
/// reader and a viewer without review support show it. The target is any markup annotation, as
/// for [`AddReply`], with the same errors.
///
/// On the wire: `{ "op": "setReviewState", "args": { "target": "a10.0", "state": "accepted",
/// "author": "Ben", "created": "D:20261102090000Z" } }`.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetReviewState {
    /// The annotation whose status it sets.
    #[serde(with = "wire::annot_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub target: AnnotId,
    /// The status.
    pub state: ReviewState,
    /// `/T`: who set it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub author: Option<String>,
    /// `/CreationDate`, and `/M` with it.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_date")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub created: Option<PdfDate>,
    /// `/NM`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub nm: Option<String>,
}

impl SetReviewState {
    /// The op that sets `state` on `target`, with no author, date or `/NM`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn new(target: AnnotId, state: ReviewState) -> Self {
        Self {
            target,
            state,
            author: None,
            created: None,
            nm: None,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let contents = match &self.author {
            Some(author) => format!("{} set by {author}", self.state.name()),
            None => self.state.name().to_owned(),
        };
        let entries = Entries {
            parent: &self.target,
            contents: &contents,
            author: self.author.as_ref(),
            created: self.created.as_ref(),
            nm: self.nm.as_ref(),
            state: Some(self.state),
        };
        entries.write(doc)
    }
}

/// What a reply or a state writes.
struct Entries<'a> {
    parent: &'a AnnotId,
    contents: &'a str,
    author: Option<&'a String>,
    created: Option<&'a PdfDate>,
    nm: Option<&'a String>,
    state: Option<ReviewState>,
}

impl Entries<'_> {
    fn write(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let all = every_annotation(doc)?;
        let found = Found::locate(&all, self.parent)?;
        if !found.annot.subtype.is_markup() {
            return Err(Error::InvalidOp(
                "a reply or a review state answers a markup annotation",
            ));
        }
        let parent = found.dict(doc)?;
        let rect = rect_array(doc, parent.get("Rect"))?;
        let page = found
            .page
            .as_lamina_id()
            .obj_ref()
            .ok_or_else(|| Error::DirectPage(found.page.clone()))?;
        let normal = blank_appearance(doc, &rect)?;
        let mut dict: Dict = [
            ("Type", Object::name("Annot")),
            ("Subtype", Object::name("Text")),
            ("Rect", Object::Array(rect.clone())),
            ("P", Object::Ref(page)),
            ("IRT", Object::Ref(found.object)),
            ("AP", Object::Dict([("N", Object::Ref(normal))].into_iter().collect())),
            (
                "F",
                Object::Integer(i64::from(
                    AnnotFlags::PRINT | AnnotFlags::NO_ZOOM | AnnotFlags::NO_ROTATE,
                )),
            ),
        ]
        .into_iter()
        .collect();
        match self.state {
            Some(state) => {
                dict.insert("StateModel", text(state.model()));
                dict.insert("State", text(state.name()));
            }
            None => {
                dict.insert("RT", Object::name("R"));
            }
        }
        set_text(&mut dict, "Contents", Some(&Some(self.contents.to_owned())));
        set_text(&mut dict, "T", Some(&self.author.cloned()));
        set_text(&mut dict, "NM", Some(&self.nm.cloned()));
        if let Some(date) = self.created {
            let date = PdfString::literal(date.to_pdf_string().into_bytes());
            dict.insert("CreationDate", Object::String(date.clone()));
            dict.insert("M", Object::String(date));
        }
        let annot = doc.cos_mut().edit().create(Object::Dict(dict))?;
        append_annot(doc, page, annot)?;
        tags::tag(doc, page, &[annot], &[])?;
        Ok(Effect {
            pages: vec![found.page],
            created_annotations: vec![AnnotId::object(annot)],
            ..Effect::default()
        })
    }
}

fn text(value: &str) -> Object {
    Object::String(PdfString::literal(value.as_bytes().to_vec()))
}

/// The parent's `/Rect` as four numbers, read through a reference; `[0 0 0 0]` when it has none
/// that reads.
fn rect_array(doc: &laminapdf_model::Document, rect: Option<&Object>) -> Result<Vec<Object>> {
    let items = match rect {
        Some(Object::Ref(r)) => laminapdf_cos::tolerant(doc.cos().resolve(*r))?
            .as_deref()
            .and_then(Object::as_array)
            .map(<[Object]>::to_vec),
        Some(Object::Array(items)) => Some(items.clone()),
        _ => None,
    };
    Ok(items
        .filter(|items| items.len() == 4 && items.iter().all(|i| i.as_f64().is_some()))
        .unwrap_or_else(|| vec![Object::Integer(0); 4]))
}

/// A Form XObject the size of `rect` that draws nothing.
fn blank_appearance(doc: &mut laminapdf_model::Document, rect: &[Object]) -> Result<ObjRef> {
    let side = |a: usize, b: usize| -> f64 {
        let at = |i: usize| rect.get(i).and_then(Object::as_f64).unwrap_or(0.0);
        (at(b) - at(a)).abs()
    };
    let dict: Dict = [
        ("Type", Object::name("XObject")),
        ("Subtype", Object::name("Form")),
        (
            "BBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::number(side(0, 2)),
                Object::number(side(1, 3)),
            ]),
        ),
        ("Resources", Object::Dict(Dict::new())),
    ]
    .into_iter()
    .collect();
    Ok(doc
        .cos_mut()
        .edit()
        .create(Object::Stream(Stream::new(dict, Vec::new())))?)
}
