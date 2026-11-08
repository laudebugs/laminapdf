//! The records of the builder API: page-space geometry ([ADR 0027]), colours, path
//! segments, stroke styles, link targets and the document features, as JSON records
//! (ADR 0028 F5, F6: every option optional, unknown keys refused), converted to
//! `laminapdf-build`'s types.
//!
//! [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md

use laminapdf::{
    AfRelationship, Artifact, ArtifactKind, Attachment, BlendMode, BuildOptions, Color, Dash, FillRule, Fit, LineCap,
    LineJoin, LinkTarget, Metadata, OutlineEntry, PageLabel, PageLayout, PageMode, PathBuilder, PdfVersion, PointPt,
    Profile, Pt, RectPt, StrokeStyle, TableScope, Tag, TagKind, ViewerPreferences,
};
use serde::Deserialize;
use tsify::Tsify;

use crate::error::{JsResult, invalid_argument};

/// A page-space rectangle from its top-left corner and size.
pub(crate) fn rect(x: f64, y: f64, width: f64, height: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(width), Pt::new(height))
}

fn at(x: f64, y: f64) -> PointPt {
    PointPt::from_points(x, y)
}

/// A device colour from its components: 1 is gray, 3 RGB, 4 CMYK, each 0 to 1.
///
/// # Errors
/// `invalid-argument` for another number of components.
pub(crate) fn color(components: &[f64]) -> JsResult<Color> {
    match *components {
        [g] => Ok(Color::gray(g)),
        [r, g, b] => Ok(Color::rgb(r, g, b)),
        [c, m, y, k] => Ok(Color::cmyk(c, m, y, k)),
        _ => Err(invalid_argument(
            "components",
            "a colour has 1 (gray), 3 (RGB) or 4 (CMYK) components".to_owned(),
            Vec::new(),
        )),
    }
}

/// A colour for text options (the text module's records).
#[cfg(feature = "text")]
#[derive(Debug, Clone, Deserialize, Tsify)]
#[serde(untagged)]
pub(crate) enum ColorInit {
    /// 1 (gray), 3 (RGB) or 4 (CMYK) components, 0 to 1.
    Components(Vec<f64>),
    /// `#rrggbb` or `#rgb`.
    Hex(String),
}

#[cfg(feature = "text")]
impl ColorInit {
    pub(crate) fn to_color(&self) -> JsResult<Color> {
        match self {
            Self::Components(c) => color(c),
            Self::Hex(hex) => Color::hex(hex).map_err(|e| crate::error::lamina_error(&e.into())),
        }
    }
}

/// The path verbs of the flat path encoding (ADR 0016 §7), each followed by its numbers.
pub(crate) const PATH_VERBS: &str = "0 x y: moveTo; 1 x y: lineTo; 2 x1 y1 x2 y2 x y: cubicTo; \
3 x1 y1 x y: quadTo; 4: close; 5 x y w h: rect; 6 x y w h: ellipse; 7 cx cy r: circle";

/// One decoded path segment.
#[derive(Debug, Clone, Copy)]
enum Segment {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    CubicTo([f64; 6]),
    QuadTo([f64; 4]),
    Close,
    Rect([f64; 4]),
    Ellipse([f64; 4]),
    Circle([f64; 3]),
}

/// A path in the flat encoding: verbs and their numbers in one `Float64Array`, so drawing
/// needs no record conversion ([`PATH_VERBS`]).
#[derive(Debug, Clone)]
pub(crate) struct Path(Vec<Segment>);

impl Path {
    /// Decodes `data`.
    ///
    /// # Errors
    /// `invalid-argument` for an unknown verb or a path cut short.
    pub(crate) fn decode(data: &[f64]) -> JsResult<Self> {
        let bad = || invalid_argument("path", format!("not a path: {PATH_VERBS}"), Vec::new());
        let mut out = Vec::new();
        let mut rest = data;
        while let Some((&verb, tail)) = rest.split_first() {
            let take = |n: usize| tail.get(..n).ok_or_else(bad);
            // The verb's number, compared bit for bit (no float comparison).
            let verb = (0..8u8)
                .find(|v| f64::from(*v).to_bits() == verb.to_bits())
                .ok_or_else(bad)?;
            let (segment, used) = match verb {
                0 => {
                    let [x, y] = array(take(2)?);
                    (Segment::MoveTo(x, y), 2)
                }
                1 => {
                    let [x, y] = array(take(2)?);
                    (Segment::LineTo(x, y), 2)
                }
                2 => (Segment::CubicTo(array(take(6)?)), 6),
                3 => (Segment::QuadTo(array(take(4)?)), 4),
                4 => (Segment::Close, 0),
                5 => (Segment::Rect(array(take(4)?)), 4),
                6 => (Segment::Ellipse(array(take(4)?)), 4),
                _ => (Segment::Circle(array(take(3)?)), 3),
            };
            out.push(segment);
            rest = tail.get(used..).unwrap_or_default();
        }
        Ok(Self(out))
    }

    /// Replays the path into `p`.
    pub(crate) fn build(&self, p: &mut PathBuilder) {
        for s in &self.0 {
            match *s {
                Segment::MoveTo(x, y) => p.move_to(at(x, y)),
                Segment::LineTo(x, y) => p.line_to(at(x, y)),
                Segment::CubicTo([x1, y1, x2, y2, x, y]) => p.cubic_to(at(x1, y1), at(x2, y2), at(x, y)),
                Segment::QuadTo([x1, y1, x, y]) => p.quad_to(at(x1, y1), at(x, y)),
                Segment::Close => p.close(),
                Segment::Rect([x, y, width, height]) => p.rect(rect(x, y, width, height)),
                Segment::Ellipse([x, y, width, height]) => p.ellipse(rect(x, y, width, height)),
                Segment::Circle([x, y, radius]) => p.circle(at(x, y), Pt::new(radius)),
            };
        }
    }
}

/// The first `N` numbers of `s` (which has at least `N`).
fn array<const N: usize>(s: &[f64]) -> [f64; N] {
    let mut out = [0.0; N];
    for (o, v) in out.iter_mut().zip(s) {
        *o = *v;
    }
    out
}

/// The value named `name` among `known`.
///
/// # Errors
/// `invalid-argument`, with the closest names in `didYouMean`.
pub(crate) fn pick<T: Copy>(argument: &'static str, name: &str, known: &[(&str, T)]) -> JsResult<T> {
    known.iter().find(|(n, _)| *n == name).map(|(_, v)| *v).ok_or_else(|| {
        let names: Vec<String> = known.iter().map(|(n, _)| (*n).to_owned()).collect();
        invalid_argument(
            argument,
            format!("expected one of {}", names.join(", ")),
            laminapdf::did_you_mean(name, &names),
        )
    })
}

/// A structure element from `@laminapdf/sdk`: `kind` is a standard structure type's name
/// ([`TagKind::as_str`]; a custom type's role when `custom` names it), `scope` a
/// [`TABLE_SCOPES`] name.
///
/// # Errors
/// `invalid-argument` for an unknown `kind` or `scope`.
pub(crate) fn tag(
    kind: &str,
    custom: Option<String>,
    attributes: [Option<String>; 4],
    scope: Option<String>,
) -> JsResult<Tag> {
    let kind = pick("kind", kind, &TagKind::ALL.map(|k| (k.as_str(), k)))?;
    let mut tag = match custom {
        Some(name) => Tag::custom(name, kind),
        None => Tag::new(kind),
    };
    let [alt, actual_text, lang, expansion] = attributes;
    if let Some(alt) = alt {
        tag = tag.with_alt(alt);
    }
    if let Some(text) = actual_text {
        tag = tag.with_actual_text(text);
    }
    if let Some(lang) = lang {
        tag = tag.with_lang(lang);
    }
    if let Some(expansion) = expansion {
        tag = tag.with_expansion(expansion);
    }
    match scope {
        Some(scope) => Ok(tag.with_scope(pick("scope", &scope, &TABLE_SCOPES)?)),
        None => Ok(tag),
    }
}

// The name tables below are the one list of these names: `cargo xtask gen boundary` reads them
// (and `TagKind::as_str`) from the source and writes @laminapdf/sdk's
// src/generated/builder-enums.ts, so keep each a `[("name", Type::Variant), …]` literal.

/// Artifacts by name: each kind, then the pagination artifacts that say what they are (a header
/// attached to the top edge, a footer to the bottom, a watermark to none).
pub(crate) const ARTIFACT_KINDS: [(&str, Artifact); 7] = [
    ("layout", Artifact::new(ArtifactKind::Layout)),
    ("page", Artifact::new(ArtifactKind::Page)),
    ("pagination", Artifact::new(ArtifactKind::Pagination)),
    ("background", Artifact::new(ArtifactKind::Background)),
    ("header", Artifact::header()),
    ("footer", Artifact::footer()),
    ("watermark", Artifact::watermark()),
];

/// Table scopes (a `TH`'s `/Scope`) by name.
pub(crate) const TABLE_SCOPES: [(&str, TableScope); 3] = [
    ("row", TableScope::Row),
    ("column", TableScope::Column),
    ("both", TableScope::Both),
];

/// Fill rules by name.
pub(crate) const FILL_RULES: [(&str, FillRule); 2] = [("nonZero", FillRule::NonZero), ("evenOdd", FillRule::EvenOdd)];

/// Line caps by name.
pub(crate) const LINE_CAPS: [(&str, LineCap); 3] = [
    ("butt", LineCap::Butt),
    ("round", LineCap::Round),
    ("square", LineCap::Square),
];

/// Line joins by name.
pub(crate) const LINE_JOINS: [(&str, LineJoin); 3] = [
    ("miter", LineJoin::Miter),
    ("round", LineJoin::Round),
    ("bevel", LineJoin::Bevel),
];

/// Blend modes by name (ISO 32000-2 §11.3.5), camelCase.
pub(crate) const BLEND_MODES: [(&str, BlendMode); 16] = [
    ("normal", BlendMode::Normal),
    ("multiply", BlendMode::Multiply),
    ("screen", BlendMode::Screen),
    ("overlay", BlendMode::Overlay),
    ("darken", BlendMode::Darken),
    ("lighten", BlendMode::Lighten),
    ("colorDodge", BlendMode::ColorDodge),
    ("colorBurn", BlendMode::ColorBurn),
    ("hardLight", BlendMode::HardLight),
    ("softLight", BlendMode::SoftLight),
    ("difference", BlendMode::Difference),
    ("exclusion", BlendMode::Exclusion),
    ("hue", BlendMode::Hue),
    ("saturation", BlendMode::Saturation),
    ("color", BlendMode::Color),
    ("luminosity", BlendMode::Luminosity),
];

/// A stroke style from its parts.
pub(crate) fn stroke_style(
    width: f64,
    cap: LineCap,
    join: LineJoin,
    miter_limit: f64,
    dash: &[f64],
    phase: f64,
) -> StrokeStyle {
    StrokeStyle {
        width: Pt::new(width),
        cap,
        join,
        miter_limit,
        dash: (!dash.is_empty()).then(|| Dash::new(dash.to_vec(), phase)),
    }
}

/// How a destination shows its page (§12.3.2.2).
#[derive(Debug, Clone, Deserialize, Tsify)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum FitInit {
    /// The whole page.
    Page,
    /// The page's width, at `top`.
    Width {
        /// The top edge shown.
        #[tsify(optional)]
        top: Option<f64>,
    },
    /// The page's height, at `left`.
    Height {
        /// The left edge shown.
        #[tsify(optional)]
        left: Option<f64>,
    },
    /// A point and a zoom.
    Xyz {
        /// Left.
        #[tsify(optional)]
        left: Option<f64>,
        /// Top.
        #[tsify(optional)]
        top: Option<f64>,
        /// Zoom (1 = 100 %).
        #[tsify(optional)]
        zoom: Option<f64>,
    },
    /// A rectangle.
    Rect {
        /// Left.
        x: f64,
        /// Top.
        y: f64,
        /// Width.
        width: f64,
        /// Height.
        height: f64,
    },
    /// The page's content bounds.
    Bounds,
}

impl FitInit {
    pub(crate) fn to_fit(&self) -> Fit {
        match *self {
            Self::Page => Fit::Page,
            Self::Width { top } => Fit::Width { top: top.map(Pt::new) },
            Self::Height { left } => Fit::Height {
                left: left.map(Pt::new),
            },
            Self::Xyz { left, top, zoom } => Fit::Xyz {
                left: left.map(Pt::new),
                top: top.map(Pt::new),
                zoom,
            },
            Self::Rect { x, y, width, height } => Fit::Rect(rect(x, y, width, height)),
            Self::Bounds => Fit::Bounds,
        }
    }
}

/// Where a link or outline item goes.
#[derive(Debug, Clone, Deserialize, Tsify)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum LinkTargetInit {
    /// A URI.
    Uri {
        /// The URI.
        uri: String,
    },
    /// A page of this document (0-based), shown as `fit` (default: the whole page).
    Page {
        /// The page index.
        index: u32,
        /// How it is shown.
        #[tsify(optional)]
        fit: Option<FitInit>,
    },
    /// A named destination ([`crate::WasmBuilder::destination`]).
    Named {
        /// The name.
        name: String,
    },
}

impl LinkTargetInit {
    pub(crate) fn to_target(&self) -> LinkTarget {
        match self {
            Self::Uri { uri } => LinkTarget::Uri(uri.clone()),
            Self::Page { index, fit } => LinkTarget::Page {
                index: *index,
                fit: fit.as_ref().map_or(Fit::Page, FitInit::to_fit),
            },
            Self::Named { name } => LinkTarget::Named(name.clone()),
        }
    }
}

/// The PDF version of a new document.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
pub(crate) enum PdfVersionInit {
    /// PDF 1.4.
    #[serde(rename = "1.4")]
    V1_4,
    /// PDF 1.5.
    #[serde(rename = "1.5")]
    V1_5,
    /// PDF 1.6.
    #[serde(rename = "1.6")]
    V1_6,
    /// PDF 1.7 (the default).
    #[serde(rename = "1.7")]
    V1_7,
    /// PDF 2.0.
    #[serde(rename = "2.0")]
    V2_0,
}

/// Options for a new document: everything that would come from the environment is given,
/// so the same calls give the same bytes.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BuildOptionsInit {
    /// The header version (default 1.7).
    #[tsify(optional)]
    pub version: Option<PdfVersionInit>,
    /// The first `/ID` entry: 16 bytes as 32 hex digits (default: a content hash).
    #[tsify(optional)]
    pub id: Option<String>,
    /// `/Producer`.
    #[tsify(optional)]
    pub producer: Option<String>,
    /// `/CreationDate`, a PDF date (`D:20261030120000Z`).
    #[tsify(optional)]
    pub creation_date: Option<String>,
    /// Flate-compress content streams (default `true`).
    #[tsify(optional)]
    pub compress: Option<bool>,
    /// A tagged document (structure tree, `/MarkInfo`) even before any element (default
    /// `false`; a page using `beginTag` or `beginArtifact` makes it tagged either way).
    #[tsify(optional)]
    pub tagged: Option<bool>,
    /// Conformance profiles: `PDF/UA-1` with at most one PDF/A profile. `finish` then fails
    /// with a coded error for each machine-checkable rule the document breaks.
    #[tsify(optional)]
    pub profiles: Option<Vec<ProfileInit>>,
}

/// A conformance profile.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
pub(crate) enum ProfileInit {
    /// PDF/UA-1 (ISO 14289-1).
    #[serde(rename = "PDF/UA-1")]
    PdfUa1,
    /// PDF/A-2b (ISO 19005-2).
    #[serde(rename = "PDF/A-2b")]
    PdfA2b,
    /// PDF/A-2u.
    #[serde(rename = "PDF/A-2u")]
    PdfA2u,
    /// PDF/A-3b (ISO 19005-3).
    #[serde(rename = "PDF/A-3b")]
    PdfA3b,
}

impl ProfileInit {
    const fn to_profile(self) -> Profile {
        match self {
            Self::PdfUa1 => Profile::PdfUa1,
            Self::PdfA2b => Profile::PdfA2b,
            Self::PdfA2u => Profile::PdfA2u,
            Self::PdfA3b => Profile::PdfA3b,
        }
    }
}

/// How an attached file relates to the document (`/AFRelationship`).
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AfRelationshipInit {
    /// The original the document was made from.
    Source,
    /// Data the content was derived from.
    Data,
    /// Another representation of the same content.
    Alternative,
    /// A supplement to the content.
    Supplement,
    /// None of these (the default).
    Unspecified,
}

/// An attached file's options.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AttachmentInit {
    /// How the file relates to the document (default `unspecified`).
    #[tsify(optional)]
    pub relationship: Option<AfRelationshipInit>,
    /// `/Desc`, shown next to the file.
    #[tsify(optional)]
    pub description: Option<String>,
    /// When the file last changed, a PDF date (default: the document's creation date).
    #[tsify(optional)]
    pub mod_date: Option<String>,
}

impl AttachmentInit {
    pub(crate) fn to_attachment(&self, name: &str, bytes: Vec<u8>, mime: &str) -> JsResult<Attachment> {
        let mut file = Attachment::new(name, bytes, mime);
        if let Some(r) = self.relationship {
            file = file.with_relationship(match r {
                AfRelationshipInit::Source => AfRelationship::Source,
                AfRelationshipInit::Data => AfRelationship::Data,
                AfRelationshipInit::Alternative => AfRelationship::Alternative,
                AfRelationshipInit::Supplement => AfRelationship::Supplement,
                AfRelationshipInit::Unspecified => AfRelationship::Unspecified,
            });
        }
        if let Some(d) = &self.description {
            file = file.with_description(d.clone());
        }
        if let Some(date) = &self.mod_date {
            file = file.with_mod_date(parse_date(date, "options.modDate")?);
        }
        Ok(file)
    }
}

impl BuildOptionsInit {
    pub(crate) fn to_options(&self) -> JsResult<BuildOptions> {
        let mut options = BuildOptions::default();
        if let Some(version) = self.version {
            options = options.with_version(match version {
                PdfVersionInit::V1_4 => PdfVersion::V1_4,
                PdfVersionInit::V1_5 => PdfVersion::V1_5,
                PdfVersionInit::V1_6 => PdfVersion::V1_6,
                PdfVersionInit::V1_7 => PdfVersion::V1_7,
                PdfVersionInit::V2_0 => PdfVersion::V2_0,
            });
        }
        if let Some(id) = &self.id {
            let id = parse_id(id)
                .ok_or_else(|| invalid_argument("options.id", "the ID must be 32 hex digits".to_owned(), Vec::new()))?;
            options = options.with_id(id);
        }
        if let Some(producer) = &self.producer {
            options = options.with_producer(producer.clone());
        }
        if let Some(date) = &self.creation_date {
            options = options.with_creation_date(parse_date(date, "options.creationDate")?);
        }
        if let Some(compress) = self.compress {
            options = options.with_compress(compress);
        }
        if let Some(tagged) = self.tagged {
            options = options.with_tagged(tagged);
        }
        for profile in self.profiles.iter().flatten() {
            options = options.with_profile(profile.to_profile());
        }
        Ok(options)
    }
}

/// 32 hex digits → 16 bytes; `None` for anything else.
fn parse_id(hex: &str) -> Option<[u8; 16]> {
    let digits = hex.as_bytes();
    // `from_str_radix` alone would also take a sign (`+f`).
    if digits.len() != 32 || !digits.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut id = [0u8; 16];
    for (byte, pair) in id.iter_mut().zip(digits.as_chunks::<2>().0) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(id)
}

fn parse_date(text: &str, argument: &'static str) -> JsResult<laminapdf::PdfDate> {
    text.parse().map_err(|_| {
        invalid_argument(
            argument,
            "not a PDF date such as D:20261030120000Z".to_owned(),
            Vec::new(),
        )
    })
}

/// The document metadata (Info and XMP from the same values).
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MetadataInit {
    /// Title.
    #[tsify(optional)]
    pub title: Option<String>,
    /// Author.
    #[tsify(optional)]
    pub author: Option<String>,
    /// Subject.
    #[tsify(optional)]
    pub subject: Option<String>,
    /// Keywords.
    #[tsify(optional)]
    pub keywords: Option<String>,
    /// Creator (the application that made the content).
    #[tsify(optional)]
    pub creator: Option<String>,
    /// Producer.
    #[tsify(optional)]
    pub producer: Option<String>,
    /// Creation date, a PDF date.
    #[tsify(optional)]
    pub creation_date: Option<String>,
    /// Modification date, a PDF date.
    #[tsify(optional)]
    pub mod_date: Option<String>,
}

impl MetadataInit {
    pub(crate) fn to_metadata(&self) -> JsResult<Metadata> {
        let mut m = Metadata::default();
        if let Some(v) = &self.title {
            m = m.with_title(v.clone());
        }
        if let Some(v) = &self.author {
            m = m.with_author(v.clone());
        }
        if let Some(v) = &self.subject {
            m = m.with_subject(v.clone());
        }
        if let Some(v) = &self.keywords {
            m = m.with_keywords(v.clone());
        }
        if let Some(v) = &self.creator {
            m = m.with_creator(v.clone());
        }
        if let Some(v) = &self.producer {
            m = m.with_producer(v.clone());
        }
        if let Some(v) = &self.creation_date {
            m = m.with_creation_date(parse_date(v, "metadata.creationDate")?);
        }
        if let Some(v) = &self.mod_date {
            m = m.with_mod_date(parse_date(v, "metadata.modDate")?);
        }
        Ok(m)
    }
}

/// One outline item and its children.
#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OutlineEntryInit {
    /// The title shown.
    pub title: String,
    /// Where it goes (none: a heading).
    #[tsify(optional)]
    pub target: Option<LinkTargetInit>,
    /// Its children.
    #[serde(default)]
    #[tsify(optional)]
    pub children: Vec<OutlineEntryInit>,
    /// Shown expanded.
    #[tsify(optional)]
    pub open: Option<bool>,
    /// The title's colour, RGB 0 to 1.
    #[tsify(optional)]
    pub color: Option<[f64; 3]>,
    /// Bold title.
    #[tsify(optional)]
    pub bold: Option<bool>,
    /// Italic title.
    #[tsify(optional)]
    pub italic: Option<bool>,
}

impl OutlineEntryInit {
    pub(crate) fn to_entry(&self) -> OutlineEntry {
        let mut e = match &self.target {
            Some(target) => OutlineEntry::new(self.title.clone(), target.to_target()),
            None => OutlineEntry::heading(self.title.clone()),
        };
        e = e.with_children(self.children.iter().map(Self::to_entry));
        if let Some(open) = self.open {
            e = e.open(open);
        }
        if let Some([r, g, b]) = self.color {
            e = e.with_color(r, g, b);
        }
        if self.bold == Some(true) {
            e = e.bold();
        }
        if self.italic == Some(true) {
            e = e.italic();
        }
        e
    }
}

/// A page-label numbering style.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum LabelStyleInit {
    /// 1, 2, 3.
    Decimal,
    /// I, II, III.
    UpperRoman,
    /// i, ii, iii.
    LowerRoman,
    /// A, B, C.
    UpperLetters,
    /// a, b, c.
    LowerLetters,
}

/// One page-label range, from page `start` (0-based) on.
#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PageLabelInit {
    /// The first page of the range.
    pub start: u32,
    /// The numbering style (none: prefix only).
    #[tsify(optional)]
    pub style: Option<LabelStyleInit>,
    /// A prefix.
    #[tsify(optional)]
    pub prefix: Option<String>,
    /// The first number (default 1).
    #[tsify(optional)]
    pub first: Option<u32>,
}

impl PageLabelInit {
    pub(crate) fn to_label(&self) -> PageLabel {
        let style = self.style.map(|s| match s {
            LabelStyleInit::Decimal => laminapdf::LabelStyle::Decimal,
            LabelStyleInit::UpperRoman => laminapdf::LabelStyle::UpperRoman,
            LabelStyleInit::LowerRoman => laminapdf::LabelStyle::LowerRoman,
            LabelStyleInit::UpperLetters => laminapdf::LabelStyle::UpperLetters,
            LabelStyleInit::LowerLetters => laminapdf::LabelStyle::LowerLetters,
        });
        let mut label = PageLabel::new(self.start, style);
        if let Some(prefix) = &self.prefix {
            label = label.with_prefix(prefix.clone());
        }
        if let Some(first) = self.first {
            label = label.with_first(first);
        }
        label
    }
}

/// The panel a viewer opens with.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PageModeInit {
    /// No panel.
    None,
    /// The outline.
    Outlines,
    /// Thumbnails.
    Thumbnails,
    /// Full screen.
    FullScreen,
    /// Optional content.
    OptionalContent,
    /// Attachments.
    Attachments,
}

/// How a viewer lays pages out.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PageLayoutInit {
    /// One page at a time.
    SinglePage,
    /// One continuous column.
    OneColumn,
    /// Two columns, odd pages left.
    TwoColumnLeft,
    /// Two columns, odd pages right.
    TwoColumnRight,
    /// Two pages at a time, odd pages left.
    TwoPageLeft,
    /// Two pages at a time, odd pages right.
    TwoPageRight,
}

/// How viewers present the document when it opens.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ViewerPreferencesInit {
    /// Show the title, not the file name.
    #[tsify(optional)]
    pub display_doc_title: Option<bool>,
    /// The panel shown.
    #[tsify(optional)]
    pub page_mode: Option<PageModeInit>,
    /// The page layout.
    #[tsify(optional)]
    pub page_layout: Option<PageLayoutInit>,
}

impl ViewerPreferencesInit {
    pub(crate) fn to_prefs(&self) -> ViewerPreferences {
        let mut prefs = ViewerPreferences::default();
        if let Some(on) = self.display_doc_title {
            prefs = prefs.with_display_doc_title(on);
        }
        if let Some(mode) = self.page_mode {
            prefs = prefs.with_page_mode(match mode {
                PageModeInit::None => PageMode::None,
                PageModeInit::Outlines => PageMode::Outlines,
                PageModeInit::Thumbnails => PageMode::Thumbnails,
                PageModeInit::FullScreen => PageMode::FullScreen,
                PageModeInit::OptionalContent => PageMode::OptionalContent,
                PageModeInit::Attachments => PageMode::Attachments,
            });
        }
        if let Some(layout) = self.page_layout {
            prefs = prefs.with_page_layout(match layout {
                PageLayoutInit::SinglePage => PageLayout::SinglePage,
                PageLayoutInit::OneColumn => PageLayout::OneColumn,
                PageLayoutInit::TwoColumnLeft => PageLayout::TwoColumnLeft,
                PageLayoutInit::TwoColumnRight => PageLayout::TwoColumnRight,
                PageLayoutInit::TwoPageLeft => PageLayout::TwoPageLeft,
                PageLayoutInit::TwoPageRight => PageLayout::TwoPageRight,
            });
        }
        prefs
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize as _;
    use serde::de::IntoDeserializer as _;

    use laminapdf::{Artifact, ArtifactKind, BlendMode, FillRule, LineCap, LineJoin, TableScope};

    use super::{
        ARTIFACT_KINDS, BLEND_MODES, FILL_RULES, LINE_CAPS, LINE_JOINS, Profile, ProfileInit, TABLE_SCOPES, parse_id,
    };

    /// Whether `table` names each value of `all` exactly once.
    fn names_each<T: PartialEq + Copy>(table: &[(&str, T)], all: &[T]) -> bool {
        table.len() == all.len() && all.iter().all(|v| table.iter().filter(|(_, t)| t == v).count() == 1)
    }

    #[test]
    fn every_boundary_table_should_name_each_variant_once() {
        assert!(names_each(&FILL_RULES, &FillRule::ALL));
        assert!(names_each(&LINE_CAPS, &LineCap::ALL));
        assert!(names_each(&LINE_JOINS, &LineJoin::ALL));
        assert!(names_each(&BLEND_MODES, &BlendMode::ALL));
        let artifacts = ArtifactKind::ALL
            .map(Artifact::new)
            .into_iter()
            .chain([Artifact::header(), Artifact::footer(), Artifact::watermark()])
            .collect::<Vec<_>>();
        assert!(names_each(&ARTIFACT_KINDS, &artifacts));
        assert!(names_each(&TABLE_SCOPES, &TableScope::ALL));
    }

    /// The profile names TypeScript sees are the Rust `Profile::as_str` names.
    #[test]
    fn every_profile_name_is_the_rust_profiles_name() {
        for profile in [Profile::PdfUa1, Profile::PdfA2b, Profile::PdfA2u, Profile::PdfA3b] {
            let name: serde::de::value::StrDeserializer<'_, serde::de::value::Error> =
                profile.as_str().into_deserializer();
            assert_eq!(ProfileInit::deserialize(name).unwrap().to_profile(), profile);
        }
    }

    #[test]
    fn parse_id_refuses_a_sign() {
        let id = format!("+{}", "f".repeat(31));
        assert_eq!(parse_id(&id), None);
    }

    #[test]
    fn parse_id_reads_32_hex_digits() {
        assert_eq!(parse_id("6c616d696e612d65782d68656c6c6f21"), Some(*b"lamina-ex-hello!"));
    }
}
