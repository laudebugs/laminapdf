//! `images_to_pdf` (todo E3h): one page per image, a JPEG embedded byte for byte, the EXIF
//! orientation applied by the drawing transform, and pages sized from the image's resolution
//! or fitted to paper.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{record, reopen, save};
use laminapdf_build::{Image, ImagePages, ImagesOptions, Orientation, PageSize, images_to_pdf};
use laminapdf_cos::Object;
use laminapdf_test_fixtures::fixture;

/// EXIF holding only IFD0's `Orientation` (big-endian TIFF).
fn exif(orientation: u8) -> Vec<u8> {
    let mut tiff = b"MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01\0".to_vec();
    tiff.extend([orientation, 0, 0, 0, 0, 0, 0]);
    tiff
}

/// A 4 × 2 RGBA PNG with `pHYs` at `dpi` and, optionally, an `eXIf` orientation.
fn png_at(dpi: f64, orientation: Option<u8>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 4, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "small test values"
    )]
    let dpm = (dpi / 0.0254).round() as u32;
    encoder.set_pixel_dims(Some(png::PixelDimensions {
        xppu: dpm,
        yppu: dpm,
        unit: png::Unit::Meter,
    }));
    let mut writer = encoder.write_header().unwrap();
    if let Some(o) = orientation {
        writer.write_chunk(png::chunk::ChunkType(*b"eXIf"), &exif(o)).unwrap();
    }
    writer.write_image_data(&[200; 32]).unwrap();
    writer.finish().unwrap();
    out
}

/// The built document, saved and reopened.
fn build(images: &[Image], options: &ImagesOptions) -> laminapdf_model::Document {
    reopen(&save(images_to_pdf(images, options).unwrap()))
}

/// A page's `/MediaBox` width and height.
fn size(doc: &laminapdf_model::Document, index: usize) -> (f64, f64) {
    let rect = doc.pages().unwrap().get(index).unwrap().media_box().unwrap().unwrap();
    (rect.width(), rect.height())
}

#[test]
fn a_jpeg_is_embedded_byte_for_byte() {
    let photo = fixture("images/photo-rgb.jpg");
    let doc = build(&[Image::jpeg(photo.clone()).unwrap()], &ImagesOptions::new());
    let drawn = &record(&doc, 0).images[0];
    assert_eq!(drawn.dict.get("Filter"), Some(&Object::name("DCTDecode")));
    let page = doc.pages().unwrap();
    let xobjects = page
        .get(0)
        .unwrap()
        .resources()
        .unwrap()
        .get("XObject")
        .unwrap()
        .clone();
    let xobjects = doc.deref(&xobjects).unwrap();
    let r = xobjects.as_dict().unwrap().iter().next().unwrap().1.as_ref().unwrap();
    let Object::Stream(stream) = &*doc.resolve(r).unwrap() else {
        panic!("the image is not a stream");
    };
    assert!(stream.data == photo, "the JPEG's bytes changed");
}

#[test]
fn every_image_gets_its_own_page_in_order() {
    let photo = Image::jpeg(fixture("images/photo-rgb.jpg")).unwrap();
    let gray = Image::jpeg(fixture("images/photo-gray.jpg")).unwrap();
    let doc = build(&[photo.clone(), gray, photo], &ImagesOptions::new());
    assert_eq!(doc.page_count().unwrap(), 3);
    assert_eq!(size(&doc, 0), size(&doc, 2));
}

/// Where EXIF's orientations 1 to 8 show the stored image's bottom-left, top-right and
/// bottom-right corners, as fractions of the shown image's width and height (y down): from what
/// each value says row 0 and column 0 become, not from our matrices.
type Corner = (f64, f64);
const CORNERS: [(Corner, Corner, Corner); 8] = [
    ((0.0, 1.0), (1.0, 0.0), (1.0, 1.0)), // 1: as stored
    ((1.0, 1.0), (0.0, 0.0), (0.0, 1.0)), // 2: mirrored left to right
    ((1.0, 0.0), (0.0, 1.0), (0.0, 0.0)), // 3: half a turn
    ((0.0, 0.0), (1.0, 1.0), (1.0, 0.0)), // 4: mirrored top to bottom
    ((1.0, 0.0), (0.0, 1.0), (1.0, 1.0)), // 5: mirrored on the main diagonal
    ((0.0, 0.0), (1.0, 1.0), (0.0, 1.0)), // 6: a quarter turn clockwise
    ((0.0, 1.0), (1.0, 0.0), (0.0, 0.0)), // 7: mirrored on the other diagonal
    ((1.0, 1.0), (0.0, 0.0), (1.0, 0.0)), // 8: a quarter turn anticlockwise
];

#[test]
fn each_orientation_shows_the_wide_photo_its_way_up_on_a_page_its_shape() {
    let wide = |o: usize| {
        let name = if o == 1 {
            "images/photo-wide.jpg".to_owned()
        } else {
            format!("images/photo-wide-exif{o}.jpg")
        };
        Image::jpeg(fixture(&name)).unwrap()
    };
    let images: Vec<Image> = (1..=8).map(wide).collect();
    assert_eq!(images[5].orientation(), Orientation::Clockwise);
    let doc = build(&images, &ImagesOptions::new());
    let (w, h) = size(&doc, 0);
    assert!(w > h, "the stored photo is wide");
    for (i, (bottom_left, top_right, bottom_right)) in CORNERS.into_iter().enumerate() {
        let (pw, ph) = size(&doc, i);
        assert_eq!((pw, ph), if i >= 4 { (h, w) } else { (w, h) }, "orientation {}", i + 1);
        // Image space (0, 0) is the stored bottom-left corner, (1, 1) the top-right and (1, 0)
        // the bottom-right; the page's user space has y up.
        let on_page = |(x, y): Corner| (x * pw, (1.0 - y) * ph);
        let drawn = &record(&doc, i).images[0];
        assert_eq!(
            drawn.corners,
            [on_page(bottom_left), on_page(top_right)],
            "orientation {}",
            i + 1
        );
        assert_eq!(drawn.bottom_right, on_page(bottom_right), "orientation {}", i + 1);
    }
}

#[test]
fn a_png_page_takes_its_size_from_phys_and_its_way_up_from_exif() {
    let doc = build(
        &[
            Image::png(&png_at(144.0, None)).unwrap(),
            Image::png(&png_at(144.0, Some(8))).unwrap(),
        ],
        &ImagesOptions::new(),
    );
    // 4 × 2 pixels at 144 dpi are 2 × 1 points, then turned. pHYs holds whole dots per
    // metre (144 dpi is 5,669.3 of them), so the sizes are a hair over.
    let close = |(w, h): (f64, f64), (ew, eh): (f64, f64)| (w - ew).abs() < 1e-3 && (h - eh).abs() < 1e-3;
    assert!(close(size(&doc, 0), (2.0, 1.0)), "{:?}", size(&doc, 0));
    assert!(close(size(&doc, 1), (1.0, 2.0)), "{:?}", size(&doc, 1));
}

#[test]
fn a_fitted_photo_sits_centred_inside_the_margins() {
    let photo = Image::jpeg(fixture("images/photo-rgb.jpg")).unwrap();
    let (iw, ih) = (f64::from(photo.width()), f64::from(photo.height()));
    let options = ImagesOptions::new()
        .with_pages(ImagePages::Fit(PageSize::A4))
        .with_margin(laminapdf_build::Pt::new(36.0));
    let doc = build(&[photo], &options);
    let (pw, ph) = size(&doc, 0);
    let [(x0, y0), (x1, y1)] = record(&doc, 0).images[0].corners;
    let (dw, dh) = (x1 - x0, y1 - y0);
    assert!((dw / dh - iw / ih).abs() < 1e-3, "the aspect ratio changed");
    assert!(x0 >= 35.99 && y0 >= 35.99 && pw - x1 >= 35.99 && ph - y1 >= 35.99);
    assert!(
        ((x0 - (pw - x1)).abs() < 0.01) && ((y0 - (ph - y1)).abs() < 0.01),
        "not centred"
    );
}

#[test]
fn a_page_past_14400_points_gets_a_user_unit_and_keeps_its_size() {
    // 30,000 × 1 pixels at 72 dpi: a page 30,000 points wide, past what readers must support.
    let long = Image::rgba(30_000, 1, &vec![90; 120_000]).unwrap();
    let doc = build(&[long], &ImagesOptions::new());
    let page = doc.pages().unwrap();
    let page = page.get(0).unwrap();
    assert_eq!(page.user_unit().unwrap(), 3.0);
    // The MediaBox is in user-space units: a third of the points, written to six places; the
    // image fills it.
    let media = page.media_box().unwrap().unwrap();
    assert_eq!(media.width(), 10_000.0);
    assert!((media.height() - 1.0 / 3.0).abs() < 1e-5);
    let [(x0, y0), (x1, y1)] = record(&doc, 0).images[0].corners;
    assert_eq!((x0, y0, x1), (0.0, 0.0, 10_000.0));
    assert!((y1 - 1.0 / 3.0).abs() < 1e-3);
}
