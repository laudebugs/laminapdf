//! `FreeText` annotations (ISO 32000-2 §12.5.6.6): a text box, a callout with a leader line, or
//! typewriter text. The text is [`RichText`], paragraphs of plain, bold and italic spans (E4 plan
//! §11); the font is one of the standard 14 families or a font's bytes by [`BlobId`].
//!
//! The keys other readers draw from are written with every box: `/DA` and `/DS` (font, size,
//! colour, alignment), `/Q`, `/Contents` (the plain text), `/RC` when a span is bold or italic,
//! `/IT`, `/CL` and `/LE` for a callout, `/RD`, `/BS` and `/BE`. Lamina PDF's own key
//! `/LMNA_Font` names a font given by its bytes, and `/LMNA_Fallbacks` the fallback fonts, so the
//! session that added the bytes can draw the box again; Annex E's second-class names keep them
//! apart from the standard's keys.

use laminapdf_cos::{Dict, Object, PdfString};
use laminapdf_model::{AnnotId, PageId, UserSpacePoint, UserSpaceRect};
use laminapdf_syntax::text::{decode_text_string, encode_text_string};
use serde::{Deserialize, Serialize};

use super::drawing::{self, Border, LineEnding};
use super::freetext_keys::{self, Defaults};
use super::geom::{self, AnnotColor};
use crate::blob::BlobId;
use crate::wire;

/// Lamina PDF's key naming a box's font by its bytes' [`BlobId`] (ISO 32000-2 Annex E).
pub(crate) const FONT_KEY: &str = "LMNA_Font";
/// Lamina PDF's key listing a box's fallback fonts by their bytes' [`BlobId`]s, in order.
const FALLBACKS_KEY: &str = "LMNA_Fallbacks";
/// The most characters a box holds: far more than a page shows, few enough to lay out at once.
const MAX_CHARS: usize = 100_000;
/// The font sizes a box takes, in points.
const SIZES: std::ops::RangeInclusive<f64> = 1.0..=500.0;

/// A `FreeText` annotation: text drawn on the page in a box, with a leader line for a callout.
///
/// `rect` is the box the text is laid out in, as the page is shown upright when `rotation` is the
/// page's `/Rotate`; the annotation's `/Rect` grows to hold the border and the leader line. The
/// mark's colour ([`AnnotCommon::color`](crate::AnnotCommon::color)) is the text's, the border's
/// and the leader line's, written in `/DA`; `fill` is the box's background, written as `/C`. The
/// box's `/Contents` is its text, whatever the op's common entries say.
///
/// On the wire: `{ "type": "freeText", "rect": { … }, "text": [[{ "text": "Check " }, { "text":
/// "this", "bold": true }]], "font": "helvetica", "size": 12 }`.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct FreeText {
    /// The text box.
    #[serde(with = "wire::user_space_rect")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "{ x0: number; y0: number; x1: number; y1: number }")
    )]
    pub rect: UserSpaceRect,
    /// The text.
    pub text: RichText,
    /// The font: `helvetica`, `times`, `courier`, or a font's bytes by their ID; Helvetica when
    /// absent.
    #[serde(default)]
    pub font: AnnotFont,
    /// Fonts by their bytes' IDs for characters `font` lacks, tried in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string[]"))]
    pub fallbacks: Vec<BlobId>,
    /// The size in points, from 1 to 500; 12 when absent.
    #[serde(default = "twelve")]
    pub size: f64,
    /// How lines sit in the box (`/Q`).
    #[serde(default)]
    pub align: TextAlign,
    /// A text box, a callout or typewriter text (`/IT`).
    #[serde(default)]
    pub intent: FreeTextIntent,
    /// The background (`/C`); none when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub fill: Option<AnnotColor>,
    /// The outline; a solid 1-point line when absent. Typewriter text draws none.
    #[serde(default)]
    pub border: Border,
    /// The page's `/Rotate` the text is upright for: 0, 90, 180 or 270.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub rotation: u16,
}

const fn twelve() -> f64 {
    12.0
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_zero(v: &u16) -> bool {
    *v == 0
}

impl FreeText {
    /// A text box over `rect` holding `text`, in 12-point Helvetica, left-aligned, with a solid
    /// 1-point border and no fill.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn new(rect: UserSpaceRect, text: RichText) -> Self {
        Self {
            rect,
            text,
            font: AnnotFont::default(),
            fallbacks: Vec::new(),
            size: 12.0,
            align: TextAlign::Left,
            intent: FreeTextIntent::TextBox,
            fill: None,
            border: Border::default(),
            rotation: 0,
        }
    }

    /// The same box in `font` at `size` points.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_font(mut self, font: AnnotFont, size: f64) -> Self {
        self.font = font;
        self.size = size;
        self
    }

    /// The same box with `fallbacks` for characters its font lacks.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_fallbacks(mut self, fallbacks: Vec<BlobId>) -> Self {
        self.fallbacks = fallbacks;
        self
    }

    /// The same box with its lines aligned by `align`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// The same box as `intent`: a text box, a callout or typewriter text.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_intent(mut self, intent: FreeTextIntent) -> Self {
        self.intent = intent;
        self
    }

    /// The same box filled with `fill`, or with no fill.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_fill(mut self, fill: Option<AnnotColor>) -> Self {
        self.fill = fill;
        self
    }

    /// The same box with `border`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn with_border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    /// The same box with its text upright on a page whose `/Rotate` is `rotation`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_rotation(mut self, rotation: u16) -> Self {
        self.rotation = rotation;
        self
    }

    /// The border as drawn: the box's, or none for typewriter text.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn drawn_border(&self) -> Border {
        match self.intent {
            FreeTextIntent::Typewriter => Border::solid(0.0),
            _ => self.border.clone(),
        }
    }

    /// Whether every value is in range: a box with area, a size from 1 to 500, a right angle,
    /// a callout of two or three points, at most 100,000 characters, and at most 100,000
    /// paragraphs and spans together.
    pub(crate) fn is_valid(&self) -> bool {
        let r = self.rect;
        let callout = match &self.intent {
            FreeTextIntent::Callout(c) => (2..=3).contains(&c.line.len()) && geom::finite(c.line.iter().copied()),
            FreeTextIntent::TextBox | FreeTextIntent::Typewriter => true,
        };
        geom::finite(geom::corners(&r))
            && r.x1 > r.x0
            && r.y1 > r.y0
            && SIZES.contains(&self.size)
            && matches!(self.rotation, 0 | 90 | 180 | 270)
            && callout
            && self.border.is_valid(true)
            && self.text.char_count() <= MAX_CHARS
            && self.text.paragraphs.len() + self.text.paragraphs.iter().map(Vec::len).sum::<usize>() <= MAX_CHARS
    }
}

/// Text in paragraphs of spans, each plain, bold or italic: the rich-text subset a box keeps
/// (E4 plan §11).
///
/// On the wire: an array of paragraphs, each an array of spans: `[[{ "text": "Q2 is " }, {
/// "text": "late", "bold": true }]]`.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct RichText {
    /// The paragraphs, each a list of spans.
    pub paragraphs: Vec<Vec<TextSpan>>,
}

impl RichText {
    /// Plain text, a paragraph per line (`\n`, `\r\n` or `\r` between them).
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn plain(text: &str) -> Self {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        Self {
            paragraphs: text.split('\n').map(|p| vec![TextSpan::plain(p)]).collect(),
        }
    }

    /// The text without styles, paragraphs joined by `\r` as `/Contents` holds them.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn to_plain(&self) -> String {
        self.paragraphs
            .iter()
            .map(|p| p.iter().map(|s| s.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\r")
    }

    /// Reads rich text as `/RC` holds it (XHTML), tolerating what other apps write, and says
    /// whether it held styles this subset doesn't keep: a size, colour or font for part of the
    /// text, or elements other than `body`, `p`, `span`, `b`, `i` and `br`. It never fails: text
    /// it can't parse is read as text.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn from_xhtml(xhtml: &str) -> (Self, bool) {
        let read = freetext_keys::read_rc(xhtml);
        (read.text, read.simplified)
    }

    /// The text as XHTML in the subset: a `p` per paragraph and a `span` per bold or italic span,
    /// which [`RichText::from_xhtml`] reads back unchanged.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn to_xhtml(&self) -> String {
        freetext_keys::write_body(self, None)
    }

    /// Whether a span is bold or italic, which only `/RC` keeps.
    pub(crate) fn is_styled(&self) -> bool {
        self.paragraphs.iter().flatten().any(|s| s.bold || s.italic)
    }

    fn char_count(&self) -> usize {
        self.paragraphs.iter().flatten().map(|s| s.text.chars().count()).sum()
    }
}

/// A run of text in one style.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct TextSpan {
    /// The text.
    pub text: String,
    /// Bold.
    #[serde(default, skip_serializing_if = "is_false")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub bold: bool,
    /// Italic.
    #[serde(default, skip_serializing_if = "is_false")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub italic: bool,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_false(v: &bool) -> bool {
    !*v
}

impl TextSpan {
    /// `text`, neither bold nor italic.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
        }
    }

    /// The same span in bold, or not.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// The same span in italic, or not.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }
}

/// A box's font: a standard 14 family, which every reader has and the extract and base
/// WebAssembly modules can lay out, or a font's bytes by their [`BlobId`], which needs the text
/// module and is embedded as a subset.
///
/// On the wire: `"helvetica"`, `"times"`, `"courier"` or the bytes' 64-digit hex ID.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(type = "string"))]
#[non_exhaustive]
pub enum AnnotFont {
    /// A standard 14 family, its bold and italic faces for bold and italic spans.
    Standard(StandardFamily),
    /// A font's bytes, added to the session with
    /// [`Document::add_blob`](crate::Document::add_blob). Bold and italic spans are drawn bolder
    /// (filled and outlined) and slanted.
    Font(BlobId),
}

impl Default for AnnotFont {
    fn default() -> Self {
        Self::Standard(StandardFamily::Helvetica)
    }
}

impl TryFrom<String> for AnnotFont {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match StandardFamily::from_name(&s) {
            Some(family) => Ok(Self::Standard(family)),
            None => s
                .parse()
                .map(Self::Font)
                .map_err(|_| "a font is helvetica, times, courier or a blob id".to_owned()),
        }
    }
}

impl From<AnnotFont> for String {
    fn from(font: AnnotFont) -> Self {
        match font {
            AnnotFont::Standard(family) => family.name().to_owned(),
            AnnotFont::Font(id) => id.to_string(),
        }
    }
}

/// The standard 14 families a box can use; Symbol and `ZapfDingbats` hold no text.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum StandardFamily {
    /// Helvetica: a sans-serif.
    Helvetica,
    /// Times: a serif.
    Times,
    /// Courier: fixed width.
    Courier,
}

impl StandardFamily {
    /// Its name on the wire.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Helvetica => "helvetica",
            Self::Times => "times",
            Self::Courier => "courier",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        [Self::Helvetica, Self::Times, Self::Courier]
            .into_iter()
            .find(|f| f.name() == name)
    }

    /// The PostScript name of its face: regular, bold, italic or both (`Times-BoldItalic`).
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn face(self, bold: bool, italic: bool) -> &'static str {
        match (self, bold, italic) {
            (Self::Helvetica, false, false) => "Helvetica",
            (Self::Helvetica, true, false) => "Helvetica-Bold",
            (Self::Helvetica, false, true) => "Helvetica-Oblique",
            (Self::Helvetica, true, true) => "Helvetica-BoldOblique",
            (Self::Times, false, false) => "Times-Roman",
            (Self::Times, true, false) => "Times-Bold",
            (Self::Times, false, true) => "Times-Italic",
            (Self::Times, true, true) => "Times-BoldItalic",
            (Self::Courier, false, false) => "Courier",
            (Self::Courier, true, false) => "Courier-Bold",
            (Self::Courier, false, true) => "Courier-Oblique",
            (Self::Courier, true, true) => "Courier-BoldOblique",
        }
    }
}

/// How a box's lines sit in it (`/Q`).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum TextAlign {
    /// Left (`/Q 0`).
    #[default]
    Left,
    /// Centred (`/Q 1`).
    Center,
    /// Right (`/Q 2`).
    Right,
}

impl TextAlign {
    pub(crate) const fn q(self) -> i64 {
        match self {
            Self::Left => 0,
            Self::Center => 1,
            Self::Right => 2,
        }
    }

    pub(crate) const fn from_q(q: i64) -> Self {
        match q {
            1 => Self::Center,
            2 => Self::Right,
            _ => Self::Left,
        }
    }

    pub(crate) const fn css(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }
}

/// What a box is (`/IT`).
///
/// On the wire: `{ "type": "textBox" }`, `{ "type": "typewriter" }` or `{ "type": "callout",
/// "line": [ … ], "end": "openArrow" }`.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum FreeTextIntent {
    /// A text box.
    #[default]
    TextBox,
    /// A callout: the box with a leader line to what it points at (`/IT /FreeTextCallout`).
    Callout(Callout),
    /// Typewriter text: no border and no fill, for filling in a flat form
    /// (`/IT /FreeTextTypeWriter`).
    Typewriter,
}

/// A callout's leader line (`/CL`, `/LE`).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Callout {
    /// From the point it points at, through the knee when there are three, to the box's edge.
    #[serde(with = "wire::points")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ x: number; y: number }[]"))]
    pub line: Vec<UserSpacePoint>,
    /// The ending at the point it points at; an open arrow when absent.
    #[serde(default = "open_arrow")]
    pub end: LineEnding,
}

const fn open_arrow() -> LineEnding {
    LineEnding::OpenArrow
}

impl Callout {
    /// A leader line along `line` (two or three points, the first where it points) ending in an
    /// open arrow.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(line: Vec<UserSpacePoint>) -> Self {
        Self {
            line,
            end: LineEnding::OpenArrow,
        }
    }

    /// The same line with `end` at the point it points at.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_end(mut self, end: LineEnding) -> Self {
        self.end = end;
        self
    }
}

/// The keys of `text` on `dict`, which already holds `/Rect`; `color` is the mark's.
pub(crate) fn write(dict: &mut Dict, text: &FreeText, color: Option<AnnotColor>) {
    freetext_keys::write_da(dict, text, color);
    dict.insert("Q", Object::Integer(text.align.q()));
    dict.insert(
        "Contents",
        Object::String(PdfString::literal(encode_text_string(&text.text.to_plain()))),
    );
    if text.text.is_styled() {
        let rc = freetext_keys::write_rc(text, color);
        dict.insert("RC", Object::String(PdfString::literal(encode_text_string(&rc))));
    } else {
        dict.remove("RC");
    }
    match text.fill {
        Some(fill) if !matches!(text.intent, FreeTextIntent::Typewriter) => {
            dict.insert("C", fill.to_object());
        }
        _ => {
            dict.remove("C");
        }
    }
    match &text.intent {
        FreeTextIntent::TextBox => {
            for key in ["IT", "CL", "LE"] {
                dict.remove(key);
            }
        }
        FreeTextIntent::Typewriter => {
            dict.insert("IT", Object::name("FreeTextTypeWriter"));
            dict.remove("CL");
            dict.remove("LE");
        }
        FreeTextIntent::Callout(callout) => {
            dict.insert("IT", Object::name("FreeTextCallout"));
            dict.insert("CL", drawing::numbers(&callout.line));
            dict.insert("LE", Object::name(callout.end.name()));
        }
    }
    drawing::write_border(dict, &text.drawn_border());
    write_rd(dict, text.rect);
    match text.font {
        AnnotFont::Font(id) => {
            dict.insert(
                FONT_KEY,
                Object::String(PdfString::literal(id.to_string().into_bytes())),
            );
        }
        AnnotFont::Standard(_) => {
            dict.remove(FONT_KEY);
        }
    }
    if text.fallbacks.is_empty() {
        dict.remove(FALLBACKS_KEY);
    } else {
        let ids = text
            .fallbacks
            .iter()
            .map(|id| Object::String(PdfString::literal(id.to_string().into_bytes())));
        dict.insert(FALLBACKS_KEY, Object::Array(ids.collect()));
    }
    if text.rotation == 0 {
        dict.remove("Rotate");
    } else {
        dict.insert("Rotate", Object::Integer(i64::from(text.rotation)));
    }
}

/// `/RD`: how far the box sits inside `/Rect` on each side.
fn write_rd(dict: &mut Dict, inner: UserSpaceRect) {
    let Some(outer) = dict.get("Rect").and_then(geom::rect_of) else {
        return;
    };
    let rd = [
        inner.x0 - outer.x0,
        inner.y0 - outer.y0,
        outer.x1 - inner.x1,
        outer.y1 - inner.y1,
    ];
    dict.insert(
        "RD",
        Object::Array(rd.iter().map(|v| Object::number(v.max(0.0))).collect()),
    );
}

/// What a `FreeText` annotation's keys describe, whichever app made it, and whether reading it
/// dropped something the subset can't keep (styles outside it, a font Lamina PDF can't name).
pub(crate) struct ReadBack {
    pub(crate) text: FreeText,
    /// The mark's colour, from `/DA`.
    pub(crate) color: Option<AnnotColor>,
    pub(crate) simplified: bool,
}

/// Reads a `FreeText` annotation's keys: the text from `/RC`, else `/Contents`; font, size, colour
/// and alignment from `/DA`, `/DS` and `/Q`, then `/RC`'s body; the box from `/Rect` less `/RD`.
pub(crate) fn read(dict: &Dict) -> Option<ReadBack> {
    let outer = dict.get("Rect").and_then(geom::rect_of)?;
    let rect = inner_rect(dict, outer);
    let da = dict
        .get("DA")
        .and_then(text_of)
        .map(|s| freetext_keys::parse_da(&s))
        .unwrap_or_default();
    let ds = dict
        .get("DS")
        .and_then(text_of)
        .map(|s| freetext_keys::parse_css(&s))
        .unwrap_or_default();
    let (rich, body, mut simplified) = match dict.get("RC").and_then(text_of) {
        Some(rc) => {
            let read = freetext_keys::read_rc(&rc);
            (Some(read.text), read.body, read.simplified)
        }
        None => (None, Defaults::default(), false),
    };
    let text = rich.unwrap_or_else(|| RichText::plain(&dict.get("Contents").and_then(text_of).unwrap_or_default()));
    // `/RC` is what readers show, then `/DS`, the default style, then `/DA`, which some apps
    // fill with the font of a form field's resources rather than the box's.
    let named = body.family.or(ds.family).or(da.family);
    let font = if let Some(id) = dict.get(FONT_KEY).and_then(text_of).and_then(|s| s.parse().ok()) {
        AnnotFont::Font(id)
    } else {
        simplified |= da.unknown_font;
        AnnotFont::Standard(named.unwrap_or(StandardFamily::Helvetica))
    };
    let size = body.size.or(ds.size).or(da.size).filter(|s| SIZES.contains(s));
    simplified |= size.is_none() && (body.size.is_some() || ds.size.is_some() || da.size.is_some());
    let align = match dict.get("Q").and_then(Object::as_int) {
        Some(q) => TextAlign::from_q(q),
        None => ds.align.or(body.align).unwrap_or_default(),
    };
    // Acrobat writes `FreeTextTypewriter`, the standard `FreeTextTypeWriter`.
    let it = dict.get_name("IT").map(<[u8]>::to_ascii_lowercase);
    let intent = match it.as_deref() {
        Some(b"freetexttypewriter") => FreeTextIntent::Typewriter,
        Some(b"freetextcallout") | None => match dict.get("CL").and_then(drawing::points_of) {
            Some(line) if (2..=3).contains(&line.len()) => FreeTextIntent::Callout(Callout {
                line,
                end: match dict.get("LE") {
                    Some(Object::Name(n)) => LineEnding::from_name(n.as_bytes()),
                    _ => LineEnding::None,
                },
            }),
            _ => FreeTextIntent::TextBox,
        },
        Some(_) => FreeTextIntent::TextBox,
    };
    let fill = dict
        .get("C")
        .and_then(geom::numbers_of)
        .and_then(|c| AnnotColor::from_components(&c));
    let rotation = dict
        .get("Rotate")
        .and_then(Object::as_int)
        .map_or(0, |r| r.rem_euclid(360));
    let text = FreeText {
        rect,
        text,
        font,
        fallbacks: match dict.get(FALLBACKS_KEY) {
            Some(Object::Array(ids)) => ids.iter().filter_map(text_of).filter_map(|s| s.parse().ok()).collect(),
            _ => Vec::new(),
        },
        size: size.unwrap_or(12.0),
        align,
        fill: (!matches!(intent, FreeTextIntent::Typewriter))
            .then_some(fill)
            .flatten(),
        border: drawing::read_border(dict, true),
        intent,
        rotation: u16::try_from(rotation - rotation % 90).unwrap_or(0),
    };
    Some(ReadBack {
        text,
        color: body.color.or(ds.color).or(da.color),
        simplified,
    })
}

/// A text box as [`read_text_box`] returns it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextBoxRead {
    /// The page it is on.
    pub page: PageId,
    /// The box as a spec, in user space: what an [`AddAnnot`](super::AddAnnot) would write.
    pub text_box: FreeText,
    /// The text's colour, from `/RC`, `/DS` or `/DA`; `None` means black.
    pub color: Option<AnnotColor>,
    /// Whether reading dropped something the spec can't hold: styles of its rich text outside
    /// bold and italic, a font Lamina PDF can't name, a size out of range. Changing the box
    /// writes it again without them (`rich-text-simplified`).
    pub simplified: bool,
}

/// The `FreeText` annotation `id` read back as the spec that would write it (ISO 32000-2
/// §12.5.6.6), as [`UpdateAnnot`](super::UpdateAnnot) reads it before changing it: text with
/// bold and italic from `/RC`, else `/Contents`; font, size, colour and alignment from `/RC`'s
/// body, `/DS` and `/DA`; the box from `/Rect` less `/RD`; its kind from `/IT` and `/CL`.
/// `None` for an annotation of another subtype, or one with no `/Rect`.
///
/// # Errors
/// `unknown-annotation` for an ID no page lists; `annotation-not-editable` for a widget or a
/// direct dictionary; reading errors.
///
/// Added in 0.7.0.
pub fn read_text_box(doc: &laminapdf_model::Document, id: &AnnotId) -> crate::Result<Option<TextBoxRead>> {
    let all = super::every_annotation(doc)?;
    let found = super::Found::locate(&all, id)?;
    let dict = found.dict(doc)?;
    if dict.get_name("Subtype") != Some(b"FreeText".as_slice()) {
        return Ok(None);
    }
    let (inline, unreadable) = super::redraw::text_keys_inline(doc, &dict)?;
    Ok(read(&inline).map(|read| TextBoxRead {
        page: found.page,
        text_box: read.text,
        color: read.color,
        simplified: read.simplified || unreadable,
    }))
}

/// `/Rect` less `/RD`, or `/Rect` when `/RD` is missing or leaves no box.
fn inner_rect(dict: &Dict, outer: UserSpaceRect) -> UserSpaceRect {
    let rd = dict.get("RD").and_then(geom::numbers_of).unwrap_or_default();
    match rd.as_slice() {
        [l, b, r, t] if [l, b, r, t].iter().all(|v| v.is_finite() && **v >= 0.0) => {
            let inner = UserSpaceRect::from_corners(outer.x0 + l, outer.y0 + b, outer.x1 - r, outer.y1 - t);
            if inner.x1 > inner.x0 && inner.y1 > inner.y0 {
                inner
            } else {
                outer
            }
        }
        _ => outer,
    }
}

/// A text string's text: UTF-16 with its byte order mark, UTF-8 with its mark, else
/// `PDFDocEncoding`.
fn text_of(value: &Object) -> Option<String> {
    match value {
        Object::String(s) => Some(decode_text_string(&s.bytes)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "freetext_tests.rs"]
mod tests;
