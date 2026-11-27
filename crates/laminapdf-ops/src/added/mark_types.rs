//! What a page mark is, on the wire and as a page keeps it (E6 plan §2.3): the slots of a header
//! and footer, a mark's margins and content, and the look of each kind ([`PageMark`]).

use serde::{Deserialize, Serialize};

use super::place::PageArtifact;
use super::spec::{ContentLayer, is_twelve, twelve};
use crate::annot::{AnnotColor, AnnotFont, Opacity};
use crate::blob::BlobId;

/// The most characters a slot, a date, a file name, a prefix or a suffix holds.
const MAX_TEXT: usize = 1_000;
/// The widest a Bates number is padded.
const MAX_DIGITS: u8 = 20;

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

    pub(super) fn is_default(&self) -> bool {
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
pub(super) fn is_false(v: &bool) -> bool {
    !*v
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")]
pub(super) fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes the field by reference")]
pub(super) const fn is_zero_u8(v: &u8) -> bool {
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
    pub(super) const fn kind(&self) -> &'static str {
        match self {
            Self::Watermark(_) => "Watermark",
            Self::Background(_) => "Background",
            Self::HeaderFooter(_) => "HeaderFooter",
            Self::Bates(_) => "Bates",
        }
    }

    pub(super) const fn layer(&self) -> ContentLayer {
        match self {
            Self::Background(_) => ContentLayer::Behind,
            _ => ContentLayer::Front,
        }
    }

    pub(super) fn opacity(&self) -> f64 {
        match self {
            Self::Watermark(w) | Self::Background(w) => w.opacity.get(),
            _ => 1.0,
        }
    }

    pub(super) fn hideable(&self) -> bool {
        matches!(self, Self::Watermark(w) | Self::Background(w) if w.hideable)
    }

    /// What a tagged file's artifact says the mark is.
    pub(super) fn artifact(&self) -> PageArtifact {
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
    pub(super) fn group_name(&self) -> String {
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
    pub(super) fn is_valid(&self) -> bool {
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
