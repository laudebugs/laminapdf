//! Extraction for retrieval across the boundary: `WasmDocument.extract(options)` returns one
//! string, the Markdown, the plain text, the `laminapdf.extract/1` JSON or the chunks as a JSON
//! array, so the output crosses in one copy and `@laminapdf/sdk` parses what it needs.

use laminapdf::extract::{
    ChunkOptions, ExtractOptions, Extraction, MarkdownOptions, ReadingOrder, Rendered, SplitOn, TableMode,
};
use serde::Deserialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::from_js;
use crate::document::WasmDocument;
use crate::error::{JsResult, internal, lamina_error};

/// What `WasmDocument.extract` returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Tsify)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ExtractFormat {
    /// Markdown in reading order, tables as pipe tables.
    Markdown,
    /// The `laminapdf.extract/1` JSON: blocks with boxes, and chunks when `chunk` is given.
    Json,
    /// Plain text in reading order.
    Text,
    /// The Markdown cut into chunks for retrieval, as a JSON array.
    Chunks,
}

/// Where the reading order comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Tsify)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ReadingOrderName {
    /// The best source available (today the layout).
    Auto,
    /// The layout, by XY-Cut++.
    Layout,
    /// The order the content stream draws.
    ContentStream,
}

/// Which table detectors run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Tsify)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TableModeName {
    /// No tables: every block is text.
    Off,
    /// Tables drawn with lines (the default).
    Lattice,
    /// Tables drawn with lines, then text lined up in columns.
    LatticeAndStream,
}

/// Where chunks may start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Tsify)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SplitOnName {
    /// At every heading.
    Headings,
    /// Only where the budget runs out.
    Blocks,
    /// At every page.
    Pages,
}

/// What happens to hidden text.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DropInit {
    /// Keep each hidden run's text in its `dropped` entry (JSON only), for auditing a file.
    #[tsify(optional)]
    pub keep_spans: Option<bool>,
}

/// How the Markdown is cut into chunks.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ChunkInit {
    /// A chunk's budget in approximate tokens (characters / 4); 512 by default.
    #[tsify(optional)]
    pub max_tokens: Option<u32>,
    /// Approximate tokens of whole blocks repeated from the chunk before; 0 by default.
    #[tsify(optional)]
    pub overlap: Option<u32>,
    /// Where chunks may start; `"headings"` by default.
    #[tsify(optional)]
    pub split_on: Option<SplitOnName>,
}

/// Options for `WasmDocument.extract`.
#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExtractInit {
    /// What to return.
    pub format: ExtractFormat,
    /// The 0-based page indices to extract, in this order; every page by default.
    #[tsify(optional)]
    pub pages: Option<Vec<u32>>,
    /// Where the reading order comes from; `"auto"` by default.
    #[tsify(optional)]
    pub reading_order: Option<ReadingOrderName>,
    /// What happens to hidden text.
    #[tsify(optional)]
    pub drop: Option<DropInit>,
    /// Which table detectors run; `"lattice"` by default.
    #[tsify(optional)]
    pub tables: Option<TableModeName>,
    /// Join words hyphenated across lines; on by default.
    #[tsify(optional)]
    pub dehyphenate: Option<bool>,
    /// Leave running headers, footers, page numbers and artifact-marked text out of the
    /// blocks (listed in `dropped` instead); on by default.
    #[tsify(optional)]
    pub headers_footers: Option<bool>,
    /// Write `<!-- page N -->` before each page's first block (Markdown, chunks and JSON).
    #[tsify(optional)]
    pub page_markers: Option<bool>,
    /// How chunks are cut (`chunks`, and `json` to include them).
    #[tsify(optional)]
    pub chunk: Option<ChunkInit>,
}

impl ExtractInit {
    /// The options, starting from `ExtractOptions`' defaults and changing only what was given,
    /// so the binding keeps no copy of them.
    fn to_options(&self) -> ExtractOptions {
        let mut options = ExtractOptions::new();
        if let Some(order) = self.reading_order {
            options = options.with_reading_order(match order {
                ReadingOrderName::Auto => ReadingOrder::Auto,
                ReadingOrderName::Layout => ReadingOrder::Layout,
                ReadingOrderName::ContentStream => ReadingOrder::ContentStream,
            });
        }
        if let Some(mode) = self.tables {
            options = options.with_tables(match mode {
                TableModeName::Off => TableMode::Off,
                TableModeName::Lattice => TableMode::Lattice,
                TableModeName::LatticeAndStream => TableMode::LatticeAndStream,
            });
        }
        if let Some(pages) = &self.pages {
            options = options.with_pages(pages.iter().map(|&p| p as usize).collect());
        }
        if let Some(keep) = self.drop.as_ref().and_then(|d| d.keep_spans) {
            options.drop.keep_spans = keep;
        }
        if let Some(on) = self.dehyphenate {
            options.dehyphenate = on;
        }
        if let Some(on) = self.headers_footers {
            options = options.with_headers_footers(on);
        }
        options
    }

    fn chunk_options(&self) -> ChunkOptions {
        let init = self.chunk.as_ref();
        let mut options = ChunkOptions::new();
        if let Some(max) = init.and_then(|c| c.max_tokens) {
            options = options.with_max_tokens(max as usize);
        }
        if let Some(overlap) = init.and_then(|c| c.overlap) {
            options = options.with_overlap(overlap as usize);
        }
        if let Some(split) = init.and_then(|c| c.split_on) {
            options = options.with_split_on(match split {
                SplitOnName::Headings => SplitOn::Headings,
                SplitOnName::Blocks => SplitOn::Blocks,
                SplitOnName::Pages => SplitOn::Pages,
            });
        }
        options
    }

    fn markdown(&self, extraction: &Extraction) -> Rendered {
        extraction.to_markdown(&MarkdownOptions::new().with_page_markers(self.page_markers.unwrap_or(false)))
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The document's text for retrieval and LLMs, as one string: Markdown, plain text, the
    /// `laminapdf.extract/1` JSON, or the chunks as a JSON array (the extract and text modules).
    ///
    /// # Errors
    /// `page-index-out-of-range` for a page in `pages` past the end; `data-pending` and the
    /// reading errors; `unknown-option` / `invalid-argument` for bad options.
    pub fn extract(&self, #[wasm_bindgen(unchecked_param_type = "ExtractInit")] options: &JsValue) -> JsResult<String> {
        let init: ExtractInit = from_js(options, "options")?;
        let doc = self.shared().borrow();
        let extraction = doc.extract(&init.to_options()).map_err(|e| lamina_error(&e))?;
        Ok(match init.format {
            ExtractFormat::Text => extraction.to_text().text,
            ExtractFormat::Markdown => init.markdown(&extraction).text,
            ExtractFormat::Chunks => {
                let markdown = init.markdown(&extraction);
                let chunks = extraction.chunks(&markdown, &init.chunk_options());
                serde_json::to_string(&chunks).map_err(|_| internal("the chunks could not be written as JSON"))?
            }
            ExtractFormat::Json => {
                let markdown = init.markdown(&extraction);
                let chunks = init
                    .chunk
                    .is_some()
                    .then(|| extraction.chunks(&markdown, &init.chunk_options()));
                doc.extract_json(&extraction, &markdown, chunks)
            }
        })
    }
}
