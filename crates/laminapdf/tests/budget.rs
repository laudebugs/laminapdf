//! Measures `SaveJob` work units per millisecond (ADR 0028 §4) for the crate README: a
//! 2,000-page document opened from bytes and saved as a full rewrite: planning (1 unit), the
//! whole parse phase in one step, then the write.
//! Release mode only, `#[ignore]`d (`cargo xtask scaling` runs it nightly); it prints and
//! asserts no timing (ADR 0018: no wall-clock asserts).
//!
//! ```sh
//! cargo test --release -p laminapdf --test budget -- --ignored --nocapture
//! ```
#![cfg(feature = "text")]
#![expect(clippy::unwrap_used, reason = "test code and helpers: a panic is a test failure")]

use std::time::Instant;

use laminapdf::{
    Budget, BuildOptions, Builder, CancelToken, Document, OpenOptions, PageSize, Phase, PointPt, Pt, SaveMode,
    SaveOptions, Standard14, Step,
};

/// A document of `pages` pages with a line of text each, saved and reopened.
fn large(pages: u32) -> Document {
    let mut b = Builder::new(BuildOptions::default().with_id([3; 16]));
    let font = b.standard_font(Standard14::Helvetica).unwrap();
    for i in 0..pages {
        let mut page = b.page(PageSize::A4);
        let _ = page
            .surface()
            .text(
                &format!("Page {i}"),
                &font,
                Pt::new(12.0),
                PointPt::from_points(72.0, 72.0),
            )
            .unwrap();
        page.finish().unwrap();
    }
    let bytes = b
        .finish()
        .unwrap()
        .save(
            &SaveOptions::new().with_mode(SaveMode::FullRewrite),
            &CancelToken::new(),
        )
        .unwrap()
        .bytes;
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

#[test]
#[ignore = "measurement for the README; release mode (cargo xtask scaling)"]
fn save_job_units_per_millisecond() {
    let mut doc = large(2_000);
    let mut job = doc.save_job(&SaveOptions::new().with_mode(SaveMode::FullRewrite));
    // One unit: planning only, which reports how many objects the parse phase has.
    let start = Instant::now();
    let Step::Progress { total: Some(total), .. } = job.step(&mut doc, Budget::units(1)).unwrap() else {
        panic!("planning a full rewrite of an opened document leaves objects to parse");
    };
    let plan_ms = ms(start);
    // Exactly the parse phase: the step ends with its budget spent, before the write.
    let start = Instant::now();
    let step = job.step(&mut doc, Budget::units(total)).unwrap();
    let parse_ms = ms(start);
    assert!(
        matches!(
            step,
            Step::Progress {
                phase: Phase::Write,
                ..
            }
        ),
        "{step:?}"
    );
    // The write and rebase: one unit.
    let start = Instant::now();
    let Step::Done(saved) = job.step(&mut doc, Budget::units(1)).unwrap() else {
        panic!("the write is the last unit");
    };
    let write_ms = ms(start);
    println!(
        "plan: {plan_ms:.2} ms; parse: {total} units in {parse_ms:.1} ms = {:.0} units/ms; \
         write + rebase: {write_ms:.1} ms for {} bytes",
        f64::from(total) / parse_ms,
        saved.bytes.len()
    );
}

fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
