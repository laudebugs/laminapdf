//! Annotations (ISO 32000-2 §12.5): a typed view of each entry of a page's `/Annots`, read
//! tolerantly (ADR 0003). The common entries of Table 166, the markup entries of Table 172
//! (§12.5.6.2), the Popup pairing (§12.5.6.14) and each subtype's geometry are read; every
//! other key stays in the dictionary, untouched, for the ops that edit it.
//!
//! An entry that isn't a dictionary is left out. An annotation stored as a direct dictionary
//! (Table 31 asks for indirect references) gets a path ID, which ops refuse to edit.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, DirectPath, ObjRef, Object, PathSegment, Result, Warning};

use crate::document::Document;
use crate::geometry::{UserSpacePoint, UserSpaceRect};
use crate::id::AnnotId;
use crate::link::quads;
use crate::page::{Page, number, read_rect};
use crate::values::read_name;

/// One annotation on a page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Annot {
    /// Its ID: the annotation object's, or a path ID for a direct dictionary.
    pub id: AnnotId,
    /// Its position in the page's `/Annots` array.
    pub index: usize,
    /// `/Subtype`.
    pub subtype: AnnotSubtype,
    /// `/Rect` in user space; `None` when absent or malformed.
    pub rect: Option<UserSpaceRect>,
    /// `/Contents`, decoded (§7.9.2.2).
    pub contents: Option<String>,
    /// `/NM`, decoded: a name unique on its page, which is only a hint (ADR 0005 §5).
    pub name: Option<String>,
    /// `/M` as stored: a PDF date or, as some writers leave it, any text.
    pub modified: Option<String>,
    /// `/F`.
    pub flags: AnnotFlags,
    /// `/C`: no components (transparent), one (gray), three (RGB) or four (CMYK).
    pub color: Vec<f64>,
    /// `/CA`, the constant opacity (§12.5.6.2; any annotation in PDF 2.0).
    pub opacity: Option<f64>,
    /// `/T`: the author, for a markup annotation.
    pub author: Option<String>,
    /// `/Subj`.
    pub subject: Option<String>,
    /// `/CreationDate` as stored.
    pub created: Option<String>,
    /// `/IRT`: the annotation this one replies to or is grouped with.
    pub in_reply_to: Option<AnnotId>,
    /// `/RT`: how it relates to [`Annot::in_reply_to`]; `Reply` when absent (Table 172).
    pub reply_type: ReplyType,
    /// The Popup annotation that shows this one's text: `/Popup`, else a Popup on the page
    /// whose `/Parent` names it.
    pub popup: Option<AnnotId>,
    /// For a Popup, its `/Parent`.
    pub parent: Option<AnnotId>,
    /// Whether `/AP` has a normal appearance (`/N`).
    pub has_appearance: bool,
    /// Whether it has `/RC`, rich-text contents (§12.7.4.4).
    pub has_rich_text: bool,
    /// The subtype's own geometry.
    pub geometry: AnnotGeometry,
}

/// An annotation's `/Subtype` (Table 171).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AnnotSubtype {
    /// `Text`, a note.
    Text,
    /// `Link`.
    Link,
    /// `FreeText`.
    FreeText,
    /// `Line`.
    Line,
    /// `Square`.
    Square,
    /// `Circle`.
    Circle,
    /// `Polygon`.
    Polygon,
    /// `PolyLine`.
    PolyLine,
    /// `Highlight`.
    Highlight,
    /// `Underline`.
    Underline,
    /// `Squiggly`.
    Squiggly,
    /// `StrikeOut`.
    StrikeOut,
    /// `Caret`.
    Caret,
    /// `Stamp`.
    Stamp,
    /// `Ink`.
    Ink,
    /// `Popup`.
    Popup,
    /// `FileAttachment`.
    FileAttachment,
    /// `Sound`.
    Sound,
    /// `Widget`, a form field's.
    Widget,
    /// `Redact`.
    Redact,
    /// Any other subtype, by its name's bytes (empty when `/Subtype` is missing).
    Other(Vec<u8>),
}

impl AnnotSubtype {
    fn from_name(name: &[u8]) -> Self {
        match name {
            b"Text" => Self::Text,
            b"Link" => Self::Link,
            b"FreeText" => Self::FreeText,
            b"Line" => Self::Line,
            b"Square" => Self::Square,
            b"Circle" => Self::Circle,
            b"Polygon" => Self::Polygon,
            b"PolyLine" => Self::PolyLine,
            b"Highlight" => Self::Highlight,
            b"Underline" => Self::Underline,
            b"Squiggly" => Self::Squiggly,
            b"StrikeOut" => Self::StrikeOut,
            b"Caret" => Self::Caret,
            b"Stamp" => Self::Stamp,
            b"Ink" => Self::Ink,
            b"Popup" => Self::Popup,
            b"FileAttachment" => Self::FileAttachment,
            b"Sound" => Self::Sound,
            b"Widget" => Self::Widget,
            b"Redact" => Self::Redact,
            other => Self::Other(other.to_vec()),
        }
    }

    /// The subtype's name as written in `/Subtype`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        match self {
            Self::Text => b"Text",
            Self::Link => b"Link",
            Self::FreeText => b"FreeText",
            Self::Line => b"Line",
            Self::Square => b"Square",
            Self::Circle => b"Circle",
            Self::Polygon => b"Polygon",
            Self::PolyLine => b"PolyLine",
            Self::Highlight => b"Highlight",
            Self::Underline => b"Underline",
            Self::Squiggly => b"Squiggly",
            Self::StrikeOut => b"StrikeOut",
            Self::Caret => b"Caret",
            Self::Stamp => b"Stamp",
            Self::Ink => b"Ink",
            Self::Popup => b"Popup",
            Self::FileAttachment => b"FileAttachment",
            Self::Sound => b"Sound",
            Self::Widget => b"Widget",
            Self::Redact => b"Redact",
            Self::Other(name) => name,
        }
    }
}

/// An annotation's `/F` flags (Table 167).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AnnotFlags(pub u32);

impl AnnotFlags {
    /// Don't show an annotation of an unknown subtype.
    ///
    /// Added in 0.2.0.
    pub const INVISIBLE: u32 = 1;
    /// Don't show or print it.
    ///
    /// Added in 0.2.0.
    pub const HIDDEN: u32 = 1 << 1;
    /// Print it.
    ///
    /// Added in 0.2.0.
    pub const PRINT: u32 = 1 << 2;
    /// Don't scale it with the page's zoom.
    ///
    /// Added in 0.2.0.
    pub const NO_ZOOM: u32 = 1 << 3;
    /// Don't rotate it with the page.
    ///
    /// Added in 0.2.0.
    pub const NO_ROTATE: u32 = 1 << 4;
    /// Print it but don't show it.
    ///
    /// Added in 0.2.0.
    pub const NO_VIEW: u32 = 1 << 5;
    /// Don't let a user interact with it.
    ///
    /// Added in 0.2.0.
    pub const READ_ONLY: u32 = 1 << 6;
    /// Don't let a user delete it or change its properties (its contents may change).
    ///
    /// Added in 0.2.0.
    pub const LOCKED: u32 = 1 << 7;
    /// Invert `NO_VIEW` on hover or selection.
    ///
    /// Added in 0.2.0.
    pub const TOGGLE_NO_VIEW: u32 = 1 << 8;
    /// Don't let a user change its contents.
    ///
    /// Added in 0.2.0.
    pub const LOCKED_CONTENTS: u32 = 1 << 9;

    /// Whether every bit of `flag` is set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn has(self, flag: u32) -> bool {
        self.0 & flag == flag
    }
}

/// How a markup annotation relates to its `/IRT` (Table 172).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReplyType {
    /// `/R`: a reply.
    #[default]
    Reply,
    /// `/Group`: grouped with it, as one unit.
    Group,
}

/// Where a subtype's own geometry puts it, in user space.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum AnnotGeometry {
    /// Nothing besides `/Rect`.
    #[default]
    None,
    /// `/QuadPoints` of text markup, four corners each, in the order stored: the standard's
    /// counter-clockwise order or Acrobat's (upper-left, upper-right, lower-left, lower-right).
    Quads(Vec<[UserSpacePoint; 4]>),
    /// `/InkList`: one point list per stroke.
    Ink(Vec<Vec<UserSpacePoint>>),
    /// `/Vertices` of a `Polygon` or `PolyLine`.
    Vertices(Vec<UserSpacePoint>),
    /// `/L` of a Line: its two ends.
    Line([UserSpacePoint; 2]),
    /// `/CL` of a `FreeText` callout: two or three points.
    Callout(Vec<UserSpacePoint>),
}

impl Document {
    /// The annotations of `page`, in the order of its `/Annots`.
    ///
    /// Two annotations with the same `/NM` on the page raise `annot-name-duplicate` (on the
    /// second); both are kept.
    ///
    /// # Errors
    /// Only errors that must propagate ([`laminapdf_cos::Error::must_propagate`]): an
    /// unreadable entry is left out.
    ///
    /// Added in 0.2.0.
    pub fn annotations(&self, page: &Page<'_>) -> Result<Vec<Annot>> {
        let found = self.annotations_quietly(page)?;
        self.check_names(&found)?;
        Ok(found)
    }

    /// [`Document::annotations`] without its warning, for the ops, which read every page's
    /// annotations to find one: a duplicate name elsewhere must not stop an edit, strict mode
    /// included.
    ///
    /// # Errors
    /// As [`Document::annotations`].
    ///
    /// Added in 0.2.0.
    pub fn annotations_quietly(&self, page: &Page<'_>) -> Result<Vec<Annot>> {
        let Some(value) = page.dict().get("Annots") else {
            return Ok(Vec::new());
        };
        let Some(array) = self.read(value)? else {
            return Ok(Vec::new());
        };
        let array_ref = value.as_ref();
        let mut found = Vec::new();
        for (index, entry) in array.as_array().unwrap_or_default().iter().enumerate() {
            let Some(dict) = self.read(entry)? else { continue };
            let Some(dict) = dict.as_dict() else { continue };
            let id = match entry.as_ref() {
                Some(r) => AnnotId::object(r),
                None => AnnotId::path(&direct_path(page, array_ref, index)),
            };
            found.push(self.annotation(id, index, dict)?);
        }
        pair_popups(&mut found);
        Ok(found)
    }

    fn annotation(&self, id: AnnotId, index: usize, dict: &Dict) -> Result<Annot> {
        let subtype = AnnotSubtype::from_name(&read_name(self, dict.get("Subtype"))?.unwrap_or_default());
        let flags = match dict.get("F") {
            Some(f) => self.read(f)?.and_then(|f| f.as_int()).unwrap_or(0),
            None => 0,
        };
        let geometry = self.geometry(&subtype, dict)?;
        Ok(Annot {
            id,
            index,
            rect: match dict.get("Rect") {
                Some(rect) => read_rect(self, rect)?,
                None => None,
            },
            contents: self.text_entry(dict, "Contents")?,
            name: self.text_entry(dict, "NM")?,
            modified: self.text_entry(dict, "M")?,
            flags: AnnotFlags(u32::try_from(flags).unwrap_or(0)),
            color: self.numbers(dict.get("C"))?.unwrap_or_default(),
            opacity: match dict.get("CA") {
                Some(ca) => number(self, ca)?,
                None => None,
            },
            author: self.text_entry(dict, "T")?,
            subject: self.text_entry(dict, "Subj")?,
            created: self.text_entry(dict, "CreationDate")?,
            in_reply_to: dict.get("IRT").and_then(Object::as_ref).map(AnnotId::object),
            reply_type: match read_name(self, dict.get("RT"))?.as_deref() {
                Some(b"Group") => ReplyType::Group,
                _ => ReplyType::Reply,
            },
            popup: dict.get("Popup").and_then(Object::as_ref).map(AnnotId::object),
            parent: if subtype == AnnotSubtype::Popup {
                dict.get("Parent").and_then(Object::as_ref).map(AnnotId::object)
            } else {
                None
            },
            has_appearance: self.has_normal_appearance(dict)?,
            has_rich_text: dict.get("RC").is_some(),
            subtype,
            geometry,
        })
    }

    fn geometry(&self, subtype: &AnnotSubtype, dict: &Dict) -> Result<AnnotGeometry> {
        Ok(match subtype {
            AnnotSubtype::Highlight
            | AnnotSubtype::Underline
            | AnnotSubtype::Squiggly
            | AnnotSubtype::StrikeOut
            | AnnotSubtype::Redact => AnnotGeometry::Quads(quads(self, dict.get("QuadPoints"))?),
            AnnotSubtype::Ink => {
                let mut strokes = Vec::new();
                if let Some(list) = dict.get("InkList")
                    && let Some(list) = self.read(list)?
                {
                    for stroke in list.as_array().unwrap_or_default() {
                        strokes.push(self.points(Some(stroke))?.unwrap_or_default());
                    }
                }
                AnnotGeometry::Ink(strokes)
            }
            AnnotSubtype::Polygon | AnnotSubtype::PolyLine => {
                AnnotGeometry::Vertices(self.points(dict.get("Vertices"))?.unwrap_or_default())
            }
            AnnotSubtype::Line => match self.points(dict.get("L"))?.as_deref() {
                Some(&[a, b]) => AnnotGeometry::Line([a, b]),
                _ => AnnotGeometry::None,
            },
            AnnotSubtype::FreeText => match self.points(dict.get("CL"))? {
                Some(points) if points.len() >= 2 => AnnotGeometry::Callout(points),
                _ => AnnotGeometry::None,
            },
            _ => AnnotGeometry::None,
        })
    }

    /// An array of numbers; `None` when absent or when any item isn't a number.
    fn numbers(&self, value: Option<&Object>) -> Result<Option<Vec<f64>>> {
        let Some(value) = value else { return Ok(None) };
        let Some(array) = self.read(value)? else {
            return Ok(None);
        };
        let Some(items) = array.as_array() else { return Ok(None) };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match number(self, item)? {
                Some(v) => out.push(v),
                None => return Ok(None),
            }
        }
        Ok(Some(out))
    }

    /// An array of coordinate pairs; `None` unless it holds an even count of numbers.
    fn points(&self, value: Option<&Object>) -> Result<Option<Vec<UserSpacePoint>>> {
        Ok(self.numbers(value)?.filter(|n| n.len().is_multiple_of(2)).map(|n| {
            let (pairs, _) = n.as_chunks::<2>();
            pairs.iter().map(|&[x, y]| UserSpacePoint::new(x, y)).collect()
        }))
    }

    fn has_normal_appearance(&self, dict: &Dict) -> Result<bool> {
        let Some(ap) = dict.get("AP") else { return Ok(false) };
        let Some(ap) = self.read(ap)? else { return Ok(false) };
        Ok(ap.as_dict().and_then(|ap| ap.get("N")).is_some())
    }

    fn check_names(&self, found: &[Annot]) -> Result<()> {
        let mut seen = BTreeSet::new();
        for annot in found {
            if let Some(name) = &annot.name
                && !seen.insert(name.as_str())
            {
                self.warn(Warning::AnnotNameDuplicate {
                    object: annot.id.as_lamina_id().obj_ref().map(|r: ObjRef| r.num),
                })?;
            }
        }
        Ok(())
    }
}

/// The path of the direct annotation at `index`: inside the indirect `/Annots` array, else
/// inside the page's own dictionary.
fn direct_path(page: &Page<'_>, array: Option<ObjRef>, index: usize) -> DirectPath {
    match array {
        Some(array) => DirectPath::new(Some(array), vec![PathSegment::Index(index)]),
        None => DirectPath::new(
            page.obj_ref(),
            vec![PathSegment::Key("Annots"), PathSegment::Index(index)],
        ),
    }
}

/// Gives each annotation without a `/Popup` the Popup on the page whose `/Parent` names it.
fn pair_popups(found: &mut [Annot]) {
    let pairs: Vec<(AnnotId, AnnotId)> = found
        .iter()
        .filter_map(|a| Some((a.parent.clone()?, a.id.clone())))
        .collect();
    for (parent, popup) in pairs {
        if let Some(owner) = found.iter_mut().find(|a| a.id == parent && a.popup.is_none()) {
            owner.popup = Some(popup);
        }
    }
}
