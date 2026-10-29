//! Pages (ISO 32000-2 §7.7.3): [`Pages`] and the [`Page`] view, with boundary boxes
//! (§14.11.2), rotation and the page's resources, contents and annotations.

use std::cell::OnceCell;
use std::collections::BTreeMap;

use lamina_cos::{Dict, DirectPath, ObjRef, Object, PathSegment, Result, Warning};

use crate::document::Document;
use crate::id::PageId;
use crate::labels::PageLabels;

/// A rectangle with its corners normalised (§7.9.5): `x0 <= x1` and `y0 <= y1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Lower-left x.
    pub x0: f64,
    /// Lower-left y.
    pub y0: f64,
    /// Upper-right x.
    pub x1: f64,
    /// Upper-right y.
    pub y1: f64,
}

impl Rect {
    /// The rectangle with corners `(a, b)` and `(c, d)`, in any order.
    #[must_use]
    pub const fn from_corners(a: f64, b: f64, c: f64, d: f64) -> Self {
        Self {
            x0: a.min(c),
            y0: b.min(d),
            x1: a.max(c),
            y1: b.max(d),
        }
    }

    /// `x1 - x0`.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    /// `y1 - y0`.
    #[must_use]
    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    /// The overlap of two rectangles, `None` when they don't overlap.
    #[must_use]
    pub const fn intersect(&self, other: &Self) -> Option<Self> {
        let r = Self {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        };
        if r.x0 <= r.x1 && r.y0 <= r.y1 { Some(r) } else { None }
    }

    /// `[x0 y0 x1 y1]`.
    #[must_use]
    pub const fn to_array(&self) -> [f64; 4] {
        [self.x0, self.y0, self.x1, self.y1]
    }
}

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
/// [`lamina_cos::Document::pages`]).
#[derive(Debug)]
pub struct Pages<'d> {
    doc: &'d Document,
    walked: Vec<lamina_cos::Page>,
    ids: Vec<PageId>,
    /// Page index by ID (IDs are unique: one object, or one path, per page).
    by_id: BTreeMap<PageId, usize>,
    labels: OnceCell<PageLabels>,
}

impl<'d> Pages<'d> {
    pub(crate) fn new(doc: &'d Document) -> Result<Self> {
        let pages = doc.cos().pages()?;
        let ids = pages
            .iter()
            .map(|p| {
                let id = match (&p.obj_ref, &p.direct_path) {
                    (Some(r), _) => PageId::object(*r),
                    (None, Some(path)) => PageId::path(path),
                    // Unreachable: the walk sets one of the two for every page. Should that
                    // change, the page is still named by where the tree starts.
                    (None, None) => PageId::path(&DirectPath::new(
                        None,
                        vec![PathSegment::Key("Root"), PathSegment::Key("Pages")],
                    )),
                };
                doc.ids().see(id.as_lamina_id().clone());
                id
            })
            .collect();
        let ids: Vec<PageId> = ids;
        let by_id = ids.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();
        Ok(Self {
            doc,
            walked: pages,
            ids,
            by_id,
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
    cos: &'p lamina_cos::Page,
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
    pub const fn cos(&self) -> &'p lamina_cos::Page {
        self.cos
    }

    fn doc(&self) -> &'p Document {
        self.pages.doc
    }

    fn object(&self) -> Option<u32> {
        self.cos.obj_ref.map(|r| r.num)
    }

    /// A box from its stored value: `None` when absent, `None` plus [`Warning::InvalidBox`]
    /// when it is not four numbers.
    fn read_box(&self, value: Option<&Object>) -> Result<Option<Rect>> {
        let Some(value) = value else {
            return Ok(None);
        };
        let rect = read_rect(self.doc(), value)?;
        if rect.is_none() {
            self.doc().warn(Warning::InvalidBox { object: self.object() })?;
        }
        Ok(rect)
    }

    /// `/MediaBox`, own or inherited (§14.11.2); `None` when absent or malformed (the latter
    /// with [`Warning::InvalidBox`]).
    ///
    /// # Errors
    /// [`lamina_cos::Error::Strict`] in strict mode for a malformed box;
    /// [`lamina_cos::Error::DataPending`] / [`lamina_cos::Error::Source`].
    pub fn media_box(&self) -> Result<Option<Rect>> {
        self.read_box(self.cos.inherited.media_box.as_deref())
    }

    /// The crop box: `/CropBox` (own or inherited) intersected with the media box, or the
    /// media box when `/CropBox` is absent or malformed (§14.11.2).
    ///
    /// # Errors
    /// As [`Page::media_box`].
    pub fn crop_box(&self) -> Result<Option<Rect>> {
        let media = self.media_box()?;
        let crop = self.read_box(self.cos.inherited.crop_box.as_deref())?;
        Ok(match (crop, media) {
            (Some(crop), Some(media)) => Some(crop.intersect(&media).unwrap_or(media)),
            (crop, media) => crop.or(media),
        })
    }

    /// A box that defaults to the crop box (bleed, trim and art boxes are not inheritable).
    fn clipped_box(&self, key: &str) -> Result<Option<Rect>> {
        let crop = self.crop_box()?;
        let own = self.read_box(self.cos.dict.get(key))?;
        Ok(match (own, crop) {
            (Some(own), Some(crop)) => Some(own.intersect(&crop).unwrap_or(crop)),
            (own, crop) => own.or(crop),
        })
    }

    /// `/BleedBox` intersected with the crop box; defaults to the crop box.
    ///
    /// # Errors
    /// As [`Page::media_box`].
    pub fn bleed_box(&self) -> Result<Option<Rect>> {
        self.clipped_box("BleedBox")
    }

    /// `/TrimBox` intersected with the crop box; defaults to the crop box.
    ///
    /// # Errors
    /// As [`Page::media_box`].
    pub fn trim_box(&self) -> Result<Option<Rect>> {
        self.clipped_box("TrimBox")
    }

    /// `/ArtBox` intersected with the crop box; defaults to the crop box.
    ///
    /// # Errors
    /// As [`Page::media_box`].
    pub fn art_box(&self) -> Result<Option<Rect>> {
        self.clipped_box("ArtBox")
    }

    /// `/Rotate`, own or inherited. A value that is not a multiple of 90 (or not a number)
    /// reads as [`Rotation::None`] with [`Warning::InvalidRotate`].
    ///
    /// # Errors
    /// [`lamina_cos::Error::Strict`] in strict mode for an invalid value;
    /// [`lamina_cos::Error::DataPending`] / [`lamina_cos::Error::Source`].
    pub fn rotate(&self) -> Result<Rotation> {
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
            self.doc().warn(Warning::InvalidRotate { object: self.object() })?;
            Ok(Rotation::None)
        }
    }

    /// `/UserUnit` (§14.11.2.1): the size of a default user-space unit in 1/72 inch; 1 when
    /// absent or not a positive number.
    ///
    /// # Errors
    /// [`lamina_cos::Error::DataPending`] / [`lamina_cos::Error::Source`].
    pub fn user_unit(&self) -> Result<f64> {
        let Some(value) = self.cos.dict.get("UserUnit") else {
            return Ok(1.0);
        };
        Ok(self
            .doc()
            .read(value)?
            .and_then(|v| v.as_f64())
            .filter(|u| u.is_finite() && *u > 0.0)
            .unwrap_or(1.0))
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
    /// [`lamina_cos::Error::DataPending`] / [`lamina_cos::Error::Source`].
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
    /// [`lamina_cos::Error::Strict`] in strict mode for a malformed label entry; errors
    /// reading the catalog.
    pub fn label(&self) -> Result<Option<String>> {
        Ok(self.pages.labels()?.label(self.index))
    }
}

/// `obj` as a rectangle of four numbers, tolerantly; `None` when it is anything else.
pub(crate) fn read_rect(doc: &Document, obj: &Object) -> Result<Option<Rect>> {
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
    Ok(Some(Rect::from_corners(x0, y0, x1, y1)))
}

/// A finite number, tolerantly.
pub(crate) fn number(doc: &Document, obj: &Object) -> Result<Option<f64>> {
    Ok(doc.read(obj)?.and_then(|n| n.as_f64()).filter(|n| n.is_finite()))
}
