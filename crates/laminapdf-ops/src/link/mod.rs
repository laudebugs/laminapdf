//! The link ops (ISO 32000-2 §12.5.6.5): [`AddLink`] makes a link on a drawn area or on text,
//! [`UpdateLink`] changes where it is, where it goes and its description, and [`DeleteLink`]
//! removes it. All three are class `Annotation`.
//!
//! A link goes where an [`OutlineTarget`] says, as a bookmark does: a page, a named
//! destination, or a web address. A web address must be an absolute `http`, `https` or `mailto`
//! one (`link-uri-refused`), so no op writes a link that runs a script. In a tagged document a
//! new link joins the structure tree as a `Link` element ([`tags`]).

mod tags;

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{AnnotFlags, AnnotId, AnnotSubtype, PageId, UserSpacePoint, UserSpaceRect};
use serde::{Deserialize, Serialize};

use crate::annot::{DeleteAnnot, Found, every_annotation, object_dict, set_text};
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::outline::OutlineTarget;
use crate::outline::target;
use crate::wire;

/// Adds a link to `page`: on the area `rect`, or on text, whose `quads` (one per line, as a
/// selection gives them) are the areas a click follows; `/Rect` then bounds every quad.
/// The link shows no border, prints with the page, and goes to `target`. `contents` is its
/// description, which screen readers read (`/Contents`), and `nm` its `/NM`, a name for
/// matching it in exchanged comments (never its identity, which is its [`AnnotId`]).
///
/// The new link's ID is in [`Applied::created_annotations`](crate::Applied::created_annotations).
///
/// In a tagged document the link becomes a `Link` element. `marked` lists the marked-content
/// IDs (MCIDs) of the page's content under the link: the element that owns the first of them
/// becomes the `Link` element's parent, and the content it owns among them moves into the
/// `Link` element. With none (a drawn area over a picture), the link joins the document
/// element. An untagged document that claims PDF/UA-1 can't hold a new link
/// (`link-not-tagged`). A rectangle or quad that isn't finite is `invalid-op`.
///
/// On the wire: `{ "op": "addLink", "args": { "page": "p3.0", "rect": { "x0": 72, "y0": 700,
/// "x1": 180, "y1": 714 }, "target": { "uri": "https://example.com/" } } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct AddLink {
    /// The page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The area, in user space.
    #[serde(with = "wire::user_space_rect")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "{ x0: number; y0: number; x1: number; y1: number }")
    )]
    pub rect: UserSpaceRect,
    /// For a link on text, the areas that follow it, four corners each in user space.
    #[serde(default, skip_serializing_if = "Vec::is_empty", with = "wire::quads")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "{ x: number; y: number }[][]"))]
    pub quads: Vec<[UserSpacePoint; 4]>,
    /// Where it goes.
    pub target: OutlineTarget,
    /// Its description (`/Contents`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub contents: Option<String>,
    /// Its `/NM`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub nm: Option<String>,
    /// The MCIDs of the page's content under the link, in reading order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub marked: Vec<u32>,
}

impl Eq for AddLink {}

impl AddLink {
    /// The op that adds a link on `rect` of `page` going to `target`, with no quads,
    /// description, `/NM` or marked content.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(page: PageId, rect: UserSpaceRect, target: OutlineTarget) -> Self {
        Self {
            page,
            rect,
            quads: Vec::new(),
            target,
            contents: None,
            nm: None,
            marked: Vec::new(),
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let rect = bounds(self.rect, &self.quads)?;
        let page = target::page_ref(doc, &self.page)?;
        let mut dict: Dict = [
            ("Type", Object::name("Annot")),
            ("Subtype", Object::name("Link")),
            ("Rect", numbers([rect.x0, rect.y0, rect.x1, rect.y1])),
            ("Border", numbers([0.0, 0.0, 0.0])),
            ("F", Object::Integer(i64::from(AnnotFlags::PRINT))),
            ("P", Object::Ref(page)),
        ]
        .into_iter()
        .collect();
        if !self.quads.is_empty() {
            dict.insert("QuadPoints", quad_points(&self.quads));
        }
        set_text(&mut dict, "Contents", Some(&self.contents));
        if let Some(nm) = &self.nm {
            set_text(&mut dict, "NM", Some(&Some(nm.clone())));
        }
        target::write(doc, &mut dict, &self.target)?;
        let annot = doc.cos_mut().edit().create(Object::Dict(dict))?;
        append_annot(doc, page, annot)?;
        tags::tag(doc, page, annot, &self.marked)?;
        Ok(Effect {
            pages: vec![self.page.clone()],
            created_annotations: vec![AnnotId::object(annot)],
            ..Effect::default()
        })
    }
}

/// Changes a link: its area, its quads (an empty list removes them, making it a link on its
/// area), where it goes, or its description (`null` removes it). An absent field is left
/// alone, and so is everything the op doesn't name, its place in the structure tree
/// included. `/Rect` always bounds the quads: new quads without a new area make it their bounds.
///
/// A `Locked` link refuses a new area, quads or target, and a `LockedContents` link refuses a new
/// description (`annotation-locked`); an annotation that isn't a link is `not-a-link`.
///
/// On the wire: `{ "op": "updateLink", "args": { "link": "a21.0", "target": { "named":
/// "chapter-2" } } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct UpdateLink {
    /// The link.
    #[serde(with = "wire::annot_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub link: AnnotId,
    /// A new area.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_user_space_rect")]
    #[cfg_attr(
        feature = "tsify",
        tsify(optional, type = "{ x0: number; y0: number; x1: number; y1: number }")
    )]
    pub rect: Option<UserSpaceRect>,
    /// New quads; empty removes them.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::quads::opt")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "{ x: number; y: number }[][]"))]
    pub quads: Option<Vec<[UserSpacePoint; 4]>>,
    /// A new target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub target: Option<OutlineTarget>,
    /// A new description, or `null` to remove it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub contents: Option<Option<String>>,
}

impl Eq for UpdateLink {}

impl UpdateLink {
    /// The op that changes nothing yet on `link`; set the fields to change.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(link: AnnotId) -> Self {
        Self {
            link,
            rect: None,
            quads: None,
            target: None,
            contents: None,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let found = locate_link(doc, &self.link)?;
        let flags = found.annot.flags;
        let moves = self.rect.is_some() || self.quads.is_some() || self.target.is_some();
        if (moves && flags.has(AnnotFlags::LOCKED))
            || (self.contents.is_some() && flags.has(AnnotFlags::LOCKED_CONTENTS))
        {
            return Err(Error::AnnotationLocked(self.link.clone()));
        }
        let mut dict = found.dict(doc)?;
        let before = dict.clone();
        if self.rect.is_some() || self.quads.is_some() {
            let quads = match &self.quads {
                Some(q) => q.clone(),
                None => read_quads(&before),
            };
            // A new rectangle, or new quads alone, set the area; the old one is kept only when
            // neither changes it, so a moved text link doesn't keep its old area.
            let base = match (self.rect, self.quads.as_deref()) {
                (Some(rect), _) => rect,
                (None, Some([[first, ..], ..])) => UserSpaceRect::from_corners(first.x, first.y, first.x, first.y),
                _ => read_rect(&before).unwrap_or(ZERO),
            };
            let rect = bounds(base, &quads)?;
            dict.insert("Rect", numbers([rect.x0, rect.y0, rect.x1, rect.y1]));
            if quads.is_empty() {
                dict.remove("QuadPoints");
            } else {
                dict.insert("QuadPoints", quad_points(&quads));
            }
        }
        if let Some(to) = &self.target {
            target::write(doc, &mut dict, to)?;
        }
        set_text(&mut dict, "Contents", self.contents.as_ref());
        if dict == before {
            return Ok(Effect::default());
        }
        doc.cos_mut().edit().set(found.object, Object::Dict(dict))?;
        Ok(Effect {
            pages: vec![found.page],
            ..Effect::default()
        })
    }
}

/// Deletes a link. In a tagged document its `Link` element goes too, and the text it held
/// goes back to the element it came from. A `Locked` link is refused (`annotation-locked`),
/// and an annotation that isn't a link is `not-a-link`.
///
/// On the wire: `{ "op": "deleteLink", "args": { "link": "a21.0" } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DeleteLink {
    /// The link.
    #[serde(with = "wire::annot_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub link: AnnotId,
}

impl DeleteLink {
    /// The op that deletes `link`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(link: AnnotId) -> Self {
        Self { link }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let found = locate_link(doc, &self.link)?;
        if found.annot.flags.has(AnnotFlags::LOCKED) {
            return Err(Error::AnnotationLocked(self.link.clone()));
        }
        tags::untag(doc, found.object)?;
        DeleteAnnot::new(self.link.clone()).apply_to(doc)
    }
}

/// The link `id`, which must be one.
fn locate_link(doc: &laminapdf_model::Document, id: &AnnotId) -> Result<Found> {
    let found = Found::locate(&every_annotation(doc)?, id)?;
    if found.annot.subtype == AnnotSubtype::Link {
        Ok(found)
    } else {
        Err(Error::NotALink(id.clone()))
    }
}

/// The empty rectangle at the origin, for a link whose `/Rect` can't be read.
const ZERO: UserSpaceRect = UserSpaceRect::from_corners(0.0, 0.0, 0.0, 0.0);

/// The link's `/Rect`, when it is four numbers.
fn read_rect(dict: &Dict) -> Option<UserSpaceRect> {
    let [a, b, c, d] = numbers_of(dict.get("Rect")?)?[..] else {
        return None;
    };
    Some(UserSpaceRect::from_corners(a, b, c, d))
}

/// The link's `/QuadPoints`, eight numbers per quad; none when it isn't that.
fn read_quads(dict: &Dict) -> Vec<[UserSpacePoint; 4]> {
    let Some(values) = dict.get("QuadPoints").and_then(numbers_of) else {
        return Vec::new();
    };
    let (quads, rest) = values.as_chunks::<8>();
    if !rest.is_empty() {
        return Vec::new();
    }
    quads
        .iter()
        .map(|[a, b, c, d, e, f, g, h]| {
            [
                UserSpacePoint::new(*a, *b),
                UserSpacePoint::new(*c, *d),
                UserSpacePoint::new(*e, *f),
                UserSpacePoint::new(*g, *h),
            ]
        })
        .collect()
}

fn numbers_of(value: &Object) -> Option<Vec<f64>> {
    value.as_array()?.iter().map(Object::as_f64).collect()
}

/// `rect` grown to hold every corner of `quads`; every number must be finite.
fn bounds(rect: UserSpaceRect, quads: &[[UserSpacePoint; 4]]) -> Result<UserSpaceRect> {
    let corners = quads.iter().flatten();
    let finite = [rect.x0, rect.y0, rect.x1, rect.y1].iter().all(|v| v.is_finite())
        && corners.clone().all(|p| p.x.is_finite() && p.y.is_finite());
    if !finite {
        return Err(Error::InvalidOp("a link's rectangle and quads must be finite numbers"));
    }
    Ok(corners.fold(rect, |r, p| {
        UserSpaceRect::from_corners(r.x0.min(p.x), r.y0.min(p.y), r.x1.max(p.x), r.y1.max(p.y))
    }))
}

fn numbers<const N: usize>(values: [f64; N]) -> Object {
    Object::Array(values.into_iter().map(Object::Real).collect())
}

/// `/QuadPoints`: eight numbers per quad, in the order given.
fn quad_points(quads: &[[UserSpacePoint; 4]]) -> Object {
    Object::Array(
        quads
            .iter()
            .flatten()
            .flat_map(|p| [Object::Real(p.x), Object::Real(p.y)])
            .collect(),
    )
}

/// Appends `annot` to the page's `/Annots`, in the array object when the page names one.
fn append_annot(doc: &mut laminapdf_model::Document, page: ObjRef, annot: ObjRef) -> Result<()> {
    let page_dict = object_dict(doc, page)?.unwrap_or_default();
    if let Some(Object::Ref(array)) = page_dict.get("Annots") {
        let mut cos = doc.cos_mut().edit();
        if let Some(current) = cos.get(*array)?
            && let Object::Array(items) = &*current
        {
            let mut items = items.clone();
            items.push(Object::Ref(annot));
            return Ok(cos.set(*array, Object::Array(items))?);
        }
    }
    let mut items = match page_dict.get("Annots") {
        Some(Object::Array(items)) => items.clone(),
        _ => Vec::new(),
    };
    items.push(Object::Ref(annot));
    let mut page_dict = page_dict;
    page_dict.insert("Annots", Object::Array(items));
    Ok(doc.cos_mut().edit().set(page, Object::Dict(page_dict))?)
}
