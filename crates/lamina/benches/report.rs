//! Release-mode generation benchmarks for the nightly lane (ADR 0018 tier 11; todo B5d): the
//! 100-page text report of pdf-library §4 (≤ 500 ms after `init()`), built natively through the
//! façade so the WASM timings of `cargo xtask pdf-runtimes` can be split into core cost and
//! binding cost.
//!
//! - `report_100_pages`: the Rust twin of `packages/sdk/examples/report.ts` (B6c), the report
//!   every runtime builds: Noto Sans and Noto Sans SC, a heading, a paragraph box, a line of
//!   Chinese, a 20-row table of text runs and right-aligned boxes with hairlines, a footer. Its
//!   bytes are checked against the TS suite's `REPORT_SHA256` before timing, so both measure the
//!   same document.
//! - `report_100_pages_text_boxes`: B6b's first measurement: a Helvetica-Bold heading and six
//!   4-sentence Noto Sans text boxes per page (line breaking dominates).
//!
//! `cargo xtask bench` runs them with the other benches and keeps the history; a regression
//! over 10% opens an issue rather than failing the build.
//!
//! ```sh
//! cargo bench -p lamina --bench report
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    missing_docs,
    reason = "a benchmark harness: failing loudly (here or in the compile-time hash lookup, \
              which can't use the non-const `get`) is the right behaviour; criterion's macros \
              generate undocumented items"
)]

use std::fmt::Write as _;
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use lamina::{
    Alignment, BuildOptions, Builder, CancelToken, Color, Metadata, PageSize, PointPt, Pt, RectPt, SaveOptions,
    Standard14, StrokeStyle, TextBoxOptions,
};
use sha2::{Digest, Sha256};

const NOTO_SANS: &[u8] = include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf");
const NOTO_SANS_SC: &[u8] = include_bytes!("../../../fixtures/fonts/NotoSansSC-Regular.otf");

/// `REPORT_SHA256` in `packages/sdk/test/suite/examples.ts` (the bytes every runtime
/// produces), read from that file when the bench compiles, so there is no copy to go stale; a
/// change to how the file declares it fails the build (clippy builds benches on every PR).
const REPORT_SHA256: &str = suite_hash(include_str!("../../../packages/sdk/test/suite/examples.ts"));

/// The 64 hex digits after `export const REPORT_SHA256 = "` in `suite`.
const fn suite_hash(suite: &str) -> &str {
    const PREFIX: &[u8] = b"export const REPORT_SHA256 = \"";
    let bytes = suite.as_bytes();
    let mut at = 0;
    while at + PREFIX.len() + 64 <= bytes.len() {
        let mut k = 0;
        while k < PREFIX.len() && bytes[at + k] == PREFIX[k] {
            k += 1;
        }
        if k == PREFIX.len() {
            let (_, rest) = bytes.split_at(at + k);
            let (hash, _) = rest.split_at(64);
            let mut d = 0;
            while d < 64 {
                assert!(hash[d].is_ascii_hexdigit(), "REPORT_SHA256 is not 64 hex digits");
                d += 1;
            }
            return match core::str::from_utf8(hash) {
                Ok(hash) => hash,
                Err(_) => panic!("REPORT_SHA256 is not 64 hex digits"),
            };
        }
        at += 1;
    }
    panic!("packages/sdk/test/suite/examples.ts no longer declares `export const REPORT_SHA256 = \"…\"`")
}

const PAGES: u32 = 100;

const PARAGRAPH: &str = "Quarterly figures for every region follow. Each table lists the units shipped, the revenue \
                         and the change against the previous quarter; totals are at the end of the report.";

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

fn options(id: [u8; 16]) -> BuildOptions {
    BuildOptions::default()
        .with_id(id)
        .with_producer("lamina examples")
        .with_creation_date("D:20261030120000Z".parse().unwrap())
}

/// `packages/sdk/examples/report.ts`, call for call.
fn report() -> Vec<u8> {
    let at = PointPt::from_points;
    let mut b = Builder::new(options(*b"lamina-ex-report"));
    let font = b.font_bytes(NOTO_SANS.to_vec()).unwrap();
    let chinese = b.font_bytes(NOTO_SANS_SC.to_vec()).unwrap();
    let end = TextBoxOptions::default().with_align(Alignment::End);
    let hairline = StrokeStyle {
        width: Pt::new(0.5),
        ..StrokeStyle::default()
    };
    for n in 1..=PAGES {
        let mut page = b.page(PageSize::A4);
        let s = page.surface();
        let _ = s
            .text(
                &format!("Region {n}: quarterly report"),
                &font,
                Pt::new(20.0),
                at(56.0, 72.0),
            )
            .unwrap();
        s.text_box(
            PARAGRAPH,
            &font,
            Pt::new(11.0),
            rect(56.0, 88.0, 483.0, 60.0),
            &TextBoxOptions::default(),
        )
        .unwrap();
        let _ = s
            .text(
                "你好世界，这是中文字体子集测试。",
                &chinese,
                Pt::new(11.0),
                at(56.0, 166.0),
            )
            .unwrap();
        for row in 0..20_u32 {
            let y = 200.0 + 24.0 * f64::from(row);
            let units = (n * 37 + row * 11) % 500;
            let _ = s
                .text(&format!("Product {}", row + 1), &font, Pt::new(10.0), at(56.0, y))
                .unwrap();
            s.text_box(
                &units.to_string(),
                &font,
                Pt::new(10.0),
                rect(300.0, y - 11.0, 60.0, 14.0),
                &end,
            )
            .unwrap();
            let euros = f64::from(units) * 12.5;
            s.text_box(
                &format!("€ {euros:.2}"),
                &font,
                Pt::new(10.0),
                rect(400.0, y - 11.0, 139.0, 14.0),
                &end,
            )
            .unwrap();
            s.stroke_color(Color::gray(0.8))
                .path(|p| {
                    p.move_to(at(56.0, y + 7.0)).line_to(at(539.0, y + 7.0));
                })
                .stroke(&hairline);
        }
        let _ = s
            .text(&format!("Page {n} of {PAGES}"), &font, Pt::new(9.0), at(56.0, 800.0))
            .unwrap();
        page.finish().unwrap();
    }
    b.metadata(
        Metadata::default()
            .with_title("Quarterly report")
            .with_author("lamina examples"),
    )
    .lang("en-GB");
    let mut doc = b.finish().unwrap();
    doc.save(&SaveOptions::new(), &CancelToken::new()).unwrap().bytes
}

const SENTENCES: &str = "The quarter closed with steady growth across every region we track. Shipments rose in \
                         the north while the south held level against a strong previous year. Margins improved \
                         as freight costs eased and the new warehouse came fully online. The outlook for the \
                         next quarter remains cautious but positive.";

/// B6b's shape: a Helvetica-Bold heading and six 4-sentence Noto Sans text boxes per page.
fn text_boxes() -> Vec<u8> {
    let mut b = Builder::new(options(*b"lamina-bench-b6b"));
    let bold = b.standard_font(Standard14::HelveticaBold).unwrap();
    let font = b.font_bytes(NOTO_SANS.to_vec()).unwrap();
    let options = TextBoxOptions::default();
    for n in 1..=PAGES {
        let mut page = b.page(PageSize::A4);
        let s = page.surface();
        let _ = s
            .text(
                &format!("Section {n}"),
                &bold,
                Pt::new(18.0),
                PointPt::from_points(56.0, 72.0),
            )
            .unwrap();
        for k in 0..6_u32 {
            let y = 96.0 + 110.0 * f64::from(k);
            s.text_box(SENTENCES, &font, Pt::new(11.0), rect(56.0, y, 483.0, 100.0), &options)
                .unwrap();
        }
        page.finish().unwrap();
    }
    let mut doc = b.finish().unwrap();
    doc.save(&SaveOptions::new(), &CancelToken::new()).unwrap().bytes
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn bench_reports(c: &mut Criterion) {
    let bytes = report();
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        REPORT_SHA256,
        "the Rust twin of examples/report.ts no longer builds the TS suite's bytes"
    );
    let mut group = c.benchmark_group("generate");
    group.sample_size(10);
    group.bench_function("report_100_pages", |b| b.iter(|| black_box(report())));
    group.bench_function("report_100_pages_text_boxes", |b| b.iter(|| black_box(text_boxes())));
    group.finish();
}

criterion_group!(benches, bench_reports);
criterion_main!(benches);
