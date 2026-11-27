//! The ops that put a page mark on pages or replace it: [`AddWatermark`], [`AddBackground`],
//! [`SetHeaderFooter`], [`ApplyBates`] and [`UpdatePageMark`]. Each builds a [`PageMark`] and
//! hands it to the one path of `marks`.

use laminapdf_model::{PageId, PdfDate};
use serde::{Deserialize, Serialize};

use super::mark_types::{BATES_DIGITS, bates_digits, is_bates_digits};
use super::mark_types::{BatesSpec, HeaderFooterSpec, MarkContent, MarkMargins, PageMark, Slot, WatermarkSpec};
use super::marks::{Mode, Put, put};
use super::spec::{ContentId, is_twelve, is_zero, twelve};
use crate::annot::{AnnotColor, AnnotFont, Appearances, Opacity};
use crate::blob::BlobStore;
use crate::error::Result;
use crate::op::Effect;
use crate::wire;

macro_rules! backdrop {
    ($(#[$doc:meta])* $name:ident, $variant:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        #[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
        #[non_exhaustive]
        pub struct $name {
            /// The mark's ID, the same on every page it is on and unique among each page's added
            /// objects and marks.
            pub id: ContentId,
            /// The pages, in the order the document lists them or not, without repeats.
            #[serde(with = "wire::page_ids")]
            #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
            pub pages: Vec<PageId>,
            /// What it shows.
            pub mark: MarkContent,
            /// How opaque it is; opaque when absent.
            #[serde(default, skip_serializing_if = "Opacity::is_opaque")]
            #[cfg_attr(feature = "tsify", tsify(optional))]
            pub opacity: Opacity,
            /// How far it is turned counterclockwise as shown, in degrees; 0 when absent.
            #[serde(default, skip_serializing_if = "is_zero")]
            #[cfg_attr(feature = "tsify", tsify(optional))]
            pub angle: f64,
            /// Whether a reader's layers panel can switch it off.
            #[serde(default, skip_serializing_if = "wire::is_false")]
            #[cfg_attr(feature = "tsify", tsify(optional))]
            pub hideable: bool,
            /// The forms' `/LastModified`.
            #[serde(with = "wire::date")]
            #[cfg_attr(feature = "tsify", tsify(type = "string"))]
            pub modified: PdfDate,
        }

        // Sound: as for `MarkMargins`.
        impl Eq for $name {}

        impl $name {
            /// The op that puts `mark` on `pages` as `id`, opaque, upright and always shown.
            ///
            /// Added in 0.7.0.
            #[must_use]
            pub const fn new(id: ContentId, pages: Vec<PageId>, mark: MarkContent, modified: PdfDate) -> Self {
                Self {
                    id,
                    pages,
                    mark,
                    opacity: Opacity::OPAQUE,
                    angle: 0.0,
                    hideable: false,
                    modified,
                }
            }

            /// The same op with `opacity`.
            ///
            /// Added in 0.7.0.
            #[must_use]
            pub const fn with_opacity(mut self, opacity: Opacity) -> Self {
                self.opacity = opacity;
                self
            }

            /// The same op turned `angle` degrees counterclockwise as shown.
            ///
            /// Added in 0.7.0.
            #[must_use]
            pub const fn with_angle(mut self, angle: f64) -> Self {
                self.angle = angle;
                self
            }

            /// The same op in an optional content group a reader's layers panel can switch off.
            ///
            /// Added in 0.7.0.
            #[must_use]
            pub const fn hideable(mut self) -> Self {
                self.hideable = true;
                self
            }

            /// The mark as a page keeps it.
            fn spec(&self) -> PageMark {
                PageMark::$variant(WatermarkSpec {
                    mark: self.mark.clone(),
                    opacity: self.opacity,
                    angle: self.angle,
                    hideable: self.hideable,
                })
            }

            pub(crate) fn apply_to(
                &self,
                doc: &mut laminapdf_model::Document,
                appearances: Option<&dyn Appearances>,
                blobs: &BlobStore,
            ) -> Result<Effect> {
                let mark = self.spec();
                put(
                    doc,
                    appearances,
                    blobs,
                    &Put {
                        id: &self.id,
                        pages: &self.pages,
                        mark: &mark,
                        modified: &self.modified,
                        mode: Mode::Add,
                    },
                )
            }
        }
    };
}

backdrop! {
    /// Puts a watermark on pages (ISO 32000-2 §14.8.2.2.2), drawn in front of each page's content:
    /// text centred on the page and turned by `angle`, or an image shown whole inside most of the
    /// page. It is one Form XObject a page, found again after a save by `id`, which is the same on
    /// every page. In a tagged file the drawing is an artifact (`/Type /Pagination /Subtype
    /// /Watermark`), so screen readers skip it. With `hideable`, the form names an optional
    /// content group (§8.11) with `/Usage /PageElement /Subtype /FG`, so a reader's layers panel
    /// switches it off.
    ///
    /// An ID another added object or mark on one of the pages has is `content-id-in-use`; text no
    /// font has a glyph for is `content-glyph-missing`, and a font or an image that can't be read
    /// is `content-font-invalid` or `content-object-invalid`; a document without a generator is
    /// `appearance-unavailable`. The text is one line per paragraph, set in a box 90% of the
    /// page's diagonal wide, so a longer line wraps and what doesn't fit is clipped.
    ///
    /// On the wire: `{ "op": "addWatermark", "args": { "id": "draft", "pages": ["p3.0"], "mark":
    /// { "text": { "text": "DRAFT", "size": 96 } }, "opacity": 0.3, "angle": 45, "modified":
    /// "D:20261118120000Z" } }`.
    ///
    /// Added in 0.7.0.
    AddWatermark, Watermark
}

backdrop! {
    /// Puts a background on pages (ISO 32000-2 §14.8.2.2.2), drawn behind each page's content, over
    /// the backgrounds added before: text centred on the page, or an image that covers it (its
    /// excess clipped to the page). Otherwise as [`AddWatermark`]; a tagged file's artifact is
    /// `/Type /Background`, and a hideable background's group is `/Subtype /BG`.
    ///
    /// On the wire: `{ "op": "addBackground", "args": { "id": "letterhead", "pages": ["p3.0"],
    /// "mark": { "image": { "blob": "…" } }, "modified": "D:20261118120000Z" } }`.
    ///
    /// Added in 0.7.0.
    AddBackground, Background
}

/// Puts a header and footer on pages: up to six slots of text, three across the top and three
/// across the bottom, each with the fields `{page}` and `{pages}` (the page's number and the
/// document's page count, as the flowing layout's headers use), `{label}` (the page's label, else
/// its number), and `{date}` and `{file}`, which stand for the strings the op carries. The fields
/// are filled when the op applies, so a page moved later keeps its number; apply the op again
/// (`UpdatePageMark`) to renumber.
///
/// Each slot is a one-line box as wide as the page between its margins, aligned left, centre or
/// right by its column, so the three slots on an edge overlap and a long text runs past its third
/// of the page; a text wider than the whole line wraps and what doesn't fit is clipped. In a tagged file
/// the drawing is an artifact, a header, a footer or both. Errors as for [`AddWatermark`].
///
/// On the wire: `{ "op": "setHeaderFooter", "args": { "id": "folio", "pages": ["p3.0", "p4.0"],
/// "slots": [null, null, null, null, "Page {page} of {pages}", null], "size": 9, "modified":
/// "D:20261118120000Z" } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetHeaderFooter {
    /// The mark's ID, the same on every page.
    pub id: ContentId,
    /// The pages, without repeats.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// The text of each slot, in the order of [`Slot::ALL`]; `null` leaves a slot empty. At least
    /// one has text.
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "[string | null, string | null, string | null, string | null, string | null, string | null]")
    )]
    pub slots: [Option<String>; 6],
    /// The font, then its fallbacks; Helvetica when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string[]"))]
    pub fonts: Vec<AnnotFont>,
    /// The size in points, from 1 to 500; 12 when absent.
    #[serde(default = "twelve", skip_serializing_if = "is_twelve")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub size: f64,
    /// The colour; black when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub color: Option<AnnotColor>,
    /// How far the text keeps from the page's edges: 36 points when absent.
    #[serde(default, skip_serializing_if = "MarkMargins::is_default")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub margins: MarkMargins,
    /// What `{date}` stands for; empty when absent.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub date: String,
    /// What `{file}` stands for; empty when absent.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub file: String,
    /// The forms' `/LastModified`.
    #[serde(with = "wire::date")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub modified: PdfDate,
}

// Sound: as for `MarkMargins`.
impl Eq for SetHeaderFooter {}

impl SetHeaderFooter {
    /// The op that puts `slots` on `pages` as `id`, in 12-point black Helvetica.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(id: ContentId, pages: Vec<PageId>, slots: [Option<String>; 6], modified: PdfDate) -> Self {
        Self {
            id,
            pages,
            slots,
            fonts: Vec::new(),
            size: 12.0,
            color: None,
            margins: MarkMargins::all(36.0),
            date: String::new(),
            file: String::new(),
            modified,
        }
    }

    /// The same op in `fonts` (the font, then fallbacks) at `size` points.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_fonts(mut self, fonts: Vec<AnnotFont>, size: f64) -> Self {
        self.fonts = fonts;
        self.size = size;
        self
    }

    /// The same op in `color`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_color(mut self, color: AnnotColor) -> Self {
        self.color = Some(color);
        self
    }

    /// The same op with `margins`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_margins(mut self, margins: MarkMargins) -> Self {
        self.margins = margins;
        self
    }

    /// The same op with `{date}` and `{file}` standing for `date` and `file`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_fields(mut self, date: impl Into<String>, file: impl Into<String>) -> Self {
        self.date = date.into();
        self.file = file.into();
        self
    }

    fn spec(&self) -> PageMark {
        PageMark::HeaderFooter(HeaderFooterSpec {
            slots: self.slots.clone(),
            fonts: self.fonts.clone(),
            size: self.size,
            color: self.color,
            margins: self.margins,
            date: self.date.clone(),
            file: self.file.clone(),
        })
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<Effect> {
        put(
            doc,
            appearances,
            blobs,
            &Put {
                id: &self.id,
                pages: &self.pages,
                mark: &self.spec(),
                modified: &self.modified,
                mode: Mode::Add,
            },
        )
    }
}

/// Numbers pages the way a law firm labels evidence: a Bates number (`prefix`, the number padded to
/// `digits`, `suffix`) in one slot of each page, counting up from `start` over `pages` in the
/// order the op lists them. Numbering many documents in turn is one op each, each with its own
/// `start`, which a batch runner does. The number is an artifact in a tagged file (`/Subtype
/// /Bates`). A number with more digits than `digits` is not cut. Errors as for [`AddWatermark`].
///
/// On the wire: `{ "op": "applyBates", "args": { "id": "bates", "pages": ["p3.0", "p4.0"],
/// "prefix": "ACME-", "start": 1, "digits": 6, "slot": "bottomRight", "size": 9, "modified":
/// "D:20261118120000Z" } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct ApplyBates {
    /// The mark's ID, the same on every page.
    pub id: ContentId,
    /// The pages, in numbering order, without repeats.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// Text before the number.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub prefix: String,
    /// The first page's number.
    pub start: u64,
    /// The number is padded with zeros to this many digits, up to 20; 6 when absent.
    #[serde(default = "bates_digits", skip_serializing_if = "is_bates_digits")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub digits: u8,
    /// Text after the number.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub suffix: String,
    /// Where the number sits.
    pub slot: Slot,
    /// The font, then its fallbacks; Helvetica when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string[]"))]
    pub fonts: Vec<AnnotFont>,
    /// The size in points, from 1 to 500; 12 when absent.
    #[serde(default = "twelve", skip_serializing_if = "is_twelve")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub size: f64,
    /// The colour; black when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub color: Option<AnnotColor>,
    /// How far the number keeps from the page's edges: 36 points when absent.
    #[serde(default, skip_serializing_if = "MarkMargins::is_default")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub margins: MarkMargins,
    /// The forms' `/LastModified`.
    #[serde(with = "wire::date")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub modified: PdfDate,
}

// Sound: as for `MarkMargins`.
impl Eq for ApplyBates {}

impl ApplyBates {
    /// The op that numbers `pages` from `start` in `slot` as `id`, six digits, in 12-point black
    /// Helvetica, with no prefix or suffix.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(id: ContentId, pages: Vec<PageId>, start: u64, slot: Slot, modified: PdfDate) -> Self {
        Self {
            id,
            pages,
            prefix: String::new(),
            start,
            digits: BATES_DIGITS,
            suffix: String::new(),
            slot,
            fonts: Vec::new(),
            size: 12.0,
            color: None,
            margins: MarkMargins::all(36.0),
            modified,
        }
    }

    /// The same op with `prefix` before and `suffix` after the number, padded to `digits`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_text(mut self, prefix: impl Into<String>, digits: u8, suffix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self.digits = digits;
        self.suffix = suffix.into();
        self
    }

    /// The same op in `fonts` (the font, then fallbacks) at `size` points.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_fonts(mut self, fonts: Vec<AnnotFont>, size: f64) -> Self {
        self.fonts = fonts;
        self.size = size;
        self
    }

    /// The same op in `color`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_color(mut self, color: AnnotColor) -> Self {
        self.color = Some(color);
        self
    }

    /// The same op with `margins`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_margins(mut self, margins: MarkMargins) -> Self {
        self.margins = margins;
        self
    }

    fn spec(&self) -> PageMark {
        PageMark::Bates(BatesSpec {
            prefix: self.prefix.clone(),
            start: self.start,
            digits: self.digits,
            suffix: self.suffix.clone(),
            slot: self.slot,
            fonts: self.fonts.clone(),
            size: self.size,
            color: self.color,
            margins: self.margins,
        })
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<Effect> {
        put(
            doc,
            appearances,
            blobs,
            &Put {
                id: &self.id,
                pages: &self.pages,
                mark: &self.spec(),
                modified: &self.modified,
                mode: Mode::Add,
            },
        )
    }
}

/// Replaces a page mark Lamina PDF added with a new one of the same kind on the pages named: the
/// new payload in place of the old, drawn again where the old one was drawn in the page's stacking
/// order, under the same ID. A hideable mark keeps its optional content group; one made
/// unhideable leaves the layers panel. Fields in a header's `{page}` and the like are filled
/// again, and a Bates number counts from the new `start` over `pages` in the order given.
///
/// A page that draws no mark with the ID is `unknown-page-mark`; a mark of another kind than the
/// one stored is `invalid-op`. The form's `/LastModified` becomes `modified`. Errors otherwise as
/// for [`AddWatermark`].
///
/// On the wire: `{ "op": "updatePageMark", "args": { "id": "draft", "pages": ["p3.0"], "mark":
/// { "watermark": { "mark": { "text": { "text": "FINAL", "size": 96 } }, "angle": 45 } },
/// "modified": "D:20261118120000Z" } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct UpdatePageMark {
    /// The mark.
    pub id: ContentId,
    /// The pages to change it on, without repeats.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// What the mark is now.
    pub mark: PageMark,
    /// The forms' new `/LastModified`.
    #[serde(with = "wire::date")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub modified: PdfDate,
}

// Sound: as for `MarkMargins`.
impl Eq for UpdatePageMark {}

impl UpdatePageMark {
    /// The op that makes the mark `id` on `pages` into `mark`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(id: ContentId, pages: Vec<PageId>, mark: PageMark, modified: PdfDate) -> Self {
        Self {
            id,
            pages,
            mark,
            modified,
        }
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<Effect> {
        put(
            doc,
            appearances,
            blobs,
            &Put {
                id: &self.id,
                pages: &self.pages,
                mark: &self.mark,
                modified: &self.modified,
                mode: Mode::Update,
            },
        )
    }
}
