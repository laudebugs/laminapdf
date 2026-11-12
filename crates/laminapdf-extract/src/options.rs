//! What an extraction does: [`ExtractOptions`], owned data with a `Default`. All of it but
//! [`Models`]' plug-in objects crosses `UniFFI` and WASM as a record (ADR 0028 §1); the
//! bindings name host models instead, which needs no object across the boundary.

use crate::plugins::Models;
use crate::tables::TableMode;

/// How an extraction runs.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExtractOptions {
    /// The page indices to extract, in this order; `None` for every page.
    pub pages: Option<Vec<usize>>,
    /// Where the reading order comes from.
    pub reading_order: ReadingOrder,
    /// What happens to hidden text.
    pub drop: DropOptions,
    /// Join a word hyphenated across two lines (ADR 0030 §5).
    pub dehyphenate: bool,
    /// Bounds on the work per page.
    pub limits: ExtractLimits,
    /// Which table detectors run.
    pub tables: TableMode,
    /// The cut-offs that decide each page's text-layer status.
    pub text_layer: TextLayerThresholds,
    /// Layout and OCR models; none by default (rules only, ADR 0030 §6).
    pub models: Models,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            pages: None,
            reading_order: ReadingOrder::default(),
            drop: DropOptions::default(),
            dehyphenate: true,
            limits: ExtractLimits::default(),
            tables: TableMode::default(),
            text_layer: TextLayerThresholds::default(),
            models: Models::default(),
        }
    }
}

impl ExtractOptions {
    /// The defaults: every page, reading order from the layout, hidden text and page furniture
    /// dropped, de-hyphenation on.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Only these pages, in this order.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_pages(mut self, pages: Vec<usize>) -> Self {
        self.pages = Some(pages);
        self
    }

    /// The reading order.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_reading_order(mut self, order: ReadingOrder) -> Self {
        self.reading_order = order;
        self
    }

    /// Which table detectors run.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_tables(mut self, mode: TableMode) -> Self {
        self.tables = mode;
        self
    }

    /// The cut-offs that decide each page's text-layer status.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_text_layer(mut self, thresholds: TextLayerThresholds) -> Self {
        self.text_layer = thresholds;
        self
    }

    /// The layout and OCR models.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_models(mut self, models: Models) -> Self {
        self.models = models;
        self
    }
}

/// The cut-offs that decide a page's text-layer status ([`crate::TextLayerStatus`]): starting
/// values, which a pipeline can tune to its documents. Shares are in thousandths (500 is half),
/// as the measures are rounded to them: of the page's area, or of its visible glyphs.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TextLayerThresholds {
    /// Images covering at least this share of the page make it an image page (500).
    pub image_page: u16,
    /// Images covering at least this share make a page with text `mixed` (250).
    pub image_mixed: u16,
    /// An image page with at most this many visible characters (a stamp, a page number) needs
    /// OCR (200).
    pub few_chars: usize,
    /// At least this share of unmapped glyphs means the page needs OCR (500).
    pub unmapped_needs_ocr: u16,
    /// At least this share of unmapped glyphs makes a page `mixed` (100).
    pub unmapped_mixed: u16,
    /// A page with no text and no image that paints at least this many paths isn't blank: its
    /// text may be outlined (16).
    pub drawn_paths: usize,
}

impl Default for TextLayerThresholds {
    fn default() -> Self {
        Self {
            image_page: 500,
            image_mixed: 250,
            few_chars: 200,
            unmapped_needs_ocr: 500,
            unmapped_mixed: 100,
            drawn_paths: 16,
        }
    }
}

/// Where the reading order comes from ([ADR 0030] §2).
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ReadingOrder {
    /// The best source available: on each page, the structure tree when the file is tagged and
    /// the tree passes ADR 0030 §2's plausibility checks, else the layout (XY-Cut++).
    #[default]
    Auto,
    /// The structure tree on every page that has one, plausible or not; the layout elsewhere.
    Tagged,
    /// The layout, by XY-Cut++, whether or not the file is tagged.
    Layout,
    /// The order the content stream draws, for debugging and comparison.
    ContentStream,
}

/// What happens to hidden text and page furniture ([ADR 0030] §4). Hidden text is never in a
/// block's text.
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DropOptions {
    /// Keep each dropped run, text included, in its `dropped` entry: for auditing a file.
    pub keep_spans: bool,
    /// Drop page furniture too: text marked as an `Artifact`, running headers and footers, and
    /// page numbers, each listed in `dropped` instead of the blocks. On by default.
    pub furniture: bool,
}

impl Default for DropOptions {
    fn default() -> Self {
        Self {
            keep_spans: false,
            furniture: true,
        }
    }
}

/// Bounds on the work per page: soft limits that leave work out with a warning (ADR 0003).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ExtractLimits {
    /// Runs of text kept per page, so at most this many blocks: the page's first runs in drawing
    /// order (`extract-limit-blocks` past it). Grouping and ordering grow faster than the runs
    /// do, so the limit applies before them.
    pub max_blocks_per_page: u32,
    /// How deep XY-Cut recurses; deeper groups keep top-to-bottom order.
    pub max_xycut_depth: u32,
    /// Cells per ruled table, and (times 16) crossings of a page's rules tried: past it the
    /// rules give no table (`extract-table-limit`), as a file drawing thousands of lines would
    /// otherwise cost time and memory in proportion to their square.
    pub max_table_cells: u32,
}

impl Default for ExtractLimits {
    fn default() -> Self {
        Self {
            max_blocks_per_page: 10_000,
            max_xycut_depth: 64,
            max_table_cells: 10_000,
        }
    }
}
