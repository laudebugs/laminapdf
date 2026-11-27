//! Page marks (E6 plan §2.3): watermarks, backgrounds, headers and footers, and Bates numbers,
//! which [`AddWatermark`], [`AddBackground`], [`SetHeaderFooter`] and [`ApplyBates`] put on many
//! pages at once, [`UpdatePageMark`] replaces and [`DeletePageMark`] takes off, before or after a
//! save. [`page_marks`] lists what a page holds.
//!
//! A mark is one Form XObject per page, drawn by one small stream of its own in the page's
//! `/Contents` like every added object (`place`): a form in page space that draws the mark's text
//! and pictures, which the generator draws as the content ops do. The mark's ID is the same on
//! every page it is on, so a page's marks and its added objects share one namespace. In a tagged
//! file the drawing is an artifact (§14.8.2.2) and never owns structure.

use std::collections::{BTreeSet, HashMap};

use laminapdf_content::{PageContentWriter, ResourceName};
use laminapdf_cos::{Dict, ObjRef, Object, Stream};
use laminapdf_model::{Matrix, PageId, PdfDate};
use laminapdf_syntax::format_real;
use serde::{Deserialize, Serialize};

use super::mark_layout::{self, Child, Face, Fields, Sheet};
use super::ocg::{self, Element};
use super::place::{Mark, PageArtifact, Placed, Placement, Private, drawing, fade, mark_form, place_form, scan};
use super::spec::{ContentId, ContentLayer, is_twelve, twelve};
use super::{draw, page_parts};
use crate::annot::{AnnotColor, AnnotFont, Appearances, Opacity};
use crate::blob::{BlobId, BlobStore};
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::parent_tree::ParentTree;
use crate::wire;

/// The most pages one mark op names.
const MAX_PAGES: usize = 100_000;
/// The most characters a slot, a date, a file name, a prefix or a suffix holds.
const MAX_TEXT: usize = 1_000;
/// The widest a Bates number is padded.
const MAX_DIGITS: u8 = 20;

/// What `invalid-op` says of a page mark whose pages, text, size, angle or margins are missing or
/// out of range.
const INVALID: &str = "a page mark needs pages without repeats, some text (or an image), a size from 1 to 500, finite angle and margins, and at most 20 digits";

/// Where a page's mark text sits: one of the six slots of a header and a footer, three columns
/// wide, left to right.
///
/// On the wire: `topLeft`, `topCenter`, `topRight`, `bottomLeft`, `bottomCenter` or `bottomRight`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum Slot {
    /// The top edge, at the left margin.
    TopLeft,
    /// The top edge, centred.
    TopCenter,
    /// The top edge, ending at the right margin.
    TopRight,
    /// The bottom edge, at the left margin.
    BottomLeft,
    /// The bottom edge, centred.
    BottomCenter,
    /// The bottom edge, ending at the right margin.
    BottomRight,
}

impl Slot {
    /// The six slots in the order [`SetHeaderFooter::slots`] lists them.
    ///
    /// Added in 0.7.0.
    pub const ALL: [Self; 6] = [
        Self::TopLeft,
        Self::TopCenter,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomCenter,
        Self::BottomRight,
    ];

    /// Which of the three columns the slot is in: 0 left, 1 centre, 2 right.
    pub(super) const fn column(self) -> u8 {
        match self {
            Self::TopLeft | Self::BottomLeft => 0,
            Self::TopCenter | Self::BottomCenter => 1,
            Self::TopRight | Self::BottomRight => 2,
        }
    }

    /// Whether the slot is on the top edge.
    pub(super) const fn is_top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopCenter | Self::TopRight)
    }
}

/// How far a mark's text keeps from the page's edges, in points: 36, half an inch, on every side
/// unless the op says otherwise.
///
/// On the wire: `{ "top": 36, "right": 36, "bottom": 36, "left": 36 }`, each key optional.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct MarkMargins {
    /// From the top edge.
    pub top: f64,
    /// From the right edge.
    pub right: f64,
    /// From the bottom edge.
    pub bottom: f64,
    /// From the left edge.
    pub left: f64,
}

// Sound: an op holding a number that isn't finite is refused before it writes (`invalid-op`).
impl Eq for MarkMargins {}

impl Default for MarkMargins {
    fn default() -> Self {
        Self::all(36.0)
    }
}

impl MarkMargins {
    /// The same margin on every side.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn all(points: f64) -> Self {
        Self {
            top: points,
            right: points,
            bottom: points,
            left: points,
        }
    }

    fn is_default(&self) -> bool {
        *self == Self::default()
    }

    fn is_valid(&self) -> bool {
        [self.top, self.right, self.bottom, self.left]
            .iter()
            .all(|m| m.is_finite() && (0.0..=10_000.0).contains(m))
    }
}

/// What a watermark or a background shows: a line of text, or a picture.
///
/// On the wire the kind is the only key: `{ "text": { "text": "DRAFT", "size": 96, "color": [0.8,
/// 0, 0] } }` or `{ "image": { "blob": "…" } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum MarkContent {
    /// Text, centred on the page, one line to a paragraph (a line feed starts one).
    Text {
        /// The text.
        text: String,
        /// The font, then its fallbacks; Helvetica when empty (as for
        /// [`AddText`](crate::AddText)).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "tsify", tsify(optional, type = "string[]"))]
        fonts: Vec<AnnotFont>,
        /// The size in points, from 1 to 500.
        size: f64,
        /// The colour; black when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "tsify", tsify(optional))]
        color: Option<AnnotColor>,
    },
    /// A JPEG or PNG image from the blob store: a watermark shows it whole inside most of the page,
    /// a background covers the page with it.
    Image {
        /// The image's bytes.
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        blob: BlobId,
    },
}

// Sound: as for `MarkMargins`.
impl Eq for MarkContent {}

/// A watermark's or a background's look.
///
/// On the wire: `{ "mark": { "text": { "text": "DRAFT", "size": 96 } }, "opacity": 0.3, "angle": 45,
/// "hideable": true }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct WatermarkSpec {
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
    /// Whether a reader's layers panel can switch it off: it is in an optional content group.
    #[serde(default, skip_serializing_if = "is_false")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub hideable: bool,
}

// Sound: as for `MarkMargins`.
impl Eq for WatermarkSpec {}

impl WatermarkSpec {
    /// A mark showing `mark`, opaque, upright and always shown.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(mark: MarkContent) -> Self {
        Self {
            mark,
            opacity: Opacity::OPAQUE,
            angle: 0.0,
            hideable: false,
        }
    }
}

/// What a header and footer says, as a page mark holds it.
///
/// On the wire: `{ "slots": [null, null, null, null, "Page {page} of {pages}", null], "size": 9,
/// "date": "27 Nov 2026" }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct HeaderFooterSpec {
    /// The text of each slot, in the order of [`Slot::ALL`], with its fields (`{page}`, `{pages}`,
    /// `{label}`, `{date}`, `{file}`; `{{` and `}}` for braces); `null` leaves a slot empty.
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
    /// How far the text keeps from the page's edges.
    #[serde(default, skip_serializing_if = "MarkMargins::is_default")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub margins: MarkMargins,
    /// What `{date}` stands for, as the caller wants it written; empty when absent.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub date: String,
    /// What `{file}` stands for; empty when absent.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub file: String,
}

// Sound: as for `MarkMargins`.
impl Eq for HeaderFooterSpec {}

/// A Bates number, as a page mark holds it. On the page itself the stored `start` is the page's
/// own number, so [`page_marks`] shows what each page carries.
///
/// On the wire: `{ "prefix": "ACME-", "start": 1, "digits": 6, "slot": "bottomRight", "size": 9 }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct BatesSpec {
    /// Text before the number.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub prefix: String,
    /// The first page's number; each page after it counts one more.
    pub start: u64,
    /// The number is padded with zeros to this many digits (up to 20); a longer number is not cut.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
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
    /// How far the number keeps from the page's edges.
    #[serde(default, skip_serializing_if = "MarkMargins::is_default")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub margins: MarkMargins,
}

// Sound: as for `MarkMargins`.
impl Eq for BatesSpec {}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")]
fn is_false(v: &bool) -> bool {
    !*v
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")]
fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")]
const fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}

/// What a page mark is, as [`UpdatePageMark`] replaces it and [`page_marks`] lists it: its kind
/// is the only key on the wire, `{ "watermark": { … } }`, `{ "background": { … } }`,
/// `{ "headerFooter": { … } }` or `{ "bates": { … } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum PageMark {
    /// A watermark, in front of the page's content.
    Watermark(WatermarkSpec),
    /// A background, behind the page's content.
    Background(WatermarkSpec),
    /// A header and footer.
    HeaderFooter(HeaderFooterSpec),
    /// A Bates number.
    Bates(BatesSpec),
}

// Sound: as for `MarkMargins`.
impl Eq for PageMark {}

impl PageMark {
    /// The kind's name, as `/PieceInfo` stores it.
    const fn kind(&self) -> &'static str {
        match self {
            Self::Watermark(_) => "Watermark",
            Self::Background(_) => "Background",
            Self::HeaderFooter(_) => "HeaderFooter",
            Self::Bates(_) => "Bates",
        }
    }

    const fn layer(&self) -> ContentLayer {
        match self {
            Self::Background(_) => ContentLayer::Behind,
            _ => ContentLayer::Front,
        }
    }

    fn opacity(&self) -> f64 {
        match self {
            Self::Watermark(w) | Self::Background(w) => w.opacity.get(),
            _ => 1.0,
        }
    }

    fn hideable(&self) -> bool {
        matches!(self, Self::Watermark(w) | Self::Background(w) if w.hideable)
    }

    /// What a tagged file's artifact says the mark is.
    fn artifact(&self) -> PageArtifact {
        match self {
            Self::Watermark(_) => PageArtifact::Watermark,
            Self::Background(_) => PageArtifact::Background,
            Self::Bates(_) => PageArtifact::Bates,
            Self::HeaderFooter(h) => {
                let (top, bottom) = h
                    .slots
                    .iter()
                    .zip(Slot::ALL)
                    .filter(|(t, _)| t.is_some())
                    .fold((false, false), |(top, bottom), (_, slot)| {
                        (top || slot.is_top(), bottom || !slot.is_top())
                    });
                match (top, bottom) {
                    (true, false) => PageArtifact::Header,
                    (false, true) => PageArtifact::Footer,
                    _ => PageArtifact::Pagination,
                }
            }
        }
    }

    /// The layers panel's name for the mark's group.
    fn group_name(&self) -> String {
        match self {
            Self::Watermark(w) | Self::Background(w) => {
                let what = if matches!(self, Self::Watermark(_)) {
                    "Watermark"
                } else {
                    "Background"
                };
                match &w.mark {
                    MarkContent::Text { text, .. } => {
                        let line: String = text.lines().next().unwrap_or_default().chars().take(40).collect();
                        format!("{what}: {line}")
                    }
                    MarkContent::Image { .. } => format!("{what} image"),
                }
            }
            Self::HeaderFooter(_) => "Header and footer".to_owned(),
            Self::Bates(_) => "Bates number".to_owned(),
        }
    }

    /// Whether the numbers and text are in range, before any page is looked at.
    fn is_valid(&self) -> bool {
        let text_ok = |s: &str| s.chars().count() <= MAX_TEXT;
        match self {
            Self::Watermark(w) | Self::Background(w) => {
                w.angle.is_finite()
                    && w.angle.abs() <= 360.0
                    && match &w.mark {
                        MarkContent::Text { text, .. } => text_ok(text),
                        MarkContent::Image { .. } => true,
                    }
            }
            Self::HeaderFooter(h) => {
                h.margins.is_valid()
                    && h.slots.iter().flatten().all(|s| text_ok(s))
                    && h.slots.iter().flatten().any(|s| !s.trim().is_empty())
                    && text_ok(&h.date)
                    && text_ok(&h.file)
            }
            Self::Bates(b) => {
                b.margins.is_valid() && b.digits <= MAX_DIGITS && text_ok(&b.prefix) && text_ok(&b.suffix)
            }
        }
    }
}

/// What the page the mark goes on looks like to the layout.
struct PageContext<'a> {
    sheet: Sheet,
    /// The page's 0-based place in the document.
    index: usize,
    pages: usize,
    label: &'a str,
    /// The page's 0-based place among the pages the op names, which Bates numbers count by.
    ordinal: usize,
}

/// What a mark draws on one page: its text and pictures, each with the matrix that places it.
fn children(
    mark: &PageMark,
    page: &PageContext<'_>,
    appearances: &dyn Appearances,
    blobs: &BlobStore,
) -> Result<Vec<Child>> {
    let sheet = page.sheet;
    Ok(match mark {
        PageMark::Watermark(w) | PageMark::Background(w) => match &w.mark {
            MarkContent::Text {
                text,
                fonts,
                size,
                color,
            } => vec![mark_layout::centred_text(
                text,
                w.angle,
                sheet,
                Face {
                    fonts,
                    size: *size,
                    color: *color,
                },
            )],
            MarkContent::Image { blob } => {
                let bytes = blobs.get(blob).ok_or(Error::UnknownBlob(*blob))?;
                let pixels = appearances.image_size(bytes).ok_or(Error::ContentObjectInvalid)?;
                let cover = matches!(mark, PageMark::Background(_));
                vec![mark_layout::centred_image(*blob, pixels, cover, w.angle, sheet)]
            }
        },
        PageMark::HeaderFooter(h) => {
            let fields = Fields {
                page: page.index + 1,
                pages: page.pages,
                label: page.label,
                date: &h.date,
                file: &h.file,
            };
            let face = Face {
                fonts: &h.fonts,
                size: h.size,
                color: h.color,
            };
            Slot::ALL
                .into_iter()
                .zip(&h.slots)
                .filter_map(|(slot, text)| {
                    let text = mark_layout::fill_fields(text.as_deref()?, &fields);
                    mark_layout::slot_child(&text, slot, h.margins, sheet, face)
                })
                .collect()
        }
        PageMark::Bates(b) => {
            let number = u64::try_from(page.ordinal)
                .ok()
                .and_then(|n| b.start.checked_add(n))
                .ok_or(Error::InvalidOp(INVALID))?;
            let text = mark_layout::bates(&b.prefix, number, b.digits, &b.suffix);
            let face = Face {
                fonts: &b.fonts,
                size: b.size,
                color: b.color,
            };
            mark_layout::slot_child(&text, b.slot, b.margins, sheet, face)
                .into_iter()
                .collect()
        }
    })
}

/// Everything a mark needs about one page, read before anything is written.
struct Target {
    page: laminapdf_cos::Page,
    sheet: Sheet,
    /// The page space to user space matrix.
    to_user: Matrix,
    index: usize,
    label: String,
    /// The page's added objects.
    placed: Vec<Placed>,
}

/// Whether a mark is new or replaces one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Add,
    Update,
}

/// What an op that puts a mark on pages needs.
struct Put<'a> {
    id: &'a ContentId,
    pages: &'a [PageId],
    mark: &'a PageMark,
    modified: &'a PdfDate,
    mode: Mode,
}

/// Distinct pages, at least one and not too many.
fn check_pages(pages: &[PageId]) -> Result<()> {
    let distinct: BTreeSet<&PageId> = pages.iter().collect();
    if pages.is_empty() || pages.len() > MAX_PAGES || distinct.len() != pages.len() {
        return Err(Error::InvalidOp(INVALID));
    }
    Ok(())
}

/// Reads each page's parts and its added objects.
fn targets(doc: &laminapdf_model::Document, pages: &[PageId]) -> Result<Vec<Target>> {
    let view = doc.pages()?;
    pages
        .iter()
        .map(|id| {
            let (page, frame) = page_parts(doc, id)?;
            let index = view.index_of(id).ok_or_else(|| Error::UnknownPage(id.clone()))?;
            let label = view
                .get(index)
                .map(|p| p.label())
                .transpose()?
                .flatten()
                .unwrap_or_else(|| (index + 1).to_string());
            Ok(Target {
                sheet: Sheet {
                    width: frame.width().get(),
                    height: frame.height().get(),
                },
                to_user: frame.to_user_space_matrix(),
                index,
                label,
                placed: scan(doc, &page)?,
                page,
            })
        })
        .collect()
}

/// A form over the page, in page space, that draws each of `forms` through its matrix. `group`,
/// when given, is the optional content group that switches the whole form off.
fn container(
    doc: &mut laminapdf_model::Document,
    sheet: Sheet,
    forms: &[(ObjRef, Matrix)],
    group: Option<ObjRef>,
) -> Result<ObjRef> {
    let mut content = String::new();
    let mut xobjects = Dict::new();
    for (i, (form, matrix)) in forms.iter().enumerate() {
        let name = format!("M{i}");
        let cm = matrix.to_array().map(format_real).join(" ");
        content.push_str(&format!("q {cm} cm /{name} Do Q\n"));
        xobjects.insert(name.as_str(), Object::Ref(*form));
    }
    let mut resources = Dict::new();
    resources.insert("XObject", xobjects);
    let bbox = [0.0, 0.0, sheet.width, sheet.height].map(Object::number);
    let mut dict = Dict::new();
    dict.insert("Type", Object::name("XObject"));
    dict.insert("Subtype", Object::name("Form"));
    dict.insert("BBox", Object::Array(bbox.into()));
    dict.insert("Resources", resources);
    if let Some(group) = group {
        dict.insert("OC", Object::Ref(group));
    }
    Ok(doc
        .cos_mut()
        .edit()
        .create(Object::Stream(Stream::flate(dict, content.as_bytes())))?)
}

/// Puts `put.mark` on `put.pages`, new or in place of the one with the same ID.
fn put(
    doc: &mut laminapdf_model::Document,
    appearances: Option<&dyn Appearances>,
    blobs: &BlobStore,
    put: &Put<'_>,
) -> Result<Effect> {
    check_pages(put.pages)?;
    if !put.mark.is_valid() {
        return Err(Error::InvalidOp(INVALID));
    }
    let generator = appearances.ok_or(Error::AppearanceUnavailable)?;
    let pages = doc.pages()?.len();
    let targets = targets(doc, put.pages)?;
    // Every page is checked before the first is written.
    let mut old_groups = BTreeSet::new();
    let mut reused = None;
    for target in &targets {
        let old = target.placed.iter().find(|p| &p.private.id == put.id);
        match (put.mode, old) {
            (Mode::Add, Some(_)) => return Err(Error::ContentIdInUse(put.id.clone())),
            (Mode::Add, None) => {}
            (Mode::Update, None) => return Err(Error::UnknownPageMark(put.id.clone())),
            (Mode::Update, Some(old)) => match &old.private.mark {
                None => return Err(Error::UnknownPageMark(put.id.clone())),
                Some(was) if std::mem::discriminant(was) != std::mem::discriminant(put.mark) => {
                    return Err(Error::InvalidOp("a page mark is replaced by one of its own kind"));
                }
                Some(_) => {
                    if let Some(group) = ocg::group_of(&old.form_dict) {
                        old_groups.insert(group);
                        reused.get_or_insert(group);
                    }
                }
            },
        }
    }
    let group = if put.mark.hideable() {
        Some(match reused {
            Some(group) => group,
            None => {
                let element = if put.mark.layer() == ContentLayer::Behind {
                    Element::Background
                } else {
                    Element::Foreground
                };
                ocg::add_group(doc, &put.mark.group_name(), element)?
            }
        })
    } else {
        None
    };
    let artifact = if ParentTree::read(doc)?.is_some() {
        Mark::Page(put.mark.artifact())
    } else {
        Mark::None
    };
    // The forms drawn so far, so the same text on many pages is drawn, and its font embedded,
    // once.
    let mut drawn: HashMap<String, ObjRef> = HashMap::new();
    for (ordinal, target) in targets.iter().enumerate() {
        let context = PageContext {
            sheet: target.sheet,
            index: target.index,
            pages,
            label: &target.label,
            ordinal,
        };
        let mut forms = Vec::new();
        for child in children(put.mark, &context, generator, blobs)? {
            if !child.spec.is_valid() {
                return Err(Error::InvalidOp(INVALID));
            }
            let key = serde_json::to_string(&child.spec).map_err(|_| Error::InvalidOp(INVALID))?;
            let form = match drawn.get(&key) {
                Some(form) => *form,
                None => {
                    let form = draw(doc, appearances, blobs, &child.spec)?;
                    drawn.insert(key, form);
                    form
                }
            };
            forms.push((form, child.matrix));
        }
        let form = container(doc, target.sheet, &forms, group)?;
        let private = Private {
            id: put.id.clone(),
            kind: put.mark.kind().to_owned(),
            spec: None,
            alt: None,
            layer: put.mark.layer(),
            mark: Some(stored(put.mark, ordinal)),
        };
        mark_form(doc, form, &private, put.modified)?;
        let placement = Placement {
            cm: target.to_user,
            opacity: put.mark.opacity(),
            mark: artifact,
            layer: put.mark.layer(),
        };
        match put.mode {
            Mode::Add => {
                let behind: Vec<ObjRef> = target
                    .placed
                    .iter()
                    .filter(|p| p.private.layer == ContentLayer::Behind)
                    .map(|p| p.stream)
                    .collect();
                place_form(doc, &target.page, form, placement, &behind)?;
            }
            Mode::Update => {
                let old = target.placed.iter().find(|p| &p.private.id == put.id);
                if let Some(old) = old {
                    replace(doc, &target.page, old, form, placement)?;
                }
            }
        }
    }
    // A group no form names any more (a mark made unhideable) leaves the document's layers.
    for group in old_groups {
        release(doc, group)?;
    }
    Ok(Effect {
        pages: put.pages.to_vec(),
        ..Effect::default()
    })
}

/// What a page keeps of the mark: a Bates number's `start` becomes the page's own number.
fn stored(mark: &PageMark, ordinal: usize) -> PageMark {
    match mark {
        PageMark::Bates(b) => {
            let mut own = b.clone();
            own.start = b.start.saturating_add(u64::try_from(ordinal).unwrap_or(0));
            PageMark::Bates(own)
        }
        other => other.clone(),
    }
}

/// Draws `form` where the drawing of the mark `old` was, in place of it, and takes the old form's
/// name and the old opacity's ExtGState out of the page's resources.
fn replace(
    doc: &mut laminapdf_model::Document,
    page: &laminapdf_cos::Page,
    old: &Placed,
    form: ObjRef,
    placement: Placement,
) -> Result<()> {
    let mut writer = PageContentWriter::new(doc.cos_mut().edit(), page)?;
    let gs = fade(&mut writer, placement.opacity)?;
    let new = drawing(&mut writer, form, placement.cm, gs, placement.mark)?;
    let index = writer
        .contents()
        .iter()
        .position(|o| o.as_ref() == Some(old.stream))
        .unwrap_or(old.index);
    writer.remove(old.stream)?;
    writer.insert(index, &new.to_bytes())?;
    let mut gone = BTreeSet::new();
    if new.form != old.drawing.form {
        gone.insert(ResourceName::new("XObject", old.drawing.form.clone()));
    }
    if let Some(gs) = old.drawing.gs.as_ref().filter(|gs| new.gs.as_ref() != Some(*gs)) {
        gone.insert(ResourceName::new("ExtGState", gs.clone()));
    }
    writer.remove_resources(&gone)?;
    Ok(())
}

/// Takes `group` out of the document's layers unless a page still draws a form that names it.
fn release(doc: &mut laminapdf_model::Document, group: ObjRef) -> Result<()> {
    for page in doc.pages()?.iter() {
        for placed in scan(doc, page.cos())? {
            if ocg::group_of(&placed.form_dict) == Some(group) {
                return Ok(());
            }
        }
    }
    ocg::remove_group(doc, group)
}

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
            #[serde(default, skip_serializing_if = "is_false")]
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
/// `start` (the batch runner's job, E14d). The number is an artifact in a tagged file (`/Subtype
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
    /// The number is padded with zeros to this many digits, up to 20.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
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
            digits: 6,
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

/// Takes a page mark off the pages named, with its optional content group once no page draws a
/// form that names it. The pages' earlier revisions still draw it after an incremental save. A
/// page that draws no mark with the ID is `unknown-page-mark`, and nothing is deleted.
///
/// On the wire: `{ "op": "deletePageMark", "args": { "id": "draft", "pages": ["p3.0"] } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DeletePageMark {
    /// The mark.
    pub id: ContentId,
    /// The pages to take it off, without repeats.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
}

impl DeletePageMark {
    /// The op that takes the mark `id` off `pages`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(id: ContentId, pages: Vec<PageId>) -> Self {
        Self { id, pages }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        check_pages(&self.pages)?;
        let targets = targets(doc, &self.pages)?;
        let mut found = Vec::new();
        for target in &targets {
            let old = target
                .placed
                .iter()
                .find(|p| p.private.id == self.id && p.private.mark.is_some())
                .ok_or_else(|| Error::UnknownPageMark(self.id.clone()))?;
            found.push(old);
        }
        let mut groups = BTreeSet::new();
        for (target, old) in targets.iter().zip(found) {
            groups.extend(ocg::group_of(&old.form_dict));
            let mut writer = PageContentWriter::new(doc.cos_mut().edit(), &target.page)?;
            writer.remove(old.stream)?;
            let mut gone: BTreeSet<ResourceName> = [ResourceName::new("XObject", old.drawing.form.clone())].into();
            gone.extend(old.drawing.gs.clone().map(|gs| ResourceName::new("ExtGState", gs)));
            writer.remove_resources(&gone)?;
        }
        for group in groups {
            release(doc, group)?;
        }
        Ok(Effect {
            pages: self.pages.clone(),
            removed: true,
            ..Effect::default()
        })
    }
}

/// A page mark on a page, as [`page_marks`] lists it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct PageMarkInfo {
    /// Its ID.
    pub id: ContentId,
    /// What it is, as the page keeps it: a header's slots are the text with its fields, a Bates
    /// number's `start` is this page's own number.
    pub mark: PageMark,
    /// Whether a reader's layers panel can switch it off.
    pub hideable: bool,
}

/// The page marks Lamina PDF added to `page`, in the order they paint: backgrounds first. A mark
/// whose form another app changed, or that an older version stored differently, is the page's own
/// content and isn't listed. Marks the user may update or delete as a set share their ID across
/// pages: the batch of E6g's Manage page marks is the IDs this lists on each page.
///
/// # Errors
/// `unknown-page`; reading errors.
///
/// Added in 0.7.0.
pub fn page_marks(doc: &laminapdf_model::Document, page: &PageId) -> Result<Vec<PageMarkInfo>> {
    let (cos_page, _) = page_parts(doc, page)?;
    Ok(scan(doc, &cos_page)?
        .into_iter()
        .filter_map(|p| {
            let mark = p.private.mark?;
            Some(PageMarkInfo {
                id: p.private.id,
                hideable: ocg::group_of(&p.form_dict).is_some(),
                mark,
            })
        })
        .collect())
}
