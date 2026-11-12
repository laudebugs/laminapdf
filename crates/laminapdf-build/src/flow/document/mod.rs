//! `laminapdf.document/1`: a flowing document as JSON, so a program or a language
//! model can write it and a schema can check it before anything is drawn.
//!
//! The document names its fonts and images; their bytes come beside it in [`Resources`], which
//! keeps the builder bytes-in, bytes-out. Text styles are named once in `styles` and referred
//! to by name. [`fill`] fills a template (a document with `$data`, `$each`, `$item` and `$if`
//! placeholders) from data, and [`Builder::document`](crate::Builder::document) lays a document
//! out. The JSON names are camelCase, and block kinds are tagged by `type`. Within `/1`, keys
//! are only added.
//!
//! ```
//! # use laminapdf_build::flow::document::{FlowDocument, Resources};
//! # use laminapdf_build::{BuildOptions, Builder};
//! # fn main() -> Result<(), laminapdf_build::Error> {
//! let json = serde_json::json!({
//!     "schema": "laminapdf.document/1",
//!     "styles": { "body": { "font": "Helvetica", "size": 11 } },
//!     "blocks": [{ "type": "paragraph", "text": "Hello" }]
//! });
//! let document = FlowDocument::from_json(&json)?;
//! let mut b = Builder::new(BuildOptions::new());
//! b.document(&document, &Resources::new())?;
//! # Ok(()) }
//! ```

mod check;
mod convert;
mod fill;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub use fill::{fill, fill_or_read};

use crate::surface::image::Image;
use crate::surface::text::FontHandle;

/// The format's name, the value of a document's `schema`.
pub const DOCUMENT_SCHEMA: &str = "laminapdf.document/1";

/// Where the committed JSON Schema of `laminapdf.document/1` is published: its `$id`.
pub const DOCUMENT_SCHEMA_URL: &str =
    "https://github.com/laudebugs/laminapdf/blob/main/crates/laminapdf-build/schemas/document.schema.json";

/// The JSON Schema (2020-12) of `laminapdf.document/1`. It describes a document, not a
/// template: a template's placeholders are filled before the check.
#[cfg(feature = "schema")]
#[must_use]
pub fn json_schema() -> serde_json::Value {
    let mut schema = schemars::schema_for!(FlowDocument).to_value();
    if let Some(object) = schema.as_object_mut() {
        object.insert("$id".to_owned(), DOCUMENT_SCHEMA_URL.into());
        object.insert("title".to_owned(), DOCUMENT_SCHEMA.into());
    }
    schema
}

/// The fonts and images a document names, by name: the document holds no bytes.
#[derive(Debug, Clone, Default)]
pub struct Resources {
    pub(crate) fonts: BTreeMap<String, FontHandle>,
    pub(crate) images: BTreeMap<String, Image>,
}

impl Resources {
    /// No fonts and no images: styles can name only the standard 14 fonts (`Helvetica`,
    /// `Times-Roman`, `Courier`, …).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// With `font` for the styles whose `font` is `name`.
    #[must_use]
    pub fn with_font(mut self, name: impl Into<String>, font: &FontHandle) -> Self {
        self.fonts.insert(name.into(), font.clone());
        self
    }

    /// With `image` for the figures whose `image` is `name`.
    #[must_use]
    pub fn with_image(mut self, name: impl Into<String>, image: Image) -> Self {
        self.images.insert(name.into(), image);
        self
    }
}

/// Text, or a number or `true`/`false` written as text: what a template's `$data` may put where
/// the document takes text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum Text {
    /// Text as given.
    String(String),
    /// A number, written as JSON writes it.
    Number(serde_json::Number),
    /// `true` or `false`.
    Bool(bool),
}

impl Text {
    pub(crate) fn as_string(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Number(n) => n.to_string(),
            Self::Bool(b) => b.to_string(),
        }
    }
}

/// A `laminapdf.document/1` document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowDocument {
    /// Always `laminapdf.document/1`.
    pub schema: String,
    /// The title: the document's metadata, `{title}` in a header or footer, and the window title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<Text>,
    /// The natural language, a BCP 47 tag such as `en-GB`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// The author, in the metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<Text>,
    /// The subject, in the metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<Text>,
    /// The profiles the PDF must meet: `PDF/UA-1`, and at most one of `PDF/A-2b`, `PDF/A-2u`
    /// and `PDF/A-3b`. The caller passes them to the builder's options, as they are checked
    /// there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<ProfileName>,
    /// The pages the blocks flow through (A4 with 72-point margins when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<PageSpec>,
    /// Whether headings make an outline (bookmarks); `true` when left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline: Option<bool>,
    /// Whether the PDF is tagged for screen readers; `true` when left out (PDF/UA-1 needs it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tagged: Option<bool>,
    /// The heading level whose text `{section}` shows in a header or footer (1 when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 6)))]
    pub section_level: Option<u8>,
    /// The text styles, by name; `body` is the one text takes when it names none.
    pub styles: BTreeMap<String, StyleSpec>,
    /// The blocks, in reading order.
    pub blocks: Vec<BlockSpec>,
}

impl FlowDocument {
    /// The document as JSON.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    /// The profiles it asks for, for [`BuildOptions::with_profile`](crate::BuildOptions::with_profile).
    #[must_use]
    pub fn profiles(&self) -> Vec<crate::Profile> {
        self.profiles.iter().map(|p| p.profile()).collect()
    }

    /// The fonts its styles name, each once, sorted: what a caller gives in [`Resources`],
    /// apart from standard 14 names, which need nothing.
    #[must_use]
    pub fn font_names(&self) -> BTreeSet<&str> {
        self.styles
            .values()
            .flat_map(|style| std::iter::once(&style.font).chain(&style.fallbacks))
            .map(String::as_str)
            .collect()
    }

    /// The images its figures name, each once, sorted: what a caller gives in [`Resources`].
    #[must_use]
    pub fn image_names(&self) -> BTreeSet<&str> {
        fn walk<'a>(blocks: &'a [BlockSpec], out: &mut BTreeSet<&'a str>) {
            for block in blocks {
                match block {
                    BlockSpec::Figure(figure) => {
                        out.insert(figure.image.as_str());
                    }
                    BlockSpec::Keep(group) | BlockSpec::Quote(group) => walk(&group.blocks, out),
                    _ => {}
                }
            }
        }
        let mut out = BTreeSet::new();
        walk(&self.blocks, &mut out);
        out
    }
}

/// `segment` as a JSON Pointer reference token (RFC 6901 §3).
pub(crate) fn escape(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

/// A conformance profile, by its usual name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ProfileName {
    /// PDF/UA-1 (ISO 14289-1).
    #[serde(rename = "PDF/UA-1")]
    PdfUa1,
    /// PDF/A-2b.
    #[serde(rename = "PDF/A-2b")]
    PdfA2b,
    /// PDF/A-2u.
    #[serde(rename = "PDF/A-2u")]
    PdfA2u,
    /// PDF/A-3b.
    #[serde(rename = "PDF/A-3b")]
    PdfA3b,
}

impl ProfileName {
    const fn profile(self) -> crate::Profile {
        match self {
            Self::PdfUa1 => crate::Profile::PdfUa1,
            Self::PdfA2b => crate::Profile::PdfA2b,
            Self::PdfA2u => crate::Profile::PdfA2u,
            Self::PdfA3b => crate::Profile::PdfA3b,
        }
    }
}

/// A text style: a font and a size, and what else the text takes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyleSpec {
    /// A font given beside the document, by name, or a standard 14 font (`Helvetica`, …).
    pub font: String,
    /// The size in points.
    #[cfg_attr(feature = "schema", schemars(range(min = 0)))]
    pub size: f64,
    /// Points the text is raised above the baseline (negative lowers it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rise: Option<f64>,
    /// Fonts for characters `font` lacks, tried in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<String>,
    /// The colour, as `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// The text's language, when it differs from the document's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Struck through.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strikethrough: bool,
}

/// Where a line's text sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum AlignSpec {
    /// The start side (left in left-to-right text).
    Start,
    /// The end side.
    End,
    /// Flush left.
    Left,
    /// Flush right.
    Right,
    /// Centred.
    Center,
}

/// A paragraph or a heading: its text as one string, or as spans of different styles.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphSpec {
    /// The heading's level, 1 to 6: needed by a heading, refused on a paragraph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 6)))]
    pub level: Option<u8>,
    /// The text, in `style`; give this or `spans`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
    /// The text as spans; give this or `text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spans: Option<Vec<SpanSpec>>,
    /// The style of `text`, and of spans that name none (`body` when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Where the lines sit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<AlignSpec>,
    /// The space between lines, as a factor of the size (1.2 when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
    /// Points of space above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_before: Option<f64>,
    /// Points of space below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_after: Option<f64>,
    /// Keep it on the page of the next block.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keep_with_next: bool,
    /// Start it on a new page.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub break_before: bool,
    /// The fewest lines of the paragraph carried to the top of a new page (2 when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(range(min = 1)))]
    pub widows: Option<usize>,
    /// The fewest lines of the paragraph left at the foot of a page before it breaks (2 when
    /// left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(range(min = 1)))]
    pub orphans: Option<usize>,
}

/// A piece of a paragraph: text in a style, a link, code, a line break or a footnote.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpanSpec {
    /// The text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
    /// Its style (the paragraph's when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// A link: a web address, or `#name` for a heading or named destination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    /// The text is code.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub code: bool,
    /// A line break instead of text.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub line_break: bool,
    /// A footnote: its marker here, its text at the foot of the page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<Box<ParagraphSpec>>,
}

/// One block of the flow, tagged by `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum BlockSpec {
    /// A paragraph.
    Paragraph(ParagraphSpec),
    /// A heading of `level` 1 to 6, in the outline and kept with the next block.
    Heading(ParagraphSpec),
    /// A bulleted or numbered list.
    List(ListSpec),
    /// An image, with its alternate text.
    Figure(FigureSpec),
    /// Empty space.
    Spacer(SpacerSpec),
    /// The next block starts a page.
    PageBreak,
    /// Blocks kept on one page.
    Keep(GroupSpec),
    /// A table whose header row repeats on each page it runs onto.
    Table(TableSpec),
    /// A contents page: the headings and the pages they start on.
    Contents(ContentsSpec),
    /// A block quotation.
    Quote(GroupSpec),
    /// Code, line by line as written.
    CodeBlock(CodeBlockSpec),
    /// A horizontal rule.
    Rule,
}

/// Empty space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpacerSpec {
    /// Its height in points.
    #[cfg_attr(feature = "schema", schemars(range(min = 0)))]
    pub height: f64,
}

/// Blocks kept together on one page, or quoted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupSpec {
    /// The blocks.
    pub blocks: Vec<BlockSpec>,
}

/// Code, line by line as written, a tab as spaces to the next multiple of four columns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodeBlockSpec {
    /// The code.
    pub text: Text,
    /// Its style (`body` when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

/// A list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSpec {
    /// `bullet` or `numbered`.
    pub kind: ListKindSpec,
    /// The first number of a numbered list (1 when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,
    /// The markers' style (`body` when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Points the items are indented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<f64>,
    /// The items.
    pub items: Vec<ListItemSpec>,
}

/// A list's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ListKindSpec {
    /// Bullets.
    Bullet,
    /// Numbers.
    Numbered,
}

/// A list item: a paragraph, and a list nested under it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListItemSpec {
    /// The text; give this or `spans`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
    /// The text as spans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spans: Option<Vec<SpanSpec>>,
    /// The text's style (`body` when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// A marker of its own, such as `☑`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    /// A list nested under the item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<Box<ListSpec>>,
}

/// An image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FigureSpec {
    /// An image given beside the document, by name.
    pub image: String,
    /// Its alternate text, which a screen reader reads.
    pub alt: Text,
    /// Its width in points (the body's width at most; its own size when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
}

/// A table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableSpec {
    /// One width per column.
    pub columns: Vec<ColumnSpec>,
    /// The header rows, repeated on each page the table runs onto.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<RowSpec>,
    /// The body rows; a row is never split between pages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<RowSpec>,
    /// The footer rows, kept with the last body row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footer: Vec<RowSpec>,
    /// Points of space inside each cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    /// A rule around the table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<RuleSpec>,
    /// A rule under the header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_rule: Option<RuleSpec>,
    /// A rule between body rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_rules: Option<RuleSpec>,
    /// The header rows' background, as `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_background: Option<String>,
    /// Points of space above the table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_before: Option<f64>,
    /// Points of space below the table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_after: Option<f64>,
}

/// A column's width.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ColumnSpec {
    /// As wide as its widest cell needs, if the table fits.
    Auto,
    /// A width in points: `{ "fixed": 80 }`.
    Fixed(f64),
    /// A share of the room the other columns leave: `{ "fraction": 1 }`.
    Fraction(f64),
}

/// A table row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RowSpec {
    /// The cells, left to right.
    pub cells: Vec<CellSpec>,
}

/// A table cell.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CellSpec {
    /// The text; give this or `spans` (an empty cell has neither).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
    /// The text as spans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spans: Option<Vec<SpanSpec>>,
    /// The text's style (`body` when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// How many columns it spans (1 when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(range(min = 1)))]
    pub span: Option<u32>,
    /// Where its lines sit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<AlignSpec>,
    /// Where its text sits in a row taller than it: `top` (the default), `middle` or `bottom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valign: Option<VAlignSpec>,
}

/// Where a cell's text sits in its row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum VAlignSpec {
    /// At the top.
    Top,
    /// In the middle.
    Middle,
    /// At the bottom.
    Bottom,
}

/// A rule: its width in points and its colour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleSpec {
    /// The width in points.
    pub width: f64,
    /// The colour, as `#rrggbb`.
    pub color: String,
}

/// A contents page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentsSpec {
    /// The entries' style (`body` when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// The deepest heading level it lists, 1 to 6 (2 when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 6)))]
    pub levels: Option<u8>,
    /// Points each level below the first is indented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<f64>,
}

/// The pages a document flows through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageSpec {
    /// A size name (`A4`, `A5`, `Letter`, `Legal`, …) or `[width, height]` in points.
    pub size: PageSizeSpec,
    /// The margins: one number for all four, or `[top, right, bottom, left]` (72 points when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margins: Option<MarginsSpec>,
    /// The header: text at its start, centre and end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<RegionSpec>,
    /// The footer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<RegionSpec>,
    /// The header and footer's style (`body` when left out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Different settings for the first page, such as no header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<Box<PageSpec>>,
}

/// A page size.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum PageSizeSpec {
    /// `A0` to `A6`, `Letter`, `Legal` or `Tabloid`.
    Name(String),
    /// `[width, height]` in points.
    Points([f64; 2]),
}

/// Margins.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum MarginsSpec {
    /// The same on all four sides.
    All(f64),
    /// `[top, right, bottom, left]`.
    Sides([f64; 4]),
}

/// A header or footer: `{page}`, `{pages}`, `{section}` and `{title}` are filled in.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegionSpec {
    /// At the start of the line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<Text>,
    /// Centred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center: Option<Text>,
    /// At the end of the line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<Text>,
}
