//! Objects added to a page as content leave the rest of the page as it was (todo E6b): hayro
//! draws each page before and after an image, a shape behind the content and a page of another
//! PDF are added, and every pixel outside the objects' boxes is the same.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use lamina_render::hayro_render::Doc;
use laminapdf::{
    AnnotColor, ContentId, ContentLayer, Document, Op, OpenOptions, PathSpec, PdfDate, Pt, RectPt, SaveMode,
    SaveOptions,
};

fn fixture(path: &str) -> Vec<u8> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    std::fs::read(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

/// `bytes` with a photo, a band behind the content and a wordmark on its first page, and the
/// page-space boxes they cover.
fn with_objects(bytes: Vec<u8>) -> (Vec<u8>, Vec<RectPt>, f64) {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new()).unwrap();
    let at: PdfDate = "D:20261118120000Z".parse().unwrap();
    let id = |s: &str| ContentId::new(s).unwrap();
    let photo = doc
        .image_at(
            0,
            id("photo"),
            rect(300.0, 60.0, 420.0, 140.0),
            fixture("images/photo-rgb.jpg"),
            at,
        )
        .unwrap();
    let band = PathSpec::Rect {
        rect: rect(40.0, 200.0, 250.0, 260.0),
        radius: 0.0,
    };
    let band = doc
        .shape_on(0, id("band"), band, Some(AnnotColor::Rgb(0.9, 0.3, 0.3)), None, at)
        .unwrap()
        .with_layer(ContentLayer::Behind);
    let wordmark = doc
        .page_at(
            0,
            id("wordmark"),
            rect(60.0, 400.0, 180.0, 460.0),
            fixture("content/add/logo.pdf"),
            0,
            at,
        )
        .unwrap();
    doc.apply(vec![Op::from(photo), Op::from(band), Op::from(wordmark)])
        .unwrap();
    let boxes = doc.added_objects(0).unwrap().iter().map(|o| o.bounds).collect();
    let width = doc.page_infos().unwrap()[0].width;
    let saved = doc
        .save(&SaveOptions::new().with_mode(SaveMode::Incremental))
        .unwrap()
        .bytes;
    (saved, boxes, width)
}

#[test]
fn every_pixel_outside_the_added_objects_stays_the_same() {
    for name in [
        "golden/tagged-report.pdf",
        "content/add/unbalanced.pdf",
        "content/add/rotated.pdf",
        "content/add/layers.pdf",
    ] {
        let original = fixture(name);
        let before = Doc::load(original.clone()).unwrap().render(0, 72.0).unwrap();
        let (edited, boxes, width) = with_objects(original);
        let after = Doc::load(edited).unwrap().render(0, 72.0).unwrap();
        assert_eq!((before.width, before.height), (after.width, after.height), "{name}");
        // Pixels per point of page space; a pixel's edge an object touches may blend.
        let scale = f64::from(before.width) / width;
        let near = |x: u32, y: u32| {
            let (x, y) = (f64::from(x) / scale, f64::from(y) / scale);
            boxes.iter().any(|b| {
                x >= b.x0.get() - 2.0 && x <= b.x1.get() + 2.0 && y >= b.y0.get() - 2.0 && y <= b.y1.get() + 2.0
            })
        };
        let mut changed = 0;
        for y in 0..before.height {
            for x in 0..before.width {
                let i = usize::try_from((y * before.width + x) * 3).unwrap();
                if before.data[i..i + 3] != after.data[i..i + 3] && !near(x, y) {
                    changed += 1;
                }
            }
        }
        assert_eq!(changed, 0, "{name}: pixels changed outside the added objects");
        assert_ne!(before.data, after.data, "{name}: the objects are drawn");
    }
}
