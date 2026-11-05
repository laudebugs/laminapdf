//! What an extraction does: [`ExtractOptions`], plain owned data with a `Default` so it crosses
//! `UniFFI` and WASM as a record (ADR 0028 §1).

use crate::tables::TableMode;

/// How an extraction runs.
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
    /// Leave page furniture out of the blocks (ADR 0030 §4): text marked as an `Artifact`,
    /// running headers and footers, and page numbers, each listed in `dropped` instead. On by
    /// default; off keeps them as blocks.
    pub headers_footers: bool,
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
            headers_footers: true,
        }
    }
}

impl ExtractOptions {
    /// The defaults: every page, reading order from the layout, hidden text and page furniture
    /// dropped, de-hyphenation on.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Only these pages, in this order.
    #[must_use]
    pub fn with_pages(mut self, pages: Vec<usize>) -> Self {
        self.pages = Some(pages);
        self
    }

    /// The reading order.
    #[must_use]
    pub const fn with_reading_order(mut self, order: ReadingOrder) -> Self {
        self.reading_order = order;
        self
    }

    /// Whether page furniture (artifacts, running headers and footers, page numbers) is left
    /// out of the blocks.
    #[must_use]
    pub const fn with_headers_footers(mut self, drop: bool) -> Self {
        self.headers_footers = drop;
        self
    }

    /// Which table detectors run.
    #[must_use]
    pub const fn with_tables(mut self, mode: TableMode) -> Self {
        self.tables = mode;
        self
    }
}

/// Where the reading order comes from ([ADR 0030] §2).
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ReadingOrder {
    /// The best source available. Today that is the layout (XY-Cut++); the structure tree of a
    /// plausibly tagged file comes first once it is read.
    #[default]
    Auto,
    /// The layout, by XY-Cut++, whether or not the file is tagged.
    Layout,
    /// The order the content stream draws, for debugging and comparison.
    ContentStream,
}

/// What happens to hidden text ([ADR 0030] §4). Hidden text is never in a block's text.
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct DropOptions {
    /// Keep each hidden run, text included, in its `dropped` entry: for auditing a file.
    pub keep_spans: bool,
}

/// Bounds on the work per page: soft limits that leave work out with a warning (ADR 0003).
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
