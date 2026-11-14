//! `SetPageBoxes` and `CropPages`: a page's boundaries (ISO 32000-2 §14.11.2). The media box is
//! the page's physical size; the crop box what a viewer shows; the bleed, trim and art boxes
//! where a print shop cuts and what the page means to show. Cropping only hides: content outside
//! the new crop box stays in the file and stays extractable (06 §10), so both ops report the
//! pages whose crop box now hides part of what it showed ([`Applied::cropped_content_remains`]).
//!
//! [`Applied::cropped_content_remains`]: crate::Applied::cropped_content_remains

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::Object;
use laminapdf_model::{PageId, PointPt, Pt, UserSpaceRect};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::cleanup::edit_dict;
use super::{checked, pages_before, widgets_on};
use crate::class::ModificationClass;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::widgets::WidgetChange;
use crate::wire;

/// The boxes [`SetPageBoxes`] writes: each box absent (left as it is), `null` (removed, so
/// the page shows the box it inherits or the default, §14.11.2; not for the media box, which
/// a page needs), or a rectangle in the page's raw user space, `[x0, y0, x1, y1]`.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct PageBoxes {
    /// `/MediaBox`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_rect")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "[number, number, number, number]"))]
    pub media: Option<UserSpaceRect>,
    /// `/CropBox`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "double_rect")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "[number, number, number, number] | null"))]
    pub crop: Option<Option<UserSpaceRect>>,
    /// `/BleedBox`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "double_rect")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "[number, number, number, number] | null"))]
    pub bleed: Option<Option<UserSpaceRect>>,
    /// `/TrimBox`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "double_rect")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "[number, number, number, number] | null"))]
    pub trim: Option<Option<UserSpaceRect>>,
    /// `/ArtBox`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "double_rect")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "[number, number, number, number] | null"))]
    pub art: Option<Option<UserSpaceRect>>,
}

impl PageBoxes {
    /// Sets the media box.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn with_media(mut self, rect: UserSpaceRect) -> Self {
        self.media = Some(rect);
        self
    }

    /// Sets the crop box, or removes it (`None`).
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn with_crop(mut self, rect: Option<UserSpaceRect>) -> Self {
        self.crop = Some(rect);
        self
    }

    /// Sets the bleed box, or removes it (`None`).
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn with_bleed(mut self, rect: Option<UserSpaceRect>) -> Self {
        self.bleed = Some(rect);
        self
    }

    /// Sets the trim box, or removes it (`None`).
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn with_trim(mut self, rect: Option<UserSpaceRect>) -> Self {
        self.trim = Some(rect);
        self
    }

    /// Sets the art box, or removes it (`None`).
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn with_art(mut self, rect: Option<UserSpaceRect>) -> Self {
        self.art = Some(rect);
        self
    }

    /// The boxes besides the media box, with their keys.
    #[expect(clippy::option_option, reason = "absent, null and a box are three cases")]
    fn others(&self) -> [(&'static str, Option<Option<UserSpaceRect>>); 4] {
        [
            ("CropBox", self.crop),
            ("BleedBox", self.bleed),
            ("TrimBox", self.trim),
            ("ArtBox", self.art),
        ]
    }
}

/// Sets the boxes of `pages` ([`PageBoxes`]). Each box must have an area and lie inside the
/// page's media box (the new one, when this op sets it); a viewer clips a box that doesn't to
/// the media box, so the op refuses it rather than write a box that means something else.
///
/// On the wire: `{ "op": "setPageBoxes", "args": { "pages": ["p3.0"], "boxes": { "crop": [36,
/// 36, 576, 756], "art": null } } }`.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetPageBoxes {
    /// The pages.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// The boxes to write.
    pub boxes: PageBoxes,
}

// Ops compare by value (`Op: Eq`); the one value that isn't equal to itself, NaN, never
// reaches an op from JSON, and is refused as `invalid-op`.
impl Eq for SetPageBoxes {}

impl SetPageBoxes {
    /// The op that writes `boxes` on `pages`.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn new(pages: Vec<PageId>, boxes: PageBoxes) -> Self {
        Self { pages, boxes }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let all = pages_before(doc)?;
        let pages = checked(&all, &self.pages)?;
        let before = shown_crops(doc, &pages)?;
        {
            let view = doc.pages()?;
            for id in &pages {
                let page = view.by_id(id).ok_or_else(|| Error::UnknownPage(id.clone()))?;
                // A page without a usable media box shows its crop box, or US Letter.
                let shown = page.frame()?.crop_box();
                let media = match self.boxes.media {
                    Some(media) => media,
                    None => page.media_box()?.filter(UserSpaceRect::has_area).unwrap_or(shown),
                };
                check_box(&media, None)?;
                for (_, rect) in self.boxes.others() {
                    if let Some(Some(rect)) = rect {
                        check_box(&rect, Some(&media))?;
                    }
                }
            }
        }
        for id in &pages {
            let r = obj_ref(id)?;
            let boxes = &self.boxes;
            edit_dict(doc, r, |d| {
                if let Some(media) = boxes.media {
                    d.insert("MediaBox", rect_object(&media));
                }
                for (key, rect) in boxes.others() {
                    match rect {
                        Some(Some(rect)) => d.insert(key, rect_object(&rect)),
                        Some(None) => {
                            d.remove(key);
                        }
                        None => {}
                    }
                }
            })?;
        }
        finish(doc, pages, &before)
    }
}

/// Margins to crop, in points of the page as displayed (ADR 0027 page space: rotation and
/// `/UserUnit` applied), so one set fits pages of mixed sizes and rotations. Each defaults to 0.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Margins {
    /// From the top edge as displayed.
    #[serde(default)]
    pub top: f64,
    /// From the right edge as displayed.
    #[serde(default)]
    pub right: f64,
    /// From the bottom edge as displayed.
    #[serde(default)]
    pub bottom: f64,
    /// From the left edge as displayed.
    #[serde(default)]
    pub left: f64,
}

impl Margins {
    /// The margins `top`, `right`, `bottom` and `left`, in points.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The same margin on every side.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn all(points: f64) -> Self {
        Self::new(points, points, points, points)
    }
}

/// Crops `pages` by `margins` taken from what each page shows now: each page's new crop box is
/// its displayed area less the margins, mapped back to its own user space, so a rotated page or
/// one with a `/UserUnit` loses the same visible amount as an upright one. Content outside the
/// new crop box stays in the file ([`Applied::cropped_content_remains`]).
///
/// On the wire: `{ "op": "cropPages", "args": { "pages": ["p3.0"], "margins": { "top": 36,
/// "left": 18 } } }`.
///
/// [`Applied::cropped_content_remains`]: crate::Applied::cropped_content_remains
///
/// Added in 0.4.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct CropPages {
    /// The pages.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// The margins to take off.
    pub margins: Margins,
}

// As for `SetPageBoxes`.
impl Eq for CropPages {}

impl CropPages {
    /// The op that crops `pages` by `margins`.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn new(pages: Vec<PageId>, margins: Margins) -> Self {
        Self { pages, margins }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let Margins {
            top,
            right,
            bottom,
            left,
        } = self.margins;
        if [top, right, bottom, left].iter().any(|m| !m.is_finite() || *m < 0.0) {
            return Err(Error::InvalidOp("margins must be finite and not negative"));
        }
        let all = pages_before(doc)?;
        let pages = checked(&all, &self.pages)?;
        let before = shown_crops(doc, &pages)?;
        let mut crops = Vec::with_capacity(pages.len());
        {
            let view = doc.pages()?;
            for id in &pages {
                let page = view.by_id(id).ok_or_else(|| Error::UnknownPage(id.clone()))?;
                let frame = page.frame()?;
                let (w, h) = (frame.width().get(), frame.height().get());
                if left + right >= w || top + bottom >= h {
                    return Err(Error::InvalidOp("the margins leave no page to show"));
                }
                let a = frame.to_user_space(PointPt::new(Pt::new(left), Pt::new(top)));
                let b = frame.to_user_space(PointPt::new(Pt::new(w - right), Pt::new(h - bottom)));
                crops.push((obj_ref(id)?, UserSpaceRect::from_corners(a.x, a.y, b.x, b.y)));
            }
        }
        for (r, crop) in crops {
            edit_dict(doc, r, |d| d.insert("CropBox", rect_object(&crop)))?;
        }
        finish(doc, pages, &before)
    }
}

/// The crop box each of `pages` shows (own or inherited, cut to the media box), read in one walk
/// of the page tree.
fn shown_crops(doc: &laminapdf_model::Document, pages: &[PageId]) -> Result<BTreeMap<PageId, UserSpaceRect>> {
    let wanted: BTreeSet<&PageId> = pages.iter().collect();
    let mut out = BTreeMap::new();
    for page in doc.pages()?.iter() {
        if wanted.contains(page.id()) {
            out.insert(page.id().clone(), page.frame()?.crop_box());
        }
    }
    Ok(out)
}

/// The effect of a box op on `pages`: the widgets it reshows, and the pages whose crop box, read
/// again after the write (so inherited boxes count), no longer shows all it showed `before`.
fn finish(
    doc: &laminapdf_model::Document,
    pages: Vec<PageId>,
    before: &BTreeMap<PageId, UserSpaceRect>,
) -> Result<Effect> {
    let after = shown_crops(doc, &pages)?;
    let cropped = pages
        .iter()
        .filter(|id| matches!((before.get(*id), after.get(*id)), (Some(b), Some(a)) if hides(b, a)))
        .cloned()
        .collect();
    let widgets = widgets_on(doc, &pages, WidgetChange::Reshown, ModificationClass::PageStructure)?;
    Ok(Effect {
        pages,
        widgets,
        cropped_content_remains: cropped,
        ..Effect::default()
    })
}

/// The page's object; a page dictionary held directly in its parent is refused.
fn obj_ref(id: &PageId) -> Result<laminapdf_cos::ObjRef> {
    id.as_lamina_id().obj_ref().ok_or_else(|| Error::DirectPage(id.clone()))
}

/// Refuses a box without an area, with a value that isn't finite, or outside `within`.
fn check_box(rect: &UserSpaceRect, within: Option<&UserSpaceRect>) -> Result<()> {
    let finite = rect.to_array().iter().all(|v| v.is_finite());
    if !finite || !rect.has_area() {
        return Err(Error::InvalidOp("a page box must be finite and have an area"));
    }
    if let Some(media) = within {
        let inside = rect.x0 >= media.x0 && rect.y0 >= media.y0 && rect.x1 <= media.x1 && rect.y1 <= media.y1;
        if !inside {
            return Err(Error::InvalidOp("a page box must lie inside the media box"));
        }
    }
    Ok(())
}

/// Whether `after` leaves out some of `before`.
fn hides(before: &UserSpaceRect, after: &UserSpaceRect) -> bool {
    const EPS: f64 = 1e-6;
    after.x0 > before.x0 + EPS || after.y0 > before.y0 + EPS || after.x1 < before.x1 - EPS || after.y1 < before.y1 - EPS
}

/// `[x0 y0 x1 y1]` as a PDF array.
fn rect_object(rect: &UserSpaceRect) -> Object {
    Object::Array(rect.to_array().iter().map(|&v| Object::number(v)).collect())
}

/// A rectangle on the wire: `[x0, y0, x1, y1]`, in any corner order.
fn rect_from(values: [f64; 4]) -> UserSpaceRect {
    UserSpaceRect::from_corners(values[0], values[1], values[2], values[3])
}

/// `Option<UserSpaceRect>` as `[x0, y0, x1, y1]`, absent when `None` (never `null`).
mod opt_rect {
    use super::{Deserialize, Deserializer, Serialize, Serializer, UserSpaceRect, rect_from};

    #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
    pub(super) fn serialize<S: Serializer>(rect: &Option<UserSpaceRect>, s: S) -> Result<S::Ok, S::Error> {
        rect.map(|r| r.to_array()).serialize(s)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<UserSpaceRect>, D::Error> {
        // `null` is refused: a page needs a media box.
        Ok(Some(rect_from(<[f64; 4]>::deserialize(d)?)))
    }
}

/// `Option<Option<UserSpaceRect>>`: absent (left alone), `null` (removed) or `[x0, y0, x1, y1]`.
mod double_rect {
    use super::{Deserialize, Deserializer, Serialize, Serializer, UserSpaceRect, rect_from};

    #[expect(
        clippy::ref_option,
        clippy::option_option,
        reason = "serde's `with` passes the field by reference; absent, null and a box are three cases"
    )]
    pub(super) fn serialize<S: Serializer>(rect: &Option<Option<UserSpaceRect>>, s: S) -> Result<S::Ok, S::Error> {
        rect.flatten().map(|r| r.to_array()).serialize(s)
    }

    #[expect(clippy::option_option, reason = "absent, null and a box are three cases")]
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<UserSpaceRect>>, D::Error> {
        Ok(Some(Option::<[f64; 4]>::deserialize(d)?.map(rect_from)))
    }
}
