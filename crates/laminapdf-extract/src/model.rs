//! The output: [`Extraction`] and its parts, the Rust form of `laminapdf.extract/1` ([ADR 0030]
//! §1). With the `serde` feature they serialise under the JSON names (camel-case keys,
//! kebab-case values, boxes as `[x, y, width, height]` rounded to 1/1000 of a point).
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use laminapdf_model::{Pt, RectPt, Rotation};

/// The schema name every serialised extraction carries.
pub const SCHEMA: &str = "laminapdf.extract/1";

/// One extraction: the pages it read and their blocks in reading order, with what it dropped.
/// The document's warnings stay on the document (`laminapdf_cos::Document::warnings`); a
/// serialiser adds them as `warnings`.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Extraction {
    /// What was read.
    pub source: ExtractedSource,
    /// The pages extracted, in the order asked for.
    pub pages: Vec<ExtractedPage>,
    /// Every block of those pages, page by page, each page's in reading order.
    pub blocks: Vec<Block>,
    /// What was left out of the text, and why.
    pub dropped: Vec<Dropped>,
    /// The models used ([ADR 0030] §6), layout first; empty for rules only. The JSON names
    /// them in `generator.models`.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub models: Vec<crate::plugins::ModelInfo>,
}

/// The input, as ADR 0030 §1's `source` describes it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ExtractedSource {
    /// The input's SHA-256 in hex: filled by the caller that holds the bytes (the façade), since
    /// a windowed source can't be hashed without reading it whole.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub sha256: Option<String>,
    /// The document's page count.
    pub page_count: usize,
    /// Whether the catalog's `/MarkInfo` says `/Marked true`.
    pub tagged: bool,
    /// Where the reading order came from.
    pub struct_source: StructSource,
}

/// Where the reading order came from ([ADR 0030] §2).
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum StructSource {
    /// The structure tree.
    Tagged,
    /// XY-Cut++ over the blocks' boxes.
    #[default]
    #[cfg_attr(feature = "serde", serde(rename = "xycut"))]
    XyCut,
    /// The order the content stream draws.
    Content,
}

/// One extracted page ([ADR 0027] §4).
///
/// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ExtractedPage {
    /// 0-based.
    pub index: usize,
    /// 1-based, as people count.
    pub number: usize,
    /// The page label (`/PageLabels`), when there is one.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub label: Option<String>,
    /// The displayed width in points (after `/Rotate` and `/UserUnit`).
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::pt"))]
    #[cfg_attr(feature = "schema", schemars(with = "f64"))]
    pub width: Pt,
    /// The displayed height in points.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::pt"))]
    #[cfg_attr(feature = "schema", schemars(with = "f64"))]
    pub height: Pt,
    /// The frame the boxes are in.
    pub coordinates: Coordinates,
}

/// ADR 0027 §4's `coordinates` block: top-left points of the displayed box.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Coordinates {
    /// Always `"top-left"`.
    pub origin: &'static str,
    /// Always `"pt"`.
    pub unit: &'static str,
    /// `"CropBox"`, or `"default"` when the page had no usable box and US Letter was used.
    #[cfg_attr(feature = "serde", serde(rename = "box"))]
    pub box_used: &'static str,
    /// The `/Rotate` applied, in degrees clockwise.
    pub rotate: u16,
    /// The `/UserUnit` applied.
    pub user_unit: f64,
}

impl Coordinates {
    /// The block for a page with this rotation and unit, `letter` when the fallback was used.
    #[must_use]
    pub fn new(rotate: Rotation, user_unit: f64, letter: bool) -> Self {
        Self {
            origin: "top-left",
            unit: "pt",
            box_used: if letter { "default" } else { "CropBox" },
            rotate: rotate.degrees(),
            user_unit,
        }
    }
}

/// A block of text: a paragraph, heading, list item and so on ([ADR 0030] §1).
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Block {
    /// `p<page index>-b<position in the page's reading order>`.
    pub id: String,
    /// What the block is.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub kind: BlockKind,
    /// The visible text, lines joined by a space after the text fixes (ADR 0030 §5).
    pub text: String,
    /// The page index.
    pub page: usize,
    /// The block's box in page space.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::rect"))]
    #[cfg_attr(feature = "schema", schemars(with = "[f64; 4]"))]
    pub bbox: RectPt,
    /// The box divided by the page's width and height, clamped to [0, 1]; `None` when the
    /// block lies wholly outside the page.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::norm"))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<[f64; 4]>"))]
    pub bbox_norm: Option<[f64; 4]>,
    /// The structure types from the root down to the block; empty when untagged.
    pub struct_path: Vec<String>,
    /// The structure element the block comes from, when it comes from one.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub struct_ref: Option<StructRef>,
    /// The runs of text, in reading order.
    pub spans: Vec<Span>,
    /// In [0, 1]: 1 for structure from the tree, lower for structure inferred from layout.
    pub confidence: f64,
    /// A table's cells, row by row; empty for every other block.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    #[cfg_attr(feature = "schema", schemars(default))]
    pub cells: Vec<Cell>,
    /// A layout model's region gave the block its kind, so the heading and list rules leave
    /// it alone, as they leave the structure tree's blocks.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub(crate) from_model: bool,
}

/// One cell of a table block.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Cell {
    /// The first row it covers, 0 at the top.
    pub row: usize,
    /// The first column it covers, 0 at the left.
    pub col: usize,
    /// How many rows it covers.
    pub row_span: usize,
    /// How many columns it covers.
    pub col_span: usize,
    /// Whether it is in a header row.
    pub header: bool,
    /// Its visible text, lines joined by a space.
    pub text: String,
    /// Its box in page space.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::rect"))]
    #[cfg_attr(feature = "schema", schemars(with = "[f64; 4]"))]
    pub bbox: RectPt,
}

/// What found a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum TableDetector {
    /// The lines that rule it.
    Lattice,
    /// Text lined up in columns, with no lines.
    Stream,
    /// The structure tree's `Table` element (ISO 32000-2 §14.8.4.8.3).
    Tagged,
}

/// What a block is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "type", rename_all = "kebab-case", rename_all_fields = "camelCase")
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum BlockKind {
    /// A heading, level 1 (the top) to 6.
    Heading {
        /// 1–6.
        level: u8,
    },
    /// Running text.
    Paragraph,
    /// One item of a list.
    ListItem {
        /// The bullet or number that starts it.
        marker: String,
        /// 0 for the outermost list.
        depth: u8,
    },
    /// A figure's or table's caption.
    Caption,
    /// A footnote.
    Footnote,
    /// Anything else.
    Other,
    /// A table: its text is the cells', a tab between cells and a line break between rows, and
    /// the block's `cells` hold them.
    Table {
        /// The grid's rows.
        rows: usize,
        /// The grid's columns.
        cols: usize,
        /// What found it.
        detector: TableDetector,
    },
}

/// A structure element ([ADR 0030] §1's `structRef`).
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct StructRef {
    /// The element's object number.
    pub obj: u32,
    /// Its generation.
    pub r#gen: u16,
    /// The marked-content identifiers it covers on the page.
    pub mcids: Vec<i64>,
}

/// One run of text in a block or a dropped entry.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Span {
    /// The run's text.
    pub text: String,
    /// Its box in page space.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::rect"))]
    #[cfg_attr(feature = "schema", schemars(with = "[f64; 4]"))]
    pub bbox: RectPt,
    /// The baseline's y in page space (ADR 0027 §5).
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::pt"))]
    #[cfg_attr(feature = "schema", schemars(with = "f64"))]
    pub baseline: Pt,
    /// The font's `/BaseFont`.
    pub font: String,
    /// The glyph size in points.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::pt"))]
    #[cfg_attr(feature = "schema", schemars(with = "f64"))]
    pub size: Pt,
    /// Why the run would be hidden; empty for visible text.
    pub flags: Flags,
    /// The OCR engine's confidence, 0 to 1, when an engine read this text from the page image
    /// rather than the file's text layer ([ADR 0030] §6); absent for the text layer's runs.
    ///
    /// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ocr: Option<f64>,
}

/// Why text is hidden from a reader ([ADR 0030] §4). Flagged text is never in a block's text.
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[expect(
    clippy::struct_excessive_bools,
    reason = "ADR 0030 §4's six independent flags, each serialised by name"
)]
pub struct Flags {
    /// Drawn in text rendering mode 3 or 7.
    pub invisible: bool,
    /// Its box lies outside the page.
    pub off_page: bool,
    /// Drawn below 1 pt.
    pub tiny: bool,
    /// Its box lies outside the clip's box.
    pub clipped: bool,
    /// Filled near white with nothing painted under it.
    pub low_contrast: bool,
    /// Painted over later on the page: by an opaque fill, an opaque image or an annotation's
    /// appearance.
    pub covered: bool,
}

impl Flags {
    /// No flag: visible text.
    pub const NONE: Self = Self {
        invisible: false,
        off_page: false,
        tiny: false,
        clipped: false,
        low_contrast: false,
        covered: false,
    };

    /// Whether any flag is set.
    #[must_use]
    pub const fn any(self) -> bool {
        self.invisible || self.off_page || self.tiny || self.clipped || self.low_contrast || self.covered
    }

    /// The flags set, as the reasons a dropped entry names, in a fixed order.
    #[must_use]
    pub fn reasons(self) -> Vec<DropReason> {
        [
            (self.invisible, DropReason::Invisible),
            (self.off_page, DropReason::OffPage),
            (self.tiny, DropReason::Tiny),
            (self.clipped, DropReason::Clipped),
            (self.low_contrast, DropReason::LowContrast),
            (self.covered, DropReason::Covered),
        ]
        .into_iter()
        .filter_map(|(set, reason)| set.then_some(reason))
        .collect()
    }
}

/// Text left out of the output ([ADR 0030] §4). It records a count, never the text, so a log of
/// the output can't leak what was hidden.
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Dropped {
    /// Why it was left out (the first reason when there are several).
    pub reason: DropReason,
    /// The page index.
    pub page: usize,
    /// Where it was, in page space.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::rect"))]
    #[cfg_attr(feature = "schema", schemars(with = "[f64; 4]"))]
    pub bbox: RectPt,
    /// How many characters were left out.
    pub chars: usize,
    /// The run itself, only when [`DropOptions::keep_spans`](crate::DropOptions::keep_spans)
    /// asked for it.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    #[cfg_attr(feature = "schema", schemars(default))]
    pub spans: Vec<Span>,
}

/// Why text was left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum DropReason {
    /// Text rendering mode 3 or 7.
    Invisible,
    /// Outside the page.
    OffPage,
    /// Below 1 pt.
    Tiny,
    /// Outside the clip.
    Clipped,
    /// Near white on nothing.
    LowContrast,
    /// Painted over by a later opaque shape, image or annotation.
    Covered,
    /// Marked content tagged `Artifact`.
    Artifact,
    /// A line repeated at the top of pages.
    RunningHeader,
    /// A line repeated at the bottom of pages.
    RunningFooter,
    /// A page number.
    PageNumber,
}

#[cfg(feature = "serde")]
impl serde::Serialize for Flags {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.reasons())
    }
}

/// `flags` is a list of [`DropReason`] names in JSON.
#[cfg(feature = "schema")]
impl schemars::JsonSchema for Flags {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Flags".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = generator.subschema_for::<Vec<DropReason>>();
        schema.insert(
            "description".to_owned(),
            "Why a run would be hidden from a reader; empty for visible text.".into(),
        );
        schema
    }
}

#[cfg(feature = "serde")]
pub(crate) mod ser {
    //! Boxes as `[x, y, width, height]` and lengths, rounded to 1/1000 of a point (ADR 0030 §7).

    use laminapdf_model::{Pt, RectPt};
    use serde::Serializer;

    fn round(v: f64) -> f64 {
        (v * 1000.0).round() / 1000.0
    }

    #[cfg_attr(
        target_pointer_width = "64",
        expect(
            clippy::trivially_copy_pass_by_ref,
            reason = "serde's serialize_with passes a reference (as wide as a Pt here)"
        )
    )]
    pub(crate) fn pt<S: Serializer>(v: &Pt, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(round(v.get()))
    }

    pub(crate) fn rect<S: Serializer>(r: &RectPt, serializer: S) -> Result<S::Ok, S::Error> {
        let values = [r.x0.get(), r.y0.get(), r.width().get(), r.height().get()];
        serializer.collect_seq(values.map(round))
    }

    #[expect(clippy::ref_option, reason = "serde's serialize_with passes a reference")]
    pub(crate) fn norm<S: Serializer>(v: &Option<[f64; 4]>, serializer: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(values) => serializer.collect_seq(values.map(round)),
            None => serializer.serialize_none(),
        }
    }
}
