//! Images written and read back: `/SMask` for alpha, JPEG bytes unchanged, the Adobe CMYK
//! inversion, 16-bit samples, palettes, placement and one object per distinct image.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, png, record, reopen, rgba_gradient, save};
use laminapdf_build::{BuildOptions, Builder, Image, PageSize, PngOptions};
use laminapdf_cos::{Dict, Object, Stream};
use laminapdf_model::{Pt, RectPt};

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

/// A Letter page drawing `images` side by side, saved and reopened.
fn drawn(images: &[Image]) -> laminapdf_model::Document {
    let mut b = Builder::new(BuildOptions::default());
    let mut page = b.page(PageSize::LETTER);
    for (i, image) in images.iter().enumerate() {
        let x = 10.0 + 60.0 * f64::from(u32::try_from(i).unwrap());
        page.surface().image(image, rect(x, 10.0, 50.0, 40.0));
    }
    page.finish().unwrap();
    reopen(&save(b.finish().unwrap()))
}

fn stream(doc: &laminapdf_model::Document, value: &Object) -> Stream {
    let r = value.as_ref().unwrap();
    match &*doc.resolve(r).unwrap() {
        Object::Stream(s) => s.clone(),
        other => panic!("not a stream: {other:?}"),
    }
}

fn only_image(image: Image) -> (laminapdf_model::Document, Dict) {
    let doc = drawn(&[image]);
    let dict = record(&doc, 0).images[0].dict.clone();
    (doc, dict)
}

#[test]
fn a_png_with_alpha_gets_an_smask_of_the_same_size() {
    let bytes = png(
        (8, 8),
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &rgba_gradient(8),
        None,
        None,
    );
    let (doc, dict) = only_image(Image::png(&bytes).unwrap());
    let smask = stream(&doc, dict.get("SMask").unwrap());
    assert_eq!(
        (
            dict.get_name("ColorSpace"),
            smask.dict.get_name("ColorSpace"),
            smask.dict.get_int("Width"),
            smask.dict.get_int("Height"),
        ),
        (Some(&b"DeviceRGB"[..]), Some(&b"DeviceGray"[..]), Some(8), Some(8))
    );
}

#[test]
fn an_opaque_png_has_no_smask() {
    let bytes = png(
        (2, 1),
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &[1, 2, 3, 255, 4, 5, 6, 255],
        None,
        None,
    );
    let (_, dict) = only_image(Image::png(&bytes).unwrap());
    assert!(dict.get("SMask").is_none());
}

#[test]
fn jpeg_bytes_are_embedded_unchanged() {
    let bytes = fixture("images/photo-rgb.jpg");
    let doc = drawn(&[Image::jpeg(bytes.clone()).unwrap()]);
    let pages = doc.pages().unwrap();
    let res = pages.get(0).unwrap().resources().unwrap();
    let image = stream(&doc, res.get("XObject").unwrap().as_dict().unwrap().get("Im1").unwrap());
    assert_eq!(
        (
            image.dict.get_name("Filter"),
            image.data == bytes,
            image.dict.get_int("Width")
        ),
        (Some(&b"DCTDecode"[..]), true, Some(192))
    );
}

#[test]
fn an_adobe_cmyk_jpeg_is_device_cmyk_with_an_inverted_decode() {
    let (_, dict) = only_image(Image::jpeg(fixture("images/photo-cmyk.jpg")).unwrap());
    let decode: Vec<i64> = dict
        .get("Decode")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o.as_int().unwrap())
        .collect();
    assert_eq!(
        (dict.get_name("ColorSpace"), decode),
        (Some(&b"DeviceCMYK"[..]), vec![1, 0, 1, 0, 1, 0, 1, 0])
    );
}

#[test]
fn a_gray_jpeg_is_device_gray() {
    let (_, dict) = only_image(Image::jpeg(fixture("images/photo-gray.jpg")).unwrap());
    assert_eq!(dict.get_name("ColorSpace"), Some(&b"DeviceGray"[..]));
}

#[test]
fn sixteen_bit_pngs_stay_16_bit_unless_down_converted() {
    let gray = png(
        (2, 2),
        png::ColorType::Grayscale,
        png::BitDepth::Sixteen,
        &[0, 1, 2, 3, 4, 5, 6, 7],
        None,
        None,
    );
    let kept = only_image(Image::png(&gray).unwrap()).1;
    let down = only_image(Image::png_with(&gray, PngOptions::default().with_downconvert_16(true)).unwrap()).1;
    assert_eq!(
        (kept.get_int("BitsPerComponent"), down.get_int("BitsPerComponent")),
        (Some(16), Some(8))
    );
}

#[test]
fn a_palette_png_with_trns_becomes_rgb_with_an_smask() {
    let bytes = png(
        (2, 1),
        png::ColorType::Indexed,
        png::BitDepth::Eight,
        &[0, 1],
        Some(&[255, 0, 0, 0, 0, 255]),
        Some(&[0]),
    );
    let image = Image::png(&bytes).unwrap();
    let (_, dict) = only_image(image);
    assert_eq!(
        (dict.get_name("ColorSpace"), dict.contains_key("SMask")),
        (Some(&b"DeviceRGB"[..]), true)
    );
}

#[test]
fn an_image_fills_its_rect_upright() {
    let doc = drawn(&[Image::rgba(1, 1, &[0, 0, 0, 255]).unwrap()]);
    // Page space [10 10 60 50] on Letter: the unit square's origin is the rect's bottom-left.
    assert_eq!(record(&doc, 0).images[0].corners, [(10.0, 742.0), (60.0, 782.0)]);
}

#[test]
fn the_same_image_is_written_once_and_named_once() {
    let a = Image::jpeg(fixture("images/photo-rgb.jpg")).unwrap();
    let b = Image::jpeg(fixture("images/photo-rgb.jpg")).unwrap();
    let c = Image::rgba(1, 1, &[1, 2, 3, 255]).unwrap();
    let doc = drawn(&[a, b, c]);
    let names: Vec<String> = record(&doc, 0).images.iter().map(|d| d.name.clone()).collect();
    let pages = doc.pages().unwrap();
    let xobjects = pages
        .get(0)
        .unwrap()
        .resources()
        .unwrap()
        .get("XObject")
        .unwrap()
        .as_dict()
        .unwrap()
        .len();
    assert_eq!(
        (names, xobjects),
        (vec!["Im1".to_owned(), "Im1".to_owned(), "Im2".to_owned()], 2)
    );
}

#[test]
fn a_png_passed_to_jpeg_is_invalid() {
    let bytes = png(
        (1, 1),
        png::ColorType::Grayscale,
        png::BitDepth::Eight,
        &[0],
        None,
        None,
    );
    assert_eq!(Image::jpeg(bytes).unwrap_err().code(), "invalid-image");
}

#[test]
fn a_truncated_png_is_invalid() {
    let bytes = png((4, 4), png::ColorType::Rgb, png::BitDepth::Eight, &[7; 48], None, None);
    assert_eq!(
        Image::png(&bytes[..bytes.len() - 20]).unwrap_err().code(),
        "invalid-image"
    );
}

#[test]
fn a_png_larger_than_the_limit_is_refused_before_decoding() {
    // An IHDR claiming 60000 x 60000 RGBA (14.4 GB), followed by nothing useful.
    let mut bytes = png((1, 1), png::ColorType::Rgba, png::BitDepth::Eight, &[0; 4], None, None);
    bytes[16..20].copy_from_slice(&60000u32.to_be_bytes());
    bytes[20..24].copy_from_slice(&60000u32.to_be_bytes());
    let crc = crc(&bytes[12..29]);
    bytes[29..33].copy_from_slice(&crc.to_be_bytes());
    let code = Image::png(&bytes).unwrap_err().code();
    assert_eq!(code, "image-too-large");
}

/// CRC-32 (ISO 3309), as PNG chunks use.
fn crc(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}
