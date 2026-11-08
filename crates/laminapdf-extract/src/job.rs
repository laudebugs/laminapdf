//! [`ExtractJob`]: an extraction as a resumable job ([ADR 0028] §3): one page per work unit, a
//! [`CancelToken`] checked before each page and inside the XY-Cut recursion.
//!
//! [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md

use std::sync::Arc;

use laminapdf_content::{ImageDraw, Matrix, PathPaint, PathSegment, ShadingPaint, Visitor};
use laminapdf_model::Document;
use laminapdf_syntax::{Budget, CancelToken, Phase, Step};
use laminapdf_text::{PageText, TextExtractor};

use crate::error::ExtractError;
use crate::model::{ExtractedSource, Extraction, StructSource};
use crate::options::{ExtractOptions, ReadingOrder};
use crate::page::extract_page;
use crate::page::{PageInput, TreeUse, has_visible_text};
use crate::plugins::{LayoutRegion, ModelRequest, ModelResult, ModelSlot, OcrWord, PageRaster};
use crate::tables::{Collector, Edges, TableMode};
use crate::tagged::TaggedIndex;
use crate::visibility::Backdrops;
use crate::{furniture, headings, lists};

/// An extraction in progress. Step it with the document it was made from until it returns
/// [`Step::Done`]; an error or a cancellation ends it with no partial result. The core document
/// has no revision to compare, so the job trusts the caller to pass that document; the façade's
/// job checks it (`document-changed`, ADR 0028 §3).
#[derive(Debug)]
pub struct ExtractJob {
    options: ExtractOptions,
    cancel: CancelToken,
    /// The page indices to extract, in order.
    todo: Vec<usize>,
    /// How many of `todo` are done.
    next: usize,
    extractor: TextExtractor,
    out: Extraction,
    finished: bool,
    /// The structure tree, when the reading order may come from it.
    tree: Option<TaggedIndex>,
    /// Pages the tree ordered, and pages whose tree failed a check.
    tree_pages: (usize, usize),
    /// The page read but not yet extracted, while models' results are gathered for it.
    pending: Option<Pending>,
    /// The result the job waits for from the host.
    request: Option<ModelRequest>,
}

/// A page whose text is read, waiting for its models' results.
#[derive(Debug)]
struct Pending {
    index: usize,
    text: PageText,
    edges: Option<Edges>,
    backdrops: Backdrops,
    /// Whether the page has visible text; OCR reads only pages that don't.
    visible: bool,
    regions: Option<Vec<LayoutRegion>>,
    words: Option<Vec<OcrWord>>,
    /// The page rendered for the plug-in models, kept while both use it.
    raster: Option<PageRaster>,
}

/// Starts an extraction of `doc`: checks the pages asked for, reads whether the document is
/// tagged and, when the reading order may come from it, its structure tree; the pages are read
/// by [`ExtractJob::step`].
///
/// # Errors
/// `page-index-out-of-range` for a page past the last; reading errors (the page tree, the
/// catalog).
pub fn extract_job(doc: &Document, options: &ExtractOptions) -> Result<ExtractJob, ExtractError> {
    let count = doc.pages()?.len();
    let mut todo = options.pages.clone().unwrap_or_else(|| (0..count).collect());
    // A page asked for twice is extracted once, where it first appears, so block IDs stay unique.
    let mut seen = std::collections::BTreeSet::new();
    todo.retain(|&i| seen.insert(i));
    if let Some(&index) = todo.iter().find(|&&i| i >= count) {
        return Err(ExtractError::PageIndexOutOfRange { index, count });
    }
    let struct_source = match options.reading_order {
        ReadingOrder::ContentStream => StructSource::Content,
        ReadingOrder::Auto | ReadingOrder::Tagged | ReadingOrder::Layout => StructSource::XyCut,
    };
    // The tree is read once, here; each page checks its own part of it.
    let tree = match options.reading_order {
        ReadingOrder::Tagged => TaggedIndex::read(doc)?,
        ReadingOrder::Auto if doc.is_tagged()? => TaggedIndex::read(doc)?,
        ReadingOrder::Auto | ReadingOrder::Layout | ReadingOrder::ContentStream => None,
    };
    let source = ExtractedSource {
        sha256: None,
        page_count: count,
        tagged: doc.is_tagged()?,
        struct_source,
    };
    Ok(ExtractJob {
        options: options.clone(),
        cancel: CancelToken::new(),
        todo,
        next: 0,
        extractor: TextExtractor::new(),
        out: Extraction {
            source,
            ..Extraction::default()
        },
        finished: false,
        tree,
        tree_pages: (0, 0),
        pending: None,
        request: None,
    })
}

/// Extracts `doc` in one call, stopped by `cancel` (ADR 0028 F10).
///
/// # Errors
/// As [`extract_job`] and [`ExtractJob::step`].
pub fn extract(doc: &Document, options: &ExtractOptions, cancel: &CancelToken) -> Result<Extraction, ExtractError> {
    let mut job = extract_job(doc, options)?;
    job.cancel = cancel.clone();
    loop {
        if let Step::Done(extraction) = job.step(doc, Budget::UNLIMITED)? {
            return Ok(extraction);
        }
    }
}

/// The models used, as `generator.models` names them.
fn record_models(out: &mut Extraction, options: &ExtractOptions) {
    out.models = options.models.infos();
}

/// A page's painting, seen once: the table rules (when tables are looked for) and the backdrops.
struct Painted<'f> {
    rules: Option<Collector<'f>>,
    backdrops: Backdrops,
}

impl Visitor for Painted<'_> {
    fn path_segment(&mut self, segment: &PathSegment, ctm: &Matrix) {
        if let Some(rules) = &mut self.rules {
            rules.path_segment(segment, ctm);
        }
    }

    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        if let Some(rules) = &mut self.rules {
            rules.paint_path(paint);
        }
        self.backdrops.paint_path(paint);
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        self.backdrops.image(image);
    }

    fn paint_shading(&mut self, shading: &ShadingPaint<'_>) {
        self.backdrops.paint_shading(shading);
    }
}

/// The rules that need every page: running headers and footers leave the blocks, then
/// headings (whose body size is the document's) and list items are marked, and each page's
/// blocks are numbered again.
fn finish(out: &mut Extraction, options: &ExtractOptions) {
    if options.drop.furniture {
        furniture::remove(out, options.drop.keep_spans);
    }
    headings::assign(&mut out.blocks);
    lists::assign(&mut out.blocks);
    for page in out.blocks.chunk_by_mut(|a, b| a.page == b.page) {
        for (n, block) in page.iter_mut().enumerate() {
            block.id = format!("p{}-b{n}", block.page);
        }
    }
}

impl ExtractJob {
    /// A token that cancels this job from another thread or callback.
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// The model result the job waits for, when its models are the host's
    /// ([`ModelSlot::Host`], or a plug-in with no renderer): render the page at the
    /// resolution asked, run the model, and hand the result to [`ExtractJob::provide`]. While a
    /// request is open, [`ExtractJob::step`] returns the same progress and does nothing else.
    #[must_use]
    pub const fn request(&self) -> Option<&ModelRequest> {
        self.request.as_ref()
    }

    /// Hands the job the result of its open [`ExtractJob::request`]; the next step goes on.
    ///
    /// # Errors
    /// `model-result-unexpected` when no request is open or `result` is the other model's;
    /// `job-finished` after the job ended. Neither ends the job.
    pub fn provide(&mut self, result: ModelResult) -> Result<(), ExtractError> {
        if self.finished {
            return Err(ExtractError::JobFinished);
        }
        let (Some(request), Some(pending)) = (&self.request, &mut self.pending) else {
            return Err(ExtractError::ModelResultUnexpected);
        };
        match (request, result) {
            (ModelRequest::Layout { .. }, ModelResult::Layout(regions)) => pending.regions = Some(regions),
            (ModelRequest::Ocr { .. }, ModelResult::Ocr(words)) => pending.words = Some(words),
            _ => return Err(ExtractError::ModelResultUnexpected),
        }
        self.request = None;
        Ok(())
    }

    /// Extracts up to `budget` pages (at least one), then returns the progress (pages done of
    /// the pages asked for) or, once every page is done, the extraction. Each step walks the
    /// page tree once, so step a long document with budgets of many pages, not one.
    ///
    /// A page with models to run takes them in turn: layout first, then OCR on a page with no
    /// visible text. A plug-in model with a renderer runs inside the step; any other result is
    /// asked of the host, and the step returns [`Phase::Model`] progress with
    /// [`ExtractJob::request`] set.
    ///
    /// # Errors
    /// `cancelled` once the token is cancelled; `job-finished` after the job returned its
    /// result or an error; `model-failed` when a plug-in failed; reading errors
    /// (`data-pending`, `strict`, …). Every error ends the job.
    pub fn step(&mut self, doc: &Document, budget: Budget) -> Result<Step<Extraction>, ExtractError> {
        if self.finished {
            return Err(ExtractError::JobFinished);
        }
        let result = self.run(doc, budget);
        if !matches!(result, Ok(Step::Progress { .. })) {
            self.finished = true;
        }
        result
    }

    fn progress(&self, phase: Phase) -> Step<Extraction> {
        Step::Progress {
            done: u32::try_from(self.next).unwrap_or(u32::MAX),
            total: Some(u32::try_from(self.todo.len()).unwrap_or(u32::MAX)),
            phase,
        }
    }

    fn run(&mut self, doc: &Document, budget: Budget) -> Result<Step<Extraction>, ExtractError> {
        if self.request.is_some() {
            return Ok(self.progress(Phase::Model));
        }
        let pages = doc.pages()?;
        let count = pages.len();
        for _ in 0..budget.get().max(1) {
            let Some(&index) = self.todo.get(self.next) else {
                break;
            };
            if self.cancel.is_cancelled() {
                return Err(ExtractError::Cancelled);
            }
            let page = pages
                .get(index)
                .ok_or(ExtractError::PageIndexOutOfRange { index, count })?;
            if self.pending.is_none() {
                self.pending = Some(self.read(doc, &page, index)?);
            }
            if let Some(request) = self.next_request() {
                if !self.run_plugin(&request)? {
                    self.request = Some(request);
                    return Ok(self.progress(Phase::Model));
                }
                // The plug-in answered: look for the next request on the same page.
                continue;
            }
            let Some(pending) = self.pending.take() else {
                continue;
            };
            let out = extract_page(
                doc,
                &page,
                index,
                &pending.text,
                &PageInput {
                    edges: pending.edges.as_ref(),
                    backdrops: &pending.backdrops,
                    tree: self.tree.as_ref(),
                    regions: pending.regions.as_deref().unwrap_or_default(),
                    words: pending.words.as_deref().unwrap_or_default(),
                },
                &self.options,
                &self.cancel,
            )?;
            match out.tree {
                TreeUse::Used => self.tree_pages.0 += 1,
                TreeUse::Ignored => self.tree_pages.1 += 1,
                TreeUse::None => {}
            }
            self.out.pages.push(out.info);
            self.out.blocks.extend(out.blocks);
            self.out.dropped.extend(out.dropped);
            self.next += 1;
        }
        if self.next >= self.todo.len() {
            let mut out = std::mem::take(&mut self.out);
            // Tagged only when the tree gave every page it covers its order.
            if let (true, (used, 0)) = (out.source.struct_source == StructSource::XyCut, self.tree_pages)
                && used > 0
            {
                out.source.struct_source = StructSource::Tagged;
            }
            record_models(&mut out, &self.options);
            finish(&mut out, &self.options);
            return Ok(Step::Done(out));
        }
        Ok(self.progress(Phase::Extract))
    }

    /// Reads page `index`'s text and, in the same walk of its content, its table rules and
    /// backdrops.
    fn read(&mut self, doc: &Document, page: &laminapdf_model::Page<'_>, index: usize) -> Result<Pending, ExtractError> {
        let frame = page.frame()?;
        let mut painted = Painted {
            rules: (self.options.tables != TableMode::Off).then(|| Collector::new(&frame)),
            backdrops: Backdrops::new(&frame),
        };
        let text = self.extractor.page_with(doc, page, &mut painted)?;
        let edges = painted.rules.map(Collector::finish);
        let visible = has_visible_text(&text, &painted.backdrops, &self.options);
        Ok(Pending {
            index,
            text,
            edges,
            backdrops: painted.backdrops,
            visible,
            regions: None,
            words: None,
            raster: None,
        })
    }

    /// The pending page's next missing model result, if any.
    fn next_request(&self) -> Option<ModelRequest> {
        let pending = self.pending.as_ref()?;
        let models = &self.options.models;
        let (page, dpi) = (pending.index, models.dpi());
        if models.layout.is_some() && pending.regions.is_none() {
            return Some(ModelRequest::Layout { page, dpi });
        }
        if models.ocr.is_some() && !pending.visible && pending.words.is_none() {
            return Some(ModelRequest::Ocr {
                page,
                dpi,
                langs: models.langs.clone(),
            });
        }
        None
    }

    /// Runs `request`'s model when it is a plug-in with a renderer, storing its result;
    /// `false` when the host must answer it instead.
    fn run_plugin(&mut self, request: &ModelRequest) -> Result<bool, ExtractError> {
        let models = &self.options.models;
        let Some(renderer) = models.renderer.clone() else {
            return Ok(false);
        };
        let layout = match &models.layout {
            Some(ModelSlot::Plugin(model)) => Some(Arc::clone(model)),
            Some(ModelSlot::Host(_)) | None => None,
        };
        let ocr = match &models.ocr {
            Some(ModelSlot::Plugin(engine)) => Some(Arc::clone(engine)),
            Some(ModelSlot::Host(_)) | None => None,
        };
        let cancel = self.cancel.clone();
        let result = match (request, layout, ocr) {
            (ModelRequest::Layout { .. }, Some(model), _) => {
                ModelResult::Layout(model.detect(self.raster(renderer.as_ref(), request)?, &cancel)?)
            }
            (ModelRequest::Ocr { langs, .. }, _, Some(engine)) => {
                ModelResult::Ocr(engine.recognise(self.raster(renderer.as_ref(), request)?, langs, &cancel)?)
            }
            _ => return Ok(false),
        };
        if cancel.is_cancelled() {
            return Err(ExtractError::Cancelled);
        }
        self.request = Some(request.clone());
        self.provide(result)?;
        Ok(true)
    }

    /// The pending page rendered for `request`, once per page.
    fn raster(
        &mut self,
        renderer: &dyn crate::plugins::PageRenderer,
        request: &ModelRequest,
    ) -> Result<&PageRaster, ExtractError> {
        let pending = self.pending.as_mut().ok_or(ExtractError::ModelResultUnexpected)?;
        if pending.raster.is_none() {
            let dpi = match request {
                ModelRequest::Layout { dpi, .. } | ModelRequest::Ocr { dpi, .. } => *dpi,
            };
            pending.raster = Some(renderer.render(pending.index, dpi, &self.cancel)?);
        }
        pending.raster.as_ref().ok_or(ExtractError::ModelResultUnexpected)
    }
}
