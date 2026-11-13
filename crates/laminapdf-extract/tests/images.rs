//! Image extraction over `fixtures/images/extract.pdf`, which draws one image of each kind
//! (`tools/fixtures/extract_images.py`): JPEG and JPEG 2000 kept byte for byte, the rest
//! decoded with their `/Decode`, colour space and masks, and the list checked against
//! `pdfimages -list` (`fixtures/images/extract-facts.json`).

#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use std::collections::BTreeSet;

use laminapdf_extract::{ExtractedImage, ExtractedImages, ImageFormat, ImageKind, ImageOptions, extract_images};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_syntax::Object;
use laminapdf_test_fixtures::{fixture, json};

fn open() -> Document {
    Document::open(fixture("images/extract.pdf"), OpenOptions::default()).unwrap()
}

fn extracted() -> ExtractedImages {
    extract_images(&open(), &ImageOptions::default()).unwrap()
}

/// The image of object `object` (the first entry of `kind`).
fn by_object(images: &ExtractedImages, object: u32, kind: ImageKind) -> &ExtractedImage {
    images
        .images
        .iter()
        .find(|i| i.object.is_some_and(|r| r.num == object) && i.kind == kind)
        .unwrap_or_else(|| panic!("no {kind:?} for object {object}"))
}

/// A PNG's size, channels and samples (8-bit samples, or 16-bit big-endian pairs).
fn pixels(png: &[u8]) -> (u32, u32, usize, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(png));
    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buffer).unwrap();
    buffer.truncate(info.buffer_size());
    (info.width, info.height, info.color_type.samples(), buffer)
}

/// The raw stream data of object `num`, as stored.
fn stored(doc: &Document, num: u32) -> Vec<u8> {
    let object = doc.resolve(laminapdf_syntax::ObjRef { num, gen_nr: 0 }).unwrap();
    let Object::Stream(stream) = &*object else {
        panic!("{num} isn't a stream")
    };
    stream.data.clone()
}

/// capability: B.03 core read
#[test]
fn jpegs_and_jpeg_2000_come_out_byte_for_byte() {
    let images = extracted();
    let doc = open();
    for num in [10, 11, 12] {
        let image = by_object(&images, num, ImageKind::Image);
        assert_eq!(image.format, ImageFormat::Jpeg, "{num}");
        assert_eq!(image.data, stored(&doc, num), "{num}");
    }
    let jpx = by_object(&images, 14, ImageKind::Image);
    assert_eq!(jpx.format, ImageFormat::Jpeg2000);
    assert_eq!(jpx.data, stored(&doc, 14));
    // The photo fixtures are the JPEGs the file embeds.
    assert_eq!(
        by_object(&images, 10, ImageKind::Image).data,
        fixture("images/photo-rgb.jpg")
    );
}

/// How many images `pdfimages -list`'s `rows` stand for, as extraction gives them: an image once
/// per object (pdfimages lists one each time a page draws it), each inline image, and a mask as
/// an image of its own only for a JPEG or JPEG 2000 image (the others carry theirs as
/// transparency).
fn expected_count(rows: &[serde_json::Value]) -> usize {
    let kept = |object: &serde_json::Value| {
        rows.iter().any(|r| {
            r["object"] == *object && r["type"] == "image" && matches!(r["enc"].as_str(), Some("jpeg" | "jpx"))
        })
    };
    let mut objects = BTreeSet::new();
    let mut inline = 0;
    for row in rows {
        let kind = row["type"].as_str().unwrap();
        if matches!(kind, "smask" | "mask") && !kept(&row["object"]) {
            continue;
        }
        match row["object"].as_u64() {
            Some(object) => {
                objects.insert((object, kind.to_owned()));
            }
            None => inline += 1,
        }
    }
    objects.len() + inline
}

/// capability: H.07 core read
#[test]
fn gives_as_many_images_as_pdfimages_lists_in_every_fixture() {
    let facts = json("images/extract-facts.json");
    let files = facts["files"].as_object().unwrap();
    assert!(
        files.len() > 50,
        "the facts cover the fixture, the corpus and the goldens"
    );
    let mut with_images = 0;
    for (file, rows) in files {
        let Some(rows) = rows.as_array() else { continue };
        let Ok(doc) = Document::open(fixture(file), OpenOptions::default()) else {
            continue;
        };
        let Ok(out) = extract_images(&doc, &ImageOptions::default()) else {
            continue;
        };
        // A skipped image is one pdfimages lists too.
        assert_eq!(out.images.len() + out.skipped.len(), expected_count(rows), "{file}");
        let mut seen = BTreeSet::new();
        for image in out.images.iter().filter(|i| i.object.is_some()) {
            assert!(
                seen.insert((image.object, format!("{:?}", image.kind))),
                "{file}: {:?} twice",
                image.object
            );
        }
        with_images += usize::from(!rows.is_empty());
    }
    assert!(with_images >= 10, "{with_images} fixtures with images");
}

#[test]
fn reports_each_image_as_pdfimages_describes_it() {
    let facts = json("images/extract-facts.json");
    for (file, rows) in facts["files"].as_object().unwrap() {
        let Some(rows) = rows.as_array() else { continue };
        let doc = Document::open(fixture(file), OpenOptions::default()).unwrap();
        let out = extract_images(&doc, &ImageOptions::default()).unwrap();
        for row in rows {
            let (Some(object), kind @ ("image" | "stencil")) = (row["object"].as_u64(), row["type"].as_str().unwrap())
            else {
                continue;
            };
            let kind = if kind == "image" {
                ImageKind::Image
            } else {
                ImageKind::Stencil
            };
            let Some(image) = out
                .images
                .iter()
                .find(|i| i.object.is_some_and(|r| u64::from(r.num) == object) && i.kind == kind)
            else {
                // Skipped: checked in the count above.
                continue;
            };
            let what = format!("{file} object {object}");
            assert_eq!(u64::from(image.width), row["width"].as_u64().unwrap(), "{what}");
            assert_eq!(u64::from(image.height), row["height"].as_u64().unwrap(), "{what}");
            assert_eq!(
                u64::from(image.bits_per_component),
                row["bpc"].as_u64().unwrap(),
                "{what}"
            );
            assert_eq!(image.color_space, row["color"].as_str().unwrap(), "{what}");
        }
    }
}

#[test]
fn extracts_an_image_once_on_the_first_page_that_draws_it() {
    let images = extracted();
    let flate: Vec<_> = images
        .images
        .iter()
        .filter(|i| i.object.is_some_and(|r| r.num == 15))
        .collect();
    assert_eq!(flate.len(), 1);
    assert_eq!(flate[0].page, 0);
    // Two inline images on page 1, each its own.
    assert_eq!(images.images.iter().filter(|i| i.object.is_none()).count(), 2);
    // Page 2 alone gives it, as its first page there.
    let second = extract_images(&open(), &ImageOptions::default().with_pages(vec![1])).unwrap();
    assert_eq!(second.images.len(), 1);
    assert_eq!(second.images[0].page, 1);
}

#[test]
fn finds_images_in_nested_forms_and_inline() {
    let images = extracted();
    let deep = by_object(&images, 42, ImageKind::Image);
    assert_eq!(pixels(&deep.data).3, vec![0, 255, 255, 0]);
    let inline = images
        .images
        .iter()
        .find(|i| i.object.is_none() && i.filters.len() == 1)
        .unwrap();
    assert_eq!(inline.filters, vec!["ASCIIHexDecode"]);
    assert_eq!(
        pixels(&inline.data).3,
        vec![0x00, 0x40, 0x80, 0xc0, 0xff, 0xc0, 0x80, 0x40]
    );
}

#[test]
fn decodes_every_general_filter_to_the_same_pixels() {
    let images = extracted();
    let png = |num| pixels(&by_object(&images, num, ImageKind::Image).data).3;
    // LZW and Flate hold the same gradient; ASCII85 then Flate and RunLength the same ramp.
    assert_eq!(png(24), png(15));
    assert_eq!(png(25), png(26));
    assert_eq!(png(25)[..16], (0..16).map(|v| v * 16).collect::<Vec<u8>>());
}

#[test]
fn applies_decode_indexed_cmyk_lab_and_sixteen_bits() {
    let images = extracted();
    let png = |num| pixels(&by_object(&images, num, ImageKind::Image).data);
    // /Decode [1 0] inverts the ramp.
    assert_eq!(png(19).3[..3], [255, 239, 223]);
    // Indexed: red, green, blue, white in every row.
    let (_, _, channels, idx) = png(16);
    assert_eq!(channels, 3);
    assert_eq!(idx[..12], [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]);
    // CMYK: red rows, then cyan.
    let (_, _, _, cmyk) = png(17);
    assert_eq!(cmyk[..3], [255, 0, 0]);
    assert_eq!(cmyk[cmyk.len() - 3..], [0, 255, 255]);
    // Lab at L* 100 with neutral a* and b*: white.
    assert!(png(20).3.iter().all(|&v| v >= 250));
    // Sixteen bits stay sixteen: 16 samples a row, two bytes each.
    let (width, _, _, gray16) = png(18);
    assert_eq!(gray16.len(), width as usize * 4 * 2);
    assert_eq!(gray16[2..4], [0x11, 0x11]);
}

#[test]
fn applies_masks_as_transparency() {
    let images = extracted();
    let rgba = |num| pixels(&by_object(&images, num, ImageKind::Image).data);
    // The /Mask stencil's 1 bits mask (§8.9.6.3): its left half is hidden, its right shows.
    let (width, _, channels, masked) = rgba(21);
    assert_eq!(channels, 4);
    assert_eq!(masked[3], 0);
    assert_eq!(masked[(width as usize - 1) * 4 + 3], 255);
    // The colour key hides red 0, green 0, blue 128: only the first pixel.
    let (_, _, _, keyed) = rgba(23);
    assert_eq!(keyed[3], 0);
    assert_eq!(keyed[7], 255);
    // A JPEG's soft mask is its own image.
    let smask = by_object(&images, 13, ImageKind::SoftMask);
    assert_eq!(smask.format, ImageFormat::Png);
    assert_eq!(pixels(&smask.data).3[..3], [0, 16, 32]);
}

#[test]
fn decodes_ccitt_fax_and_stencils() {
    let images = extracted();
    // Row 1 white; row 2 four white then four black (BlackIs1 false: black is 0).
    let fax = pixels(&by_object(&images, 27, ImageKind::Image).data).3;
    assert_eq!(
        fax,
        vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0]
    );
    // A stencil: its 0 bits painted, black.
    let stencil = by_object(&images, 28, ImageKind::Stencil);
    assert_eq!(pixels(&stencil.data).3[..8], [255, 0, 255, 0, 255, 0, 255, 0]);
}

#[test]
fn takes_a_soft_masks_matte_out_and_resamples_a_smaller_one() {
    let images = extracted();
    // Red premultiplied by half opacity over black comes out red, at half opacity.
    let (_, _, channels, matted) = pixels(&by_object(&images, 50, ImageKind::Image).data);
    assert_eq!(channels, 4);
    assert!(matted[0] >= 254 && matted[1] == 0 && matted[3] == 128, "{matted:?}");
    assert_eq!(matted[4..8], [255, 0, 0, 255]);
    // An 8 × 8 mask on a 16 × 16 image: each mask pixel covers two by two.
    let (width, _, _, soft) = pixels(&by_object(&images, 52, ImageKind::Image).data);
    let alpha = |x: usize, y: usize| soft[(y * width as usize + x) * 4 + 3];
    assert_eq!((alpha(0, 0), alpha(1, 1), alpha(2, 0)), (0, 0, 4));
}

#[test]
fn converts_icc_based_and_separation_and_skips_device_n() {
    let images = extracted();
    let icc = by_object(&images, 54, ImageKind::Image);
    assert_eq!(icc.color_space, "icc");
    assert_eq!(pixels(&icc.data).3[..3], [0, 16, 32]);
    // A tint is darker with more ink.
    let sep = by_object(&images, 56, ImageKind::Image);
    assert_eq!(sep.color_space, "sep");
    assert_eq!(pixels(&sep.data).3, vec![255, 191, 127, 0]);
    let skipped: Vec<_> = images.skipped.iter().filter_map(|s| s.object.map(|r| r.num)).collect();
    assert_eq!(skipped, vec![58]);
    // The reason names the colour space's family, never the file's own names.
    assert_eq!(images.skipped[0].reason, "its colour space: DeviceN");
}

#[test]
fn honours_black_is_1() {
    let images = extracted();
    let fax = pixels(&by_object(&images, 59, ImageKind::Image).data).3;
    assert_eq!(fax, vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255]);
}
