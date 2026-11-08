//! Extraction for retrieval across the boundary: `WasmDocument.extract(options)` returns one
//! string, the Markdown, the plain text, the `laminapdf.extract/1` JSON or the chunks as a JSON
//! array, so the output crosses in one copy and `@laminapdf/sdk` parses what it needs.

use js_sys::Reflect;
use laminapdf::extract::ExtractionJob;
use laminapdf::extract::{
    ChunkOptions, ExtractOptions, Extraction, LayoutRegion, MarkdownOptions, ModelInfo, ModelRequest, ModelResult,
    ModelSlot, Models, OcrWord, ReadingOrder, RegionLabel, Rendered, SplitOn, TableMode,
};
use laminapdf::{Budget, Pt, RectPt, Step};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, to_js};
use crate::document::{WasmDocument, upgrade};
use crate::dto::phase_name;
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
    /// The best source available: a plausible structure tree, else the layout.
    Auto,
    /// The structure tree wherever there is one, plausible or not.
    Tagged,
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

/// What happens to hidden text and page furniture.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DropInit {
    /// Keep each dropped run's text in its `dropped` entry (JSON only), for auditing a file.
    #[tsify(optional)]
    pub keep_spans: Option<bool>,
    /// Drop running headers, footers, page numbers and artifact-marked text too, listing them
    /// in `dropped`; on by default.
    #[tsify(optional)]
    pub furniture: Option<bool>,
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
    /// Write `<!-- page N -->` before each page's first block (Markdown, chunks and JSON).
    #[tsify(optional)]
    pub page_markers: Option<bool>,
    /// How chunks are cut (`chunks`, and `json` to include them).
    #[tsify(optional)]
    pub chunk: Option<ChunkInit>,
    /// Models the host runs (`extractJob` only): the job asks for their results.
    #[tsify(optional)]
    pub models: Option<ModelsInit>,
}

/// A model the host runs, as the output's `generator.models` names it.
#[derive(Debug, Clone, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModelInfoInit {
    /// Its name.
    pub name: String,
    /// Its version or revision.
    pub version: String,
    /// Its weights' SHA-256, lowercase hex.
    pub sha256: String,
    /// Its weights' SPDX licence.
    pub licence: String,
}

/// The models the host runs for an extraction job.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModelsInit {
    /// A layout model: every page's regions are asked for.
    #[tsify(optional)]
    pub layout: Option<ModelInfoInit>,
    /// An OCR engine: pages with no visible text are asked for.
    #[tsify(optional)]
    pub ocr: Option<ModelInfoInit>,
    /// The resolution pages are asked for at; 150 by default.
    #[tsify(optional)]
    pub dpi: Option<u32>,
    /// OCR's languages (BCP 47).
    #[tsify(optional)]
    pub langs: Option<Vec<String>>,
}

fn info(init: &ModelInfoInit) -> ModelInfo {
    ModelInfo::new(&init.name, &init.version, &init.sha256, &init.licence)
}

impl ExtractInit {
    /// The options, starting from `ExtractOptions`' defaults and changing only what was given,
    /// so the binding keeps no copy of them.
    fn to_options(&self) -> ExtractOptions {
        let mut options = ExtractOptions::new();
        if let Some(init) = &self.models {
            let mut models = Models::new();
            if let Some(layout) = &init.layout {
                models = models.with_layout(ModelSlot::Host(info(layout)));
            }
            if let Some(ocr) = &init.ocr {
                models = models.with_ocr(ModelSlot::Host(info(ocr)));
            }
            models.dpi = init.dpi.unwrap_or(0);
            models.langs = init.langs.clone().unwrap_or_default();
            options = options.with_models(models);
        }
        if let Some(order) = self.reading_order {
            options = options.with_reading_order(match order {
                ReadingOrderName::Auto => ReadingOrder::Auto,
                ReadingOrderName::Tagged => ReadingOrder::Tagged,
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
        if let Some(on) = self.drop.as_ref().and_then(|d| d.furniture) {
            options.drop.furniture = on;
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
        init.output(&doc, &extraction)
    }

    /// An extraction stepped a page at a time, whose models the host runs: each step returns
    /// its progress, with the result it waits for under `request` (render that page at `dpi`,
    /// run the model, hand the result to `provide`), and once every page is done, the output in
    /// `options.format` (the extract and text modules).
    ///
    /// # Errors
    /// As `extract`, for the options and the pages asked for.
    #[wasm_bindgen(js_name = extractJob)]
    pub fn extract_job(
        &self,
        #[wasm_bindgen(unchecked_param_type = "ExtractInit")] options: &JsValue,
    ) -> JsResult<WasmExtractJob> {
        let init: ExtractInit = from_js(options, "options")?;
        let job = self
            .shared()
            .borrow()
            .extract_job(&init.to_options())
            .map_err(|e| lamina_error(&e))?;
        Ok(WasmExtractJob {
            doc: std::rc::Rc::downgrade(self.shared()),
            job,
            init,
        })
    }
}

impl ExtractInit {
    /// The extraction in `self.format`.
    fn output(&self, doc: &laminapdf::Document, extraction: &Extraction) -> JsResult<String> {
        Ok(match self.format {
            ExtractFormat::Text => extraction.to_text().text,
            ExtractFormat::Markdown => self.markdown(extraction).text,
            ExtractFormat::Chunks => {
                let markdown = self.markdown(extraction);
                let chunks = extraction.chunks(&markdown, &self.chunk_options());
                serde_json::to_string(&chunks).map_err(|_| internal("the chunks could not be written as JSON"))?
            }
            ExtractFormat::Json => {
                let markdown = self.markdown(extraction);
                let chunks = self
                    .chunk
                    .is_some()
                    .then(|| extraction.chunks(&markdown, &self.chunk_options()));
                doc.extract_json(extraction, &markdown, chunks)
            }
        })
    }
}

/// An extraction in progress, from `WasmDocument.extractJob`; it holds its document weakly.
#[wasm_bindgen]
pub struct WasmExtractJob {
    doc: std::rc::Weak<std::cell::RefCell<laminapdf::Document>>,
    job: ExtractionJob,
    init: ExtractInit,
}

/// A step's progress, and the model result the job waits for, if any.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractProgressJs {
    /// Pages done.
    pub done: u32,
    /// Pages asked for.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub total: Option<u32>,
    /// `extract`, or `model` while a request is open.
    pub phase: &'static str,
    /// The result the job waits for: hand it to `provide`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub request: Option<ModelRequestJs>,
}

/// A model result the job waits for.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelRequestJs {
    /// `layout` (regions) or `ocr` (words).
    pub kind: &'static str,
    /// The 0-based page to render.
    pub page: u32,
    /// The resolution to render it at.
    pub dpi: u32,
    /// OCR's languages (BCP 47); empty for the engine's default.
    pub langs: Vec<String>,
}

/// A layout region, its box `[x, y, width, height]` in top-left points (ADR 0027), as the
/// output's boxes are.
#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RegionInit {
    /// What it is.
    pub label: RegionLabelName,
    /// `[x, y, width, height]`.
    pub bbox: [f64; 4],
    /// The model's confidence, 0 to 1.
    pub score: f64,
}

/// An OCR word, its box as a region's.
#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WordInit {
    /// Its text.
    pub text: String,
    /// `[x, y, width, height]`.
    pub bbox: [f64; 4],
    /// The engine's confidence, 0 to 1.
    pub confidence: f64,
}

/// What `provide` takes.
#[derive(Debug, Deserialize, Tsify)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum ModelResultInit {
    /// For a `layout` request.
    Layout {
        /// The regions found.
        regions: Vec<RegionInit>,
    },
    /// For an `ocr` request.
    Ocr {
        /// The words read.
        words: Vec<WordInit>,
    },
}

/// A region's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Tsify)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RegionLabelName {
    /// Running text.
    Text,
    /// The document's title.
    Title,
    /// A section heading.
    SectionHeader,
    /// A list item.
    ListItem,
    /// A table.
    Table,
    /// A picture.
    Picture,
    /// A caption.
    Caption,
    /// A footnote.
    Footnote,
    /// A formula.
    Formula,
    /// A running header.
    PageHeader,
    /// A running footer.
    PageFooter,
    /// Anything else.
    Other,
}

fn rect([x, y, w, h]: [f64; 4]) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

impl ModelResultInit {
    fn into_result(self) -> ModelResult {
        match self {
            Self::Layout { regions } => ModelResult::Layout(
                regions
                    .into_iter()
                    .map(|r| {
                        let label = match r.label {
                            RegionLabelName::Text => RegionLabel::Text,
                            RegionLabelName::Title => RegionLabel::Title,
                            RegionLabelName::SectionHeader => RegionLabel::SectionHeader,
                            RegionLabelName::ListItem => RegionLabel::ListItem,
                            RegionLabelName::Table => RegionLabel::Table,
                            RegionLabelName::Picture => RegionLabel::Picture,
                            RegionLabelName::Caption => RegionLabel::Caption,
                            RegionLabelName::Footnote => RegionLabel::Footnote,
                            RegionLabelName::Formula => RegionLabel::Formula,
                            RegionLabelName::PageHeader => RegionLabel::PageHeader,
                            RegionLabelName::PageFooter => RegionLabel::PageFooter,
                            RegionLabelName::Other => RegionLabel::Other,
                        };
                        LayoutRegion::new(label, rect(r.bbox), r.score)
                    })
                    .collect(),
            ),
            Self::Ocr { words } => ModelResult::Ocr(
                words
                    .into_iter()
                    .map(|w| OcrWord::new(w.text, rect(w.bbox), w.confidence))
                    .collect(),
            ),
        }
    }
}

#[wasm_bindgen]
impl WasmExtractJob {
    /// Extracts up to `budget` pages: `{ state: "progress", done, total, phase, request? }` or
    /// `{ state: "done", output }`, the output in the job's format. While `request` is set, a
    /// step does nothing until `provide` answers it.
    ///
    /// # Errors
    /// `cancelled`, `document-changed`, `job-finished`, `disposed` when the document is freed,
    /// and the reading errors.
    #[wasm_bindgen(
        unchecked_return_type = "({ state: \"progress\" } & ExtractProgressJs) | { state: \"done\"; output: string }"
    )]
    pub fn step(&mut self, budget: u32) -> JsResult<JsValue> {
        let doc = upgrade(&self.doc)?;
        let doc = doc.borrow();
        let step = self
            .job
            .step(&doc, Budget::units(budget))
            .map_err(|e| lamina_error(&e))?;
        let (state, out) = match step {
            Step::Done(extraction) => {
                let out = js_sys::Object::new();
                Reflect::set(&out, &"output".into(), &self.init.output(&doc, &extraction)?.into())?;
                ("done", out.into())
            }
            Step::Progress { done, total, phase } => (
                "progress",
                to_js(&ExtractProgressJs {
                    done,
                    total,
                    phase: phase_name(phase),
                    request: self.job.request().map(request_js),
                }),
            ),
            // A step kind added to the non-exhaustive enum: report no progress.
            _ => (
                "progress",
                to_js(&ExtractProgressJs {
                    done: 0,
                    total: None,
                    phase: "other",
                    request: None,
                }),
            ),
        };
        Reflect::set(&out, &"state".into(), &state.into())?;
        Ok(out)
    }

    /// Hands the job the result of its open request: `{ kind: "layout", regions }` or
    /// `{ kind: "ocr", words }`, boxes as `[x, y, width, height]` in top-left points.
    ///
    /// # Errors
    /// `model-result-unexpected` when nothing was asked for or `result` is the other kind;
    /// `invalid-argument` / `unknown-option` for a malformed result.
    pub fn provide(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "ModelResultInit")] result: &JsValue,
    ) -> JsResult<()> {
        let result: ModelResultInit = from_js(result, "result")?;
        self.job.provide(result.into_result()).map_err(|e| lamina_error(&e))
    }

    /// Stops the job: its next step fails with `cancelled`.
    pub fn cancel(&self) {
        self.job.cancel_token().cancel();
    }
}

fn request_js(request: &ModelRequest) -> ModelRequestJs {
    let page = |p: usize| u32::try_from(p).unwrap_or(u32::MAX);
    match request {
        ModelRequest::Layout { page: p, dpi } => ModelRequestJs {
            kind: "layout",
            page: page(*p),
            dpi: *dpi,
            langs: Vec::new(),
        },
        ModelRequest::Ocr { page: p, dpi, langs } => ModelRequestJs {
            kind: "ocr",
            page: page(*p),
            dpi: *dpi,
            langs: langs.clone(),
        },
        // A request kind added to the non-exhaustive enum: nothing the host can answer.
        _ => ModelRequestJs {
            kind: "other",
            page: page(request.page()),
            dpi: 0,
            langs: Vec::new(),
        },
    }
}
