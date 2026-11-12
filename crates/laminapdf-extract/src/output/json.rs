//! `laminapdf.extract/1` JSON ([ADR 0030] §1): the top-level keys in the ADR's order, each
//! block with its `textRange` in the Markdown, numbers rounded as §7 says, so the same input
//! and options give the same bytes.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use super::{Chunk, Rendered, TextRange};
use crate::model::{Block, Dropped, ExtractedPage, ExtractedSource, Extraction, SCHEMA};
use crate::plugins::ModelInfo;

/// What the JSON carries besides the extraction.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct JsonOptions {
    /// The document's warnings as ADR 0026 objects (the façade writes them; the extraction
    /// itself doesn't hold them).
    pub warnings: Vec<serde_json::Value>,
    /// The chunks, when they were made (`chunks` is absent otherwise).
    pub chunks: Option<Vec<Chunk>>,
}

impl JsonOptions {
    /// No warnings and no chunks.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The warnings.
    #[must_use]
    pub fn with_warnings(mut self, warnings: Vec<serde_json::Value>) -> Self {
        self.warnings = warnings;
        self
    }

    /// The chunks.
    #[must_use]
    pub fn with_chunks(mut self, chunks: Vec<Chunk>) -> Self {
        self.chunks = Some(chunks);
        self
    }
}

/// Who made the extraction: the library, its version, and the models it ran, if any.
#[derive(Debug, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub(crate) struct Generator<'a> {
    name: &'static str,
    version: &'static str,
    /// `"rules"`, or `"rules+models"` when a model plug-in was used.
    mode: &'static str,
    /// Each model used: name, version, weights' SHA-256 and licence.
    models: &'a [ModelInfo],
}

/// A block and where its text sits in the Markdown.
#[derive(Debug, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsonBlock<'a> {
    #[serde(flatten)]
    block: &'a Block,
    text_range: TextRange,
}

/// The whole document.
#[derive(Debug, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsonDocument<'a> {
    /// Always `"laminapdf.extract/1"`.
    schema: &'static str,
    generator: Generator<'a>,
    source: &'a ExtractedSource,
    pages: &'a [ExtractedPage],
    blocks: Vec<JsonBlock<'a>>,
    dropped: &'a [Dropped],
    warnings: &'a [serde_json::Value],
    #[serde(skip_serializing_if = "Option::is_none")]
    chunks: Option<&'a [Chunk]>,
}

/// `extraction` as `laminapdf.extract/1` JSON, one line. `markdown` is
/// [`Extraction::to_markdown`]'s output, which `textRange` points into.
#[must_use]
pub fn to_json(extraction: &Extraction, markdown: &Rendered, options: &JsonOptions) -> String {
    let document = JsonDocument {
        schema: SCHEMA,
        generator: Generator {
            name: "laminapdf",
            version: env!("CARGO_PKG_VERSION"),
            mode: if extraction.models.is_empty() {
                "rules"
            } else {
                "rules+models"
            },
            models: &extraction.models,
        },
        source: &extraction.source,
        pages: &extraction.pages,
        blocks: extraction
            .blocks
            .iter()
            .enumerate()
            .map(|(i, block)| JsonBlock {
                block,
                text_range: markdown.ranges.get(i).copied().unwrap_or_default(),
            })
            .collect(),
        dropped: &extraction.dropped,
        warnings: &options.warnings,
        chunks: options.chunks.as_deref(),
    };
    // Every value is a string, a number, a bool or a sequence of them: serialising can't fail.
    serde_json::to_string(&document).unwrap_or_default()
}

/// The chunks as JSON Lines: one chunk object per line, for streaming into an index.
#[must_use]
pub fn chunks_jsonl(chunks: &[Chunk]) -> String {
    chunks
        .iter()
        .filter_map(|c| serde_json::to_string(c).ok())
        .map(|line| line + "\n")
        .collect()
}

/// The JSON Schema (2020-12) of [`to_json`]'s output, as `tests/json_schema.rs` commits it.
#[cfg(feature = "schema")]
#[must_use]
pub fn json_schema() -> serde_json::Value {
    let mut schema = schemars::schema_for!(JsonDocument<'static>).to_value();
    if let Some(object) = schema.as_object_mut() {
        object.insert(
            "$id".to_owned(),
            serde_json::Value::String("https://docs.laminapdf.com/schemas/extract-1.json".to_owned()),
        );
        object.insert("title".to_owned(), serde_json::Value::String(SCHEMA.to_owned()));
    }
    schema
}
