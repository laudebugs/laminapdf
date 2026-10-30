//! Measures `SaveJob` work units per millisecond (ADR 0028 §4) for the crate README: a
//! 2,000-page document opened from bytes and saved as a full rewrite in steps of 256 units.
//! Release mode only, `#[ignore]`d (`cargo xtask scaling` runs it nightly); it prints and
//! asserts no timing (ADR 0018: no wall-clock asserts).
//!
//! ```sh
//! cargo test --release -p lamina --test budget -- --ignored --nocapture
//! ```
#![cfg(feature = "text")]

use std::time::Instant;

use lamina::{
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
            .text(&format!("Page {i}"), &font, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
            .unwrap();
        page.finish().unwrap();
    }
    let bytes = b
        .finish()
        .unwrap()
        .save(&SaveOptions::new().with_mode(SaveMode::FullRewrite), &CancelToken::new())
        .unwrap()
        .bytes;
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

#[test]
#[ignore = "measurement for the README; release mode (cargo xtask scaling)"]
fn save_job_units_per_millisecond() {
    let mut doc = large(2_000);
    let mut job = doc.save_job(&SaveOptions::new().with_mode(SaveMode::FullRewrite));
    let (mut parse_units, mut parse_time, mut write_time) = (0_u32, 0.0_f64, 0.0_f64);
    let mut done_before = 0;
    loop {
        let start = Instant::now();
        let step = job.step(&mut doc, Budget::units(256)).unwrap();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        match step {
            Step::Progress {
                done,
                phase: Phase::Parse,
                ..
            } => {
                parse_units += done - done_before;
                done_before = done;
                parse_time += ms;
            }
            Step::Done(saved) => {
                write_time += ms;
                println!(
                    "parse: {parse_units} units in {parse_time:.1} ms = {:.0} units/ms; write + rebase: {write_time:.1} ms for {} bytes",
                    f64::from(parse_units) / parse_time,
                    saved.bytes.len()
                );
                break;
            }
            _ => write_time += ms,
        }
    }
}
