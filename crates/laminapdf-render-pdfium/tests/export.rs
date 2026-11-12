//! Page export through PDFium (todo E3h): a page exported as PNG at the goldens' 96 dpi matches
//! its committed PDFium golden, and at 150 dpi holds exactly the pixels PDFium draws. Ignored:
//! needs the PDFium library (`cargo xtask render-golden` runs it).
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

mod common;

use std::io::Cursor;

use common::{Bitmap, golden_dir, golden_name, open, pdfium, render};
use laminapdf_render::{CancelToken, ExportFormat, ExportOptions, RenderOpenOptions, RenderRequest, export_pages};
use laminapdf_test_fixtures::fixture;
use laminapdf_test_fixtures::image::{read_png, ssim};

fn decode(png_bytes: &[u8]) -> Bitmap {
    let mut reader = png::Decoder::new(Cursor::new(png_bytes)).read_info().unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    rgba.truncate(info.buffer_size());
    Bitmap {
        width: info.width,
        height: info.height,
        rgba,
    }
}

/// Every corpus file PDFium renders, its first page exported as PNG at `dpi`.
fn exported(dpi: f32) -> Vec<(String, Bitmap)> {
    let skip = common::skipped("pdfium");
    common::corpus()
        .into_iter()
        .filter(|file| !skip.contains_key(file))
        .map(|file| {
            let doc = open(&pdfium(), fixture(&format!("corpus/{file}")), &RenderOpenOptions::new()).unwrap();
            let options = ExportOptions::new(ExportFormat::Png).with_dpi(dpi);
            let files = export_pages(doc.as_ref(), &[0], &options, &CancelToken::new()).unwrap();
            (file, decode(files.first().unwrap()))
        })
        .collect()
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn a_page_exported_at_96_dpi_matches_its_pdfium_golden() {
    let pages = exported(common::DPI);
    assert!(!pages.is_empty());
    for (file, page) in pages {
        let golden = read_png(&golden_dir("pdfium").join(golden_name(&file, 0)));
        assert_eq!((page.width, page.height), (golden.width, golden.height), "{file}");
        let score = ssim(&page, &golden);
        assert!(score >= 0.99, "{file}: SSIM {score:.4}");
    }
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn a_page_exported_at_150_dpi_holds_exactly_the_pixels_pdfium_draws() {
    // The scale export_pages uses: 150 / 72 as an f32, nudged up so no pixel is lost.
    let exact = 150.0f64 / 72.0;
    #[expect(clippy::cast_possible_truncation, reason = "the nearest f32, nudged up below")]
    let scale = exact as f32;
    let scale = if f64::from(scale) < exact {
        scale.next_up()
    } else {
        scale
    };
    for (file, page) in exported(150.0) {
        let doc = open(&pdfium(), fixture(&format!("corpus/{file}")), &RenderOpenOptions::new()).unwrap();
        let drawn = render(doc.as_ref(), &RenderRequest::page(0, scale)).unwrap();
        assert_eq!((page.width, page.height), (drawn.width, drawn.height), "{file}");
        assert!(
            page.rgba == drawn.rgba,
            "{file}: the exported pixels differ from PDFium's"
        );
    }
}
