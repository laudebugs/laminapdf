//! Model plug-ins with deterministic fakes (no weights): layout regions reorder and relabel
//! blocks, OCR reads only pages with no visible text, the request/provide path gives the
//! same output as plug-ins run inside the job, and cancellation reaches the model.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use laminapdf_build::{BuildOptions, Builder, PageSize, Standard14};
use laminapdf_extract::{
    BlockKind, DropReason, ExtractError, ExtractOptions, Extraction, LayoutModel, LayoutRegion, ModelError,
    ModelInfo, ModelRequest, ModelResult, ModelSlot, Models, OcrEngine, OcrWord, PageRaster, PageRenderer,
    RegionLabel, extract, extract_job,
};
use laminapdf_model::{Document, OpenOptions, PointPt, Pt, RectPt};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_syntax::{Budget, CancelToken, Phase, Step};

/// A document with one A4 page per entry, each drawing its `(text, x, baseline)` lines in
/// 12 pt Helvetica; an empty entry is a page with no text (a scan, as far as text goes).
fn build(pages: &[&[(&str, f64, f64)]]) -> Document {
    let mut b = Builder::new(BuildOptions::default());
    let font = b.standard_font(Standard14::Helvetica).unwrap();
    for lines in pages {
        let mut page = b.page(PageSize::A4);
        let s = page.surface();
        for &(text, x, y) in *lines {
            s.text(text, &font, Pt::new(12.0), PointPt::from_points(x, y)).unwrap();
        }
        page.finish().unwrap();
    }
    let mut doc = b.finish().unwrap();
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new()).unwrap();
    Document::open(bytes, OpenOptions::default()).unwrap()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

fn info(name: &str) -> ModelInfo {
    ModelInfo::new(name, "1", "00".repeat(32), "Apache-2.0")
}

/// Two columns 7 pt apart, closer than the rules' gutter, so the rules read each row across
/// both columns.
fn two_columns() -> Document {
    build(&[&[
        ("LLL1", 72.0, 100.0),
        ("RRR1", 106.0, 100.0),
        ("LLL2", 72.0, 114.0),
        ("RRR2", 106.0, 114.0),
        ("LLL3", 72.0, 128.0),
        ("RRR3", 106.0, 128.0),
    ]])
}

/// Fixed regions on every page, counting its calls.
struct FakeLayout {
    regions: Vec<LayoutRegion>,
    calls: AtomicUsize,
}

impl FakeLayout {
    fn new(regions: Vec<LayoutRegion>) -> Arc<Self> {
        Arc::new(Self {
            regions,
            calls: AtomicUsize::new(0),
        })
    }
}

impl LayoutModel for FakeLayout {
    fn info(&self) -> ModelInfo {
        info("fake-layout")
    }

    fn detect(&self, _page: &PageRaster, _cancel: &CancelToken) -> Result<Vec<LayoutRegion>, ModelError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.regions.clone())
    }
}

/// Fixed words on every page it is asked to read, counting its calls.
struct FakeOcr {
    words: Vec<OcrWord>,
    calls: AtomicUsize,
}

impl OcrEngine for FakeOcr {
    fn info(&self) -> ModelInfo {
        info("fake-ocr")
    }

    fn recognise(&self, _page: &PageRaster, _langs: &[String], _cancel: &CancelToken) -> Result<Vec<OcrWord>, ModelError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.words.clone())
    }
}

/// A blank A4 raster at any resolution.
struct BlankRenderer;

impl PageRenderer for BlankRenderer {
    fn render(&self, _index: usize, dpi: u32, _cancel: &CancelToken) -> Result<PageRaster, ModelError> {
        let (w, h) = (595 * dpi / 72, 842 * dpi / 72);
        let pixels = vec![255; usize::try_from(w * h * 4).unwrap()];
        PageRaster::new(w, h, dpi, pixels, rect(0.0, 0.0, 595.0, 842.0))
    }
}

fn column_regions() -> Vec<LayoutRegion> {
    vec![
        LayoutRegion::new(RegionLabel::Text, rect(60.0, 80.0, 104.0, 140.0), 0.9),
        LayoutRegion::new(RegionLabel::Text, rect(104.0, 80.0, 200.0, 140.0), 0.8),
    ]
}

fn texts(out: &Extraction) -> Vec<String> {
    out.blocks.iter().map(|b| b.text.replace('\n', " ")).collect()
}

fn with_layout(model: Arc<dyn LayoutModel>) -> ExtractOptions {
    ExtractOptions::new().with_models(
        Models::new()
            .with_layout(ModelSlot::Plugin(model))
            .with_renderer(Arc::new(BlankRenderer)),
    )
}

#[test]
fn rules_alone_read_close_columns_row_by_row() {
    let out = extract(&two_columns(), &ExtractOptions::new(), &CancelToken::new()).unwrap();
    assert_eq!(texts(&out), ["LLL1 RRR1 LLL2 RRR2 LLL3 RRR3"]);
    assert_eq!(out.models, []);
}

#[test]
fn layout_regions_should_read_each_column_in_turn() {
    let model = FakeLayout::new(column_regions());
    let out = extract(&two_columns(), &with_layout(model.clone()), &CancelToken::new()).unwrap();
    assert_eq!(texts(&out), ["LLL1 LLL2 LLL3", "RRR1 RRR2 RRR3"]);
    assert_eq!(model.calls.load(Ordering::Relaxed), 1);
    assert_eq!(out.models, [info("fake-layout")]);
    assert!((out.blocks[0].confidence - 0.9).abs() < 1e-9);
}

#[test]
fn layout_labels_should_set_kinds_and_drop_page_headers() {
    let doc = build(&[&[("Running head", 72.0, 40.0), ("Results", 72.0, 100.0), ("Body text.", 72.0, 130.0)]]);
    let model = FakeLayout::new(vec![
        LayoutRegion::new(RegionLabel::PageHeader, rect(60.0, 25.0, 300.0, 45.0), 0.9),
        LayoutRegion::new(RegionLabel::SectionHeader, rect(60.0, 85.0, 300.0, 105.0), 0.9),
        LayoutRegion::new(RegionLabel::Text, rect(60.0, 115.0, 300.0, 135.0), 0.9),
    ]);
    let out = extract(&doc, &with_layout(model), &CancelToken::new()).unwrap();
    assert_eq!(texts(&out), ["Results", "Body text."]);
    assert_eq!(out.blocks[0].kind, BlockKind::Heading { level: 2 });
    assert_eq!(out.dropped[0].reason, DropReason::RunningHeader);
}

#[test]
fn ocr_should_read_only_pages_without_visible_text() {
    let doc = build(&[&[("Real text layer.", 72.0, 100.0)], &[]]);
    let engine = Arc::new(FakeOcr {
        words: vec![
            OcrWord::new("Scanned", rect(72.0, 90.0, 120.0, 102.0), 0.9),
            OcrWord::new("words", rect(124.0, 90.0, 160.0, 102.0), 0.7),
        ],
        calls: AtomicUsize::new(0),
    });
    let options = ExtractOptions::new().with_models(
        Models::new()
            .with_ocr(ModelSlot::Plugin(engine.clone()))
            .with_renderer(Arc::new(BlankRenderer)),
    );
    let out = extract(&doc, &options, &CancelToken::new()).unwrap();
    assert_eq!(engine.calls.load(Ordering::Relaxed), 1, "the page with text is not OCR'd");
    assert_eq!(texts(&out), ["Real text layer.", "Scanned words"]);
    let scanned = &out.blocks[1];
    assert_eq!(scanned.page, 1);
    assert_eq!(scanned.spans.iter().map(|s| s.ocr).collect::<Vec<_>>(), [Some(0.9), Some(0.7)]);
    assert!(out.blocks[0].spans.iter().all(|s| s.ocr.is_none()));
    assert!((scanned.confidence - 0.8).abs() < 1e-9, "the words' mean confidence");
}

#[test]
fn request_and_provide_should_give_what_the_plugin_path_gives() {
    let doc = two_columns();
    let sync = extract(&doc, &with_layout(FakeLayout::new(column_regions())), &CancelToken::new()).unwrap();
    let options = ExtractOptions::new().with_models(Models::new().with_layout(ModelSlot::Host(info("fake-layout"))));
    let mut job = extract_job(&doc, &options).unwrap();
    let mut asked = Vec::new();
    let hosted = loop {
        match job.step(&doc, Budget::UNLIMITED).unwrap() {
            Step::Done(out) => break out,
            Step::Progress { phase, .. } => {
                let request = job.request().cloned().unwrap();
                assert_eq!(phase, Phase::Model);
                // A step while the request is open does nothing.
                assert!(matches!(
                    job.step(&doc, Budget::UNLIMITED).unwrap(),
                    Step::Progress { phase: Phase::Model, .. }
                ));
                job.provide(ModelResult::Layout(column_regions())).unwrap();
                asked.push(request);
            }
            _ => panic!("a step neither done nor in progress"),
        }
    };
    assert_eq!(asked, [ModelRequest::Layout { page: 0, dpi: Models::DEFAULT_DPI }]);
    assert_eq!(hosted, sync);
}

#[test]
fn provide_should_refuse_a_result_nobody_asked_for() {
    let doc = two_columns();
    let options = ExtractOptions::new().with_models(Models::new().with_layout(ModelSlot::Host(info("fake-layout"))));
    let mut job = extract_job(&doc, &options).unwrap();
    assert_eq!(
        job.provide(ModelResult::Layout(Vec::new())),
        Err(ExtractError::ModelResultUnexpected)
    );
    job.step(&doc, Budget::UNLIMITED).unwrap();
    assert_eq!(job.provide(ModelResult::Ocr(Vec::new())), Err(ExtractError::ModelResultUnexpected));
    assert_eq!(job.provide(ModelResult::Layout(Vec::new())), Ok(()));
}

/// Cancels the job's token from inside the model, as a user pressing stop mid-model would.
struct CancellingLayout;

impl LayoutModel for CancellingLayout {
    fn info(&self) -> ModelInfo {
        info("cancelling")
    }

    fn detect(&self, _page: &PageRaster, cancel: &CancelToken) -> Result<Vec<LayoutRegion>, ModelError> {
        cancel.cancel();
        Err(ModelError::Cancelled)
    }
}

#[test]
fn cancelling_during_a_model_should_end_the_job_cancelled() {
    let cancel = CancelToken::new();
    let result = extract(&two_columns(), &with_layout(Arc::new(CancellingLayout)), &cancel);
    assert_eq!(result, Err(ExtractError::Cancelled));
    assert!(cancel.is_cancelled(), "the model got the caller's token");
}

/// Fails, with a message that names no document text.
struct FailingLayout;

impl LayoutModel for FailingLayout {
    fn info(&self) -> ModelInfo {
        info("failing")
    }

    fn detect(&self, _page: &PageRaster, _cancel: &CancelToken) -> Result<Vec<LayoutRegion>, ModelError> {
        Err(ModelError::Failed {
            reason: "weights missing".to_owned(),
        })
    }
}

#[test]
fn a_failing_model_should_end_the_job_with_model_failed() {
    let err = extract(&two_columns(), &with_layout(Arc::new(FailingLayout)), &CancelToken::new()).unwrap_err();
    assert_eq!(err.code(), "model-failed");
    assert_eq!(err.to_string(), "a model failed: weights missing");
}

#[test]
fn the_json_should_name_the_models_used() {
    use laminapdf_extract::{JsonOptions, MarkdownOptions, to_json};
    let doc = two_columns();
    let json_of = |options: &ExtractOptions| {
        let out = extract(&doc, options, &CancelToken::new()).unwrap();
        let markdown = out.to_markdown(&MarkdownOptions::default());
        let json: serde_json::Value = serde_json::from_str(&to_json(&out, &markdown, &JsonOptions::default())).unwrap();
        json["generator"].clone()
    };
    assert_eq!(json_of(&ExtractOptions::new())["mode"], "rules");
    let generator = json_of(&with_layout(FakeLayout::new(column_regions())));
    assert_eq!(generator["mode"], "rules+models");
    assert_eq!(
        generator["models"],
        serde_json::json!([{ "name": "fake-layout", "version": "1", "sha256": "00".repeat(32), "licence": "Apache-2.0" }])
    );
}
