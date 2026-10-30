//! Shared helpers for `lamina-build`'s tests: fixture loading, saving and reopening, a
//! `lamina-content` visitor that records what a page draws (in user space), small PNG inputs,
//! the golden samples (`samples`: the builder's; `legacy`: the frozen `hello` and
//! `imaging-demo`) and the spike's content builder (`content`, which `legacy` needs).
#![allow(
    dead_code,
    unreachable_pub,
    reason = "each test crate uses a different subset of the helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; pixel values are in 0..=255"
)]

pub mod b5b;
pub mod content;
pub mod legacy;
pub mod samples;
pub mod text;

use std::path::{Path, PathBuf};

use lamina_content::{ImageDraw, ImageSource, PathPaint, PathSegment, Visitor, interpret_page};
use lamina_cos::{Object, OpenOptions, Warning};
use lamina_save::{Save as _, SaveOptions, XrefKind};

/// The repository's shared `fixtures/` directory.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn fixture(rel: &str) -> Vec<u8> {
    let path = fixtures_dir().join(rel);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// Saves a built document as a full rewrite with a classic xref table (the goldens' layout).
pub fn save(mut doc: lamina_ops::Document) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new().with_xref(XrefKind::Classic))
        .unwrap();
    out
}

pub fn reopen(bytes: &[u8]) -> lamina_model::Document {
    lamina_model::Document::open(bytes.to_vec(), OpenOptions::default()).unwrap()
}

/// A painted path: its points in default user space (the CTM applied) and how it was painted.
#[derive(Debug, Clone, PartialEq)]
pub struct Painted {
    pub points: Vec<(f64, f64)>,
    pub paint: lamina_content::Paint,
    pub clip: Option<lamina_content::FillRule>,
    pub fill: Vec<f64>,
    pub fill_alpha: f64,
    pub line_width: f64,
}

/// An image drawn: its resource name and where the unit square's corners land.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn {
    pub name: String,
    /// The image dictionary.
    pub dict: lamina_cos::Dict,
    /// (0, 0) and (1, 1) of the unit square in default user space.
    pub corners: [(f64, f64); 2],
}

/// Records what a page draws.
#[derive(Debug, Default)]
pub struct Recorder {
    pending: Vec<(f64, f64)>,
    pub painted: Vec<Painted>,
    pub images: Vec<Drawn>,
    pub warnings: Vec<&'static str>,
}

fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

impl Visitor for Recorder {
    fn path_segment(&mut self, segment: &PathSegment, ctm: &lamina_content::Matrix) {
        let mut at = |x: f64, y: f64| {
            let (x, y) = ctm.transform_point(x, y);
            self.pending.push((round(x), round(y)));
        };
        match *segment {
            PathSegment::MoveTo { x, y } | PathSegment::LineTo { x, y } => at(x, y),
            PathSegment::CurveTo { c1, c2, end } => {
                at(c1.0, c1.1);
                at(c2.0, c2.1);
                at(end.0, end.1);
            }
            PathSegment::Rect { x, y, width, height } => {
                at(x, y);
                at(x + width, y + height);
            }
            PathSegment::ClosePath => {}
        }
    }

    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        self.painted.push(Painted {
            points: std::mem::take(&mut self.pending),
            paint: paint.paint,
            clip: paint.clip,
            fill: paint.state.fill.components.clone(),
            fill_alpha: paint.state.fill_alpha,
            line_width: paint.state.line_width,
        });
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        let ImageSource::XObject { name, stream, .. } = image.source else {
            panic!("the builder writes no inline images");
        };
        let Object::Stream(stream) = stream else {
            panic!("an image is a stream")
        };
        let ctm = image.state.ctm;
        let corner = |x, y| {
            let (x, y) = ctm.transform_point(x, y);
            (round(x), round(y))
        };
        self.images.push(Drawn {
            name: String::from_utf8_lossy(name.as_bytes()).into_owned(),
            dict: stream.dict.clone(),
            corners: [corner(0.0, 0.0), corner(1.0, 1.0)],
        });
    }

    fn warning(&mut self, warning: &Warning) {
        self.warnings.push(warning.code());
    }
}

/// What page `index` of `doc` draws.
pub fn record(doc: &lamina_model::Document, index: usize) -> Recorder {
    let pages = doc.pages().unwrap();
    let mut recorder = Recorder::default();
    interpret_page(doc, &pages.get(index).unwrap(), &mut recorder).unwrap();
    recorder
}

/// A PNG of `color`/`depth` with `data` (rows from the top), and optional palette and `tRNS`.
pub fn png(
    (width, height): (u32, u32),
    color: png::ColorType,
    depth: png::BitDepth,
    data: &[u8],
    palette: Option<&[u8]>,
    trns: Option<&[u8]>,
) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, width, height);
        e.set_color(color);
        e.set_depth(depth);
        if let Some(p) = palette {
            e.set_palette(p.to_vec());
        }
        if let Some(t) = trns {
            e.set_trns(t.to_vec());
        }
        let mut w = e.write_header().unwrap();
        w.write_image_data(data).unwrap();
    }
    out
}

/// A `size`×`size` RGBA gradient with a radial alpha falloff (the spike's test image).
pub fn rgba_gradient(size: u32) -> Vec<u8> {
    let mut px = Vec::new();
    let c = f64::from(size - 1) / 2.0;
    for y in 0..size {
        for x in 0..size {
            let (fx, fy) = (f64::from(x), f64::from(y));
            let d = ((fx - c).powi(2) + (fy - c).powi(2)).sqrt() / c;
            let a = (255.0 * (1.0 - d).clamp(0.0, 1.0)).round() as u8;
            px.extend_from_slice(&[(fx / c * 127.0) as u8, (fy / c * 127.0) as u8, 200, a]);
        }
    }
    px
}
