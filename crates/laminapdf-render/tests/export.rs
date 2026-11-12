//! Page export (todo E3h) through the hayro adapter: PNG and TIFF hold exactly the pixels the
//! renderer drew, at the size the resolution gives, with that resolution in their headers; a
//! multi-page TIFF holds every page; JPEG is opaque with its density in the JFIF header; and
//! bad requests are typed errors. PDFium's pixels are laminapdf-render-pdfium's ignored tests.
#![cfg(all(feature = "export", feature = "hayro"))]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use std::io::Cursor;
use std::sync::Arc;

use laminapdf_render::{
    ByteSource, CancelToken, ExportFormat, ExportOptions, HayroBackend, RenderBackend, RenderDocument, RenderError,
    RenderOpenOptions, RenderRequest, export_pages,
};
use laminapdf_test_fixtures::fixture;

fn open(path: &str) -> Box<dyn RenderDocument> {
    let source: Arc<dyn ByteSource> = Arc::new(fixture(path));
    HayroBackend::new().open(source, &RenderOpenOptions::new()).unwrap()
}

/// What the renderer draws for `page` at `dpi` on white: width, height, RGBA.
fn drawn(doc: &dyn RenderDocument, page: u32, dpi: f64) -> (u32, u32, Vec<u8>) {
    #[expect(clippy::cast_possible_truncation, reason = "the test resolutions are exact in f32")]
    let request = RenderRequest::page(page, (dpi / 72.0) as f32);
    let plan = doc.plan(&request).unwrap();
    let mut rgba = vec![0; plan.byte_len()];
    doc.render(&request, &mut rgba, &CancelToken::new()).unwrap();
    (plan.tile.width, plan.tile.height, rgba)
}

fn export(doc: &dyn RenderDocument, pages: &[u32], options: ExportOptions) -> Result<Vec<Vec<u8>>, RenderError> {
    export_pages(doc, pages, &options, &CancelToken::new())
}

fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>, Option<png::PixelDimensions>) {
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    rgba.truncate(info.buffer_size());
    let dims = reader.info().pixel_dims;
    (info.width, info.height, rgba, dims)
}

/// Every image in a TIFF: width, height, RGBA.
fn decode_tiff(bytes: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut decoder = tiff::decoder::Decoder::new(Cursor::new(bytes)).unwrap();
    let mut images = Vec::new();
    loop {
        let (width, height) = decoder.dimensions().unwrap();
        let tiff::decoder::DecodingResult::U8(rgba) = decoder.read_image().unwrap() else {
            panic!("not 8-bit samples");
        };
        images.push((width, height, rgba));
        if !decoder.more_images() {
            return images;
        }
        decoder.next_image().unwrap();
    }
}

/// capability: H.06 core read
/// capability: H.11 core read
#[test]
fn a_png_holds_the_drawn_pixels_with_the_resolution_in_its_header() {
    let doc = open("corpus/hello.pdf");
    let files = export(
        doc.as_ref(),
        &[0],
        ExportOptions::new(ExportFormat::Png).with_dpi(144.0),
    )
    .unwrap();
    let (width, height, rgba, dims) = decode_png(&files[0]);
    assert_eq!((width, height, rgba), drawn(doc.as_ref(), 0, 144.0));
    let dims = dims.unwrap();
    assert_eq!((dims.xppu, dims.yppu, dims.unit), (5669, 5669, png::Unit::Meter));
}

#[test]
fn a_letter_page_at_150_dpi_is_exactly_1275_by_1650_pixels() {
    // 150 / 72 isn't exact in f32; rounded down, the page would lose its last column and row.
    let doc = open("corpus/hello.pdf");
    let page = doc.page(0).unwrap();
    assert_eq!((page.width, page.height), (612.0, 792.0));
    let files = export(
        doc.as_ref(),
        &[0],
        ExportOptions::new(ExportFormat::Png).with_dpi(150.0),
    )
    .unwrap();
    let (width, height, _, _) = decode_png(&files[0]);
    assert_eq!((width, height), (1275, 1650));
}

#[test]
fn a_tiff_holds_the_drawn_pixels_and_a_multi_page_tiff_every_page() {
    let doc = open("corpus/outline.pdf");
    let single = export(
        doc.as_ref(),
        &[1],
        ExportOptions::new(ExportFormat::Tiff { multi_page: false }).with_dpi(36.0),
    )
    .unwrap();
    assert_eq!(decode_tiff(&single[0]), [drawn(doc.as_ref(), 1, 36.0)]);
    let multi = export(
        doc.as_ref(),
        &[0, 1, 2],
        ExportOptions::new(ExportFormat::Tiff { multi_page: true }).with_dpi(36.0),
    )
    .unwrap();
    assert_eq!(multi.len(), 1);
    let pages: Vec<_> = (0..3).map(|i| drawn(doc.as_ref(), i, 36.0)).collect();
    assert_eq!(decode_tiff(&multi[0]), pages);
}

#[test]
fn a_jpeg_is_one_file_per_page_with_its_size_and_density_in_the_headers() {
    let doc = open("corpus/outline.pdf");
    let options = ExportOptions::new(ExportFormat::Jpeg { quality: 85 })
        .with_dpi(72.0)
        .with_background(None);
    let files = export(doc.as_ref(), &[0, 2], options).unwrap();
    assert_eq!(files.len(), 2);
    let (width, height, _) = drawn(doc.as_ref(), 0, 72.0);
    let jpeg = &files[0];
    assert!(jpeg.starts_with(&[0xFF, 0xD8]));
    // JFIF APP0: units 1 (per inch), density 72 × 72.
    let jfif = jpeg.windows(5).position(|w| w == b"JFIF\0").unwrap();
    assert_eq!(&jpeg[jfif + 7..jfif + 12], &[1, 0, 72, 0, 72]);
    // The baseline frame header holds the page's size.
    let sof = jpeg.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
    let side = |at: usize| u32::from(u16::from_be_bytes([jpeg[at], jpeg[at + 1]]));
    assert_eq!((side(sof + 7), side(sof + 5)), (width, height));
}

#[test]
fn bad_requests_are_typed_errors() {
    let doc = open("corpus/hello.pdf");
    let png = ExportOptions::new(ExportFormat::Png);
    for dpi in [0.0, -1.0, f64::NAN] {
        let err = export(doc.as_ref(), &[0], png.with_dpi(dpi)).unwrap_err();
        assert_eq!(err.code(), "invalid-argument", "{dpi}");
    }
    assert_eq!(
        export(doc.as_ref(), &[7], png).unwrap_err().code(),
        "page-index-out-of-range"
    );
    let multi = ExportOptions::new(ExportFormat::Tiff { multi_page: true });
    assert_eq!(export(doc.as_ref(), &[], multi).unwrap_err().code(), "invalid-argument");
    let cancel = CancelToken::new();
    cancel.cancel();
    let err = export_pages(doc.as_ref(), &[0], &png, &cancel).unwrap_err();
    assert_eq!(err, RenderError::Cancelled);
}
