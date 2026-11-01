//! Pages (ISO 32000-2 §7.7.3): [`Pages`] and the [`Page`] view, with boundary boxes
//! (§14.11.2), rotation and the page's resources, contents and annotations.

use std::cell::OnceCell;
use std::collections::BTreeMap;

use laminapdf_cos::{Dict, DirectPath, ObjRef, Object, PathSegment, Result, Warning};

use crate::document::Document;
use crate::geometry::{LETTER, PageFrame, UserSpaceRect};
use crate::id::PageId;
use crate::labels::PageLabels;
use crate::outline::Outline;

/// A page's rotation (`/Rotate`, §7.7.3.3), clockwise when displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Rotation {
    /// No rotation.
    #[default]
    None,
    /// 90° clockwise.
    Clockwise90,
    /// 180°.
    Half,
    /// 270° clockwise (90° counter-clockwise).
    Clockwise270,
}

impl Rotation {
    /// The rotation in degrees: 0, 90, 180 or 270.
    #[must_use]
    pub const fn degrees(self) -> u16 {
        match self {
            Self::None => 0,
            Self::Clockwise90 => 90,
            Self::Half => 180,
            Self::Clockwise270 => 270,
        }
    }

    /// The rotation for `degrees` (any multiple of 90, negative included), else `None`.
    #[must_use]
    pub const fn from_degrees(degrees: i64) -> Option<Self> {
        match degrees.rem_euclid(360) {
            0 => Some(Self::None),
            90 => Some(Self::Clockwise90),
            180 => Some(Self::Half),
            270 => Some(Self::Clockwise270),
            _ => None,
        }
    }
}

/// The document's pages in page-tree order (the result of one walk; see
/// [`laminapdf_cos::Document::pages`]).
#[derive(Debug)]
pub struct Pages<'d> {
    doc: &'d Document,
    walked: Vec<laminapdf_cos::Page>,
    ids: Vec<PageId>,
    /// Page index by ID (IDs are unique: one object, or one path, per page).
    by_id: BTreeMap<PageId, usize>,
    /// What each page reads once: its media and crop boxes (every other box is clipped to
    /// them), rotation, user unit and frame (ADR 0027 §3), so their warnings are raised once.
    cache: Vec<PageCache>,
    labels: OnceCell<PageLabels>,
}

/// A page's boxes, rotation, user unit and frame, each computed at most once.
#[derive(Debug, Default)]
struct PageCache {
    boxes: OnceCell<Boxes>,
    rotate: OnceCell<Rotation>,
    user_unit: OnceCell<f64>,
    frame: OnceCell<PageFrame>,
}

/// `cell`'s value, computing it with `compute` the first time (a failure is not cached).
fn memo<T: Copy>(cell: Option<&OnceCell<T>>, compute: impl FnOnce() -> Result<T>) -> Result<T> {
    if let Some(value) = cell.and_then(OnceCell::get) {
        return Ok(*value);
    }
    let value = compute()?;
    Ok(cell.map_or(value, |c| *c.get_or_init(|| value)))
}

/// A page's effective media and crop boxes.
#[derive(Debug, Clone, Copy)]
struct Boxes {
    media: Option<UserSpaceRect>,
    crop: Option<UserSpaceRect>,
}

impl<'d> Pages<'d> {
    pub(crate) fn new(doc: &'d Document) -> Result<Self> {
        let walked = doc.cos().pages()?;
        let ids: Vec<PageId> = walked
            .iter()
            .map(|p| match (&p.obj_ref, &p.direct_path) {
                (Some(r), _) => PageId::object(*r),
                (None, Some(path)) => PageId::path(path),
                // Unreachable: the walk sets one of the two for every page. Should that
                // change, the page is still named by where the tree starts.
                (None, None) => PageId::path(&DirectPath::new(
                    None,
                    vec![PathSegment::Key("Root"), PathSegment::Key("Pages")],
                )),
            })
            .collect();
        let by_id = ids.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();
        let cache = walked.iter().map(|_| PageCache::default()).collect();
        Ok(Self {
            doc,
            walked,
            ids,
            by_id,
            cache,
            labels: OnceCell::new(),
        })
    }

    /// The number of pages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.walked.len()
    }

    /// True when the document has no pages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.walked.is_empty()
    }

    /// The page at `index` (0-based), if any.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<Page<'_>> {
        let cos = self.walked.get(index)?;
        let id = self.ids.get(index)?;
        Some(Page {
            pages: self,
            cos,
            id,
            index,
        })
    }

    /// The page with ID `id`, if it is in this document.
    #[must_use]
    pub fn by_id(&self, id: &PageId) -> Option<Page<'_>> {
        self.index_of(id).and_then(|i| self.get(i))
    }

    /// The index of the page with ID `id`.
    #[must_use]
    pub fn index_of(&self, id: &PageId) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    /// The index of the page whose object is `r`.
    pub(crate) fn index_of_object(&self, r: ObjRef) -> Option<usize> {
        self.index_of(&PageId::object(r))
    }

    /// The pages in order.
    pub fn iter(&self) -> impl Iterator<Item = Page<'_>> + '_ {
        (0..self.len()).filter_map(|i| self.get(i))
    }

    /// The document outline, resolving destinations against these pages instead of walking
    /// the page tree again.
    ///
    /// # Errors
    /// As [`Document::outline`].
    pub fn outline(&self) -> Result<Outline<'_>> {
        Outline::with_pages(self.doc, self)
    }

    /// The page labels, read once per [`Pages`].
    fn labels(&self) -> Result<&PageLabels> {
        if let Some(labels) = self.labels.get() {
            return Ok(labels);
        }
        let labels = PageLabels::new(self.doc)?;
        Ok(self.labels.get_or_init(|| labels))
    }
}

/// One page: a thin view over its dictionary and inherited attributes.
#[derive(Debug, Clone, Copy)]
pub struct Page<'p> {
    pages: &'p Pages<'p>,
    cos: &'p laminapdf_cos::Page,
    id: &'p PageId,
    index: usize,
}

impl<'p> Page<'p> {
    /// The page's stable ID.
    #[must_use]
    pub const fn id(&self) -> &'p PageId {
        self.id
    }

    /// The page's 0-based index in the document.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// The page object (`None` for a direct page dictionary).
    #[must_use]
    pub const fn obj_ref(&self) -> Option<ObjRef> {
        self.cos.obj_ref
    }

    /// The page dictionary.
    #[must_use]
    pub const fn dict(&self) -> &'p Dict {
        &self.cos.dict
    }

    /// The COS page (inherited attributes resolved).
    #[must_use]
    pub const fn cos(&self) -> &'p laminapdf_cos::Page {
        self.cos
    }

    fn doc(&self) -> &'p Document {
        self.pages.doc
    }

    fn object(&self) -> Option<u32> {
        self.cos.obj_ref.map(|r| r.num)
    }

    fn page_number(&self) -> u64 {
        u64::try_from(self.index).unwrap_or(u64::MAX)
    }

    /// The box `key` from its stored value: `None` when absent, `None` plus
    /// [`Warning::InvalidBox`] (naming the page and the box) when it is not four numbers.
    fn read_box(&self, key: &'static str, value: Option<&Object>) -> Result<Option<UserSpaceRect>> {
        let Some(value) = value else {
            return Ok(None);
        };
        let rect = read_rect(self.doc(), value)?;
        if rect.is_none() {
            self.doc().warn(Warning::InvalidBox {
                object: self.object(),
                page: self.page_number(),
                key,
            })?;
        }
        Ok(rect)
    }

    /// The media and crop boxes, read once per page.
    fn boxes(&self) -> Result<Boxes> {
        memo(self.cache().map(|c| &c.boxes), || {
            let media = self.read_box("MediaBox", self.cos.inherited.media_box.as_deref())?;
            let crop = self.read_box("CropBox", self.cos.inherited.crop_box.as_deref())?;
            let crop = match (crop, media) {
                (Some(crop), Some(media)) => Some(crop.intersect(&media).unwrap_or(media)),
                (crop, media) => crop.or(media),
            };
            Ok(Boxes { media, crop })
        })
    }

    fn cache(&self) -> Option<&'p PageCache> {
        self.pages.cache.get(self.index)
    }

    /// `/MediaBox`, own or inherited (§14.11.2); `None` when absent or malformed (the latter
    /// with [`Warning::InvalidBox`]).
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::Strict`] in strict mode for a malformed box;
    /// [`laminapdf_cos::Error::DataPending`] / [`laminapdf_cos::Error::Source`].
    pub fn media_box(&self) -> Result<Option<UserSpaceRect>> {
        Ok(self.boxes()?.media)
    }

    /// The crop box: `/CropBox` (own or inherited) intersected with the media box, or the
    /// media box when `/CropBox` is absent or malformed (§14.11.2).
    ///
    /// # Errors
    /// As [`Page::media_box`].
    pub fn crop_box(&self) -> Result<Option<UserSpaceRect>> {
        Ok(self.boxes()?.crop)
    }

    /// A box that defaults to the crop box (bleed, trim and art boxes are not inheritable).
    /// The box itself is read first, so in strict mode its own problem is the one reported.
    fn clipped_box(&self, key: &'static str) -> Result<Option<UserSpaceRect>> {
        let own = self.read_box(key, self.cos.dict.get(key))?;
        let crop = self.crop_box()?;
        Ok(match (own, crop) {
            (Some(own), Some(crop)) => Some(own.intersect(&crop).unwrap_or(crop)),
            (own, crop) => own.or(crop),
        })
    }

    /// `/BleedBox` intersected with the crop box; defaults to the crop box.
    ///
    /// # Errors
    /// As [`Page::media_box`].
    pub fn bleed_box(&self) -> Result<Option<UserSpaceRect>> {
        self.clipped_box("BleedBox")
    }

    /// `/TrimBox` intersected with the crop box; defaults to the crop box.
    ///
    /// # Errors
    /// As [`Page::media_box`].
    pub fn trim_box(&self) -> Result<Option<UserSpaceRect>> {
        self.clipped_box("TrimBox")
    }

    /// `/ArtBox` intersected with the crop box; defaults to the crop box.
    ///
    /// # Errors
    /// As [`Page::media_box`].
    pub fn art_box(&self) -> Result<Option<UserSpaceRect>> {
        self.clipped_box("ArtBox")
    }

    /// `/Rotate`, own or inherited, read once per page. A value that is not a multiple of 90
    /// (or not a number) reads as [`Rotation::None`] with [`Warning::InvalidRotate`].
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::Strict`] in strict mode for an invalid value;
    /// [`laminapdf_cos::Error::DataPending`] / [`laminapdf_cos::Error::Source`].
    pub fn rotate(&self) -> Result<Rotation> {
        memo(self.cache().map(|c| &c.rotate), || self.read_rotate())
    }

    fn read_rotate(&self) -> Result<Rotation> {
        let Some(value) = self.cos.inherited.rotate.as_deref() else {
            return Ok(Rotation::None);
        };
        let degrees = self.doc().read(value)?.and_then(|v| match *v {
            Object::Integer(i) => Some(i),
            #[expect(
                clippy::cast_possible_truncation,
                reason = "only integral values in i64 range get here"
            )]
            Object::Real(r) if r.fract() == 0.0 && r.abs() < 1e15 => Some(r as i64),
            _ => None,
        });
        if let Some(rotation) = degrees.and_then(Rotation::from_degrees) {
            Ok(rotation)
        } else {
            self.doc().warn(Warning::InvalidRotate {
                object: self.object(),
                page: self.page_number(),
            })?;
            Ok(Rotation::None)
        }
    }

    /// `/UserUnit` (§14.11.2.1): the size of a default user-space unit in 1/72 inch, read
    /// once per page; 1 when absent, and 1 with [`Warning::InvalidEntry`] when not a positive
    /// number.
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::Strict`] in strict mode for an invalid value;
    /// [`laminapdf_cos::Error::DataPending`] / [`laminapdf_cos::Error::Source`].
    pub fn user_unit(&self) -> Result<f64> {
        memo(self.cache().map(|c| &c.user_unit), || self.read_user_unit())
    }

    fn read_user_unit(&self) -> Result<f64> {
        let Some(value) = self.cos.dict.get("UserUnit") else {
            return Ok(1.0);
        };
        let unit = self
            .doc()
            .read(value)?
            .and_then(|v| v.as_f64())
            .filter(|u| u.is_finite() && *u > 0.0);
        if let Some(unit) = unit {
            Ok(unit)
        } else {
            self.doc().warn(Warning::InvalidEntry {
                object: self.object(),
                key: "UserUnit",
            })?;
            Ok(1.0)
        }
    }

    /// The page's frame ([ADR 0027] §3): the effective crop box, [`Page::rotate`] and
    /// [`Page::user_unit`], computed once per page. When neither the crop box nor the media
    /// box has an area, the frame is US Letter (`[0 0 612 792]`) and [`Warning::InvalidBox`]
    /// (key `MediaBox`) is raised, once.
    ///
    /// # Errors
    /// As [`Page::media_box`], [`Page::rotate`] and [`Page::user_unit`].
    ///
    /// [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
    pub fn frame(&self) -> Result<PageFrame> {
        memo(self.cache().map(|c| &c.frame), || self.read_frame())
    }

    fn read_frame(&self) -> Result<PageFrame> {
        let Boxes { media, crop } = self.boxes()?;
        let rotate = self.rotate()?;
        let unit = self.user_unit()?;
        let reference = crop
            .filter(UserSpaceRect::has_area)
            .or(media.filter(UserSpaceRect::has_area));
        let frame = if let Some(reference) = reference {
            PageFrame::new(reference, rotate, unit)
        } else {
            self.doc().warn(Warning::InvalidBox {
                object: self.object(),
                page: self.page_number(),
                key: "MediaBox",
            })?;
            PageFrame::new(LETTER, rotate, unit).with_letter_fallback()
        };
        Ok(frame)
    }

    /// `/Resources`, own or inherited, resolved.
    #[must_use]
    pub fn resources(&self) -> Option<&'p Dict> {
        self.cos.resources_dict()
    }

    /// The references in `/Contents` (one stream, or an array of streams, possibly itself
    /// indirect). Direct elements and anything unreadable are left out.
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::DataPending`] / [`laminapdf_cos::Error::Source`].
    pub fn contents(&self) -> Result<Vec<ObjRef>> {
        self.refs("Contents", true)
    }

    /// The references in `/Annots` (an array, possibly indirect). Direct annotation
    /// dictionaries are left out (annotation views arrive with the editor rounds).
    ///
    /// # Errors
    /// As [`Page::contents`].
    pub fn annots(&self) -> Result<Vec<ObjRef>> {
        self.refs("Annots", false)
    }

    fn refs(&self, key: &str, single_ok: bool) -> Result<Vec<ObjRef>> {
        let Some(value) = self.cos.dict.get(key) else {
            return Ok(Vec::new());
        };
        let Some(resolved) = self.doc().read(value)? else {
            return Ok(Vec::new());
        };
        Ok(match (&*resolved, value) {
            (Object::Array(items), _) => items.iter().filter_map(Object::as_ref).collect(),
            (Object::Null, _) => Vec::new(),
            (_, Object::Ref(r)) if single_ok => vec![*r],
            _ => Vec::new(),
        })
    }

    /// The page label (§12.4.2), when the document has `/PageLabels`.
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::Strict`] in strict mode for a malformed label entry; errors
    /// reading the catalog.
    pub fn label(&self) -> Result<Option<String>> {
        Ok(self.pages.labels()?.label(self.index))
    }
}

/// `obj` as a rectangle of four numbers, tolerantly; `None` when it is anything else.
pub(crate) fn read_rect(doc: &Document, obj: &Object) -> Result<Option<UserSpaceRect>> {
    let Some(arr) = doc.read(obj)? else {
        return Ok(None);
    };
    let Some(items @ [_, _, _, _]) = arr.as_array() else {
        return Ok(None);
    };
    let mut out = [0.0; 4];
    for (slot, item) in out.iter_mut().zip(items) {
        match number(doc, item)? {
            Some(value) => *slot = value,
            None => return Ok(None),
        }
    }
    let [x0, y0, x1, y1] = out;
    Ok(Some(UserSpaceRect::from_corners(x0, y0, x1, y1)))
}

/// A finite number, tolerantly.
pub(crate) fn number(doc: &Document, obj: &Object) -> Result<Option<f64>> {
    Ok(doc.read(obj)?.and_then(|n| n.as_f64()).filter(|n| n.is_finite()))
}
