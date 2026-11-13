//! Image extraction (todo E3i) over fixtures/images/extract.pdf, which draws one image of each
//! kind (tools/fixtures/extract_images.py): JPEG and JPEG 2000 kept byte for byte, the rest
//! decoded with their `/Decode`, colour space and masks, and the list checked against
//! `pdfimages -list` (fixtures/images/extract-facts.json).

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
    let Object::Stream(stream) = &*object else { panic!("{num} isn't a stream") };
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
    assert_eq!(by_object(&images, 10, ImageKind::Image).data, fixture("images/photo-rgb.jpg"));
}

/// capability: H.07 core read
#[test]
fn lists_the_images_pdfimages_lists() {
    let images = extracted();
    let facts = json("images/extract-facts.json");
    // pdfimages lists an image each time a page draws it, and every mask; extraction gives an
    // image once, on its first page, and a mask as an image of its own only for a JPEG or
    // JPEG 2000 image, the other images carrying theirs as transparency.
    let mut theirs = BTreeSet::new();
    let mut decoded_masks = 0;
    for row in facts["rows"].as_array().unwrap() {
        let object = row["object"].as_u64();
        let kind = row["type"].as_str().unwrap();
        let parent_is_kept = matches!(row["enc"].as_str(), Some("jpeg" | "jpx"))
            || facts["rows"].as_array().unwrap().iter().any(|r| {
                r["object"] == row["object"] && r["type"] == "image" && matches!(r["enc"].as_str(), Some("jpeg" | "jpx"))
            });
        match (kind, object) {
            ("smask" | "mask", _) if !parent_is_kept => decoded_masks += 1,
            (_, Some(object)) => {
                theirs.insert((object, kind.to_owned()));
            }
            (_, None) => {
                theirs.insert((u64::MAX, format!("inline {}", row["page"])));
            }
        }
    }
    // pdfimages names a mask by the image it masks; extraction by the mask's own object.
    let doc = open();
    let parent_of = |mask: u32| -> u32 {
        images
            .images
            .iter()
            .filter_map(|i| i.object.filter(|_| i.kind == ImageKind::Image))
            .find(|r| {
                let object = doc.resolve(*r).unwrap();
                let dict = object.as_dict().unwrap();
                ["SMask", "Mask"].iter().any(|k| dict.get(k).and_then(Object::as_ref).is_some_and(|m| m.num == mask))
            })
            .map_or(mask, |r| r.num)
    };
    let ours: BTreeSet<(u64, String)> = images
        .images
        .iter()
        .map(|i| match i.object {
            Some(r) => (
                u64::from(if matches!(i.kind, ImageKind::SoftMask | ImageKind::Mask) { parent_of(r.num) } else { r.num }),
                match i.kind {
                    ImageKind::Stencil => "stencil",
                    ImageKind::SoftMask => "smask",
                    ImageKind::Mask => "mask",
                    _ => "image",
                }
                .to_owned(),
            ),
            None => (u64::MAX, format!("inline {}", i.page)),
        })
        .collect();
    assert_eq!(ours, theirs);
    assert_eq!(decoded_masks, 1, "the RGB image's /Mask stencil, applied as its transparency");
    assert_eq!(images.skipped, vec![]);
}

#[test]
fn reports_each_image_as_pdfimages_describes_it() {
    let images = extracted();
    let facts = json("images/extract-facts.json");
    for row in facts["rows"].as_array().unwrap() {
        let (Some(object), "image") = (row["object"].as_u64(), row["type"].as_str().unwrap()) else {
            continue;
        };
        let image = by_object(&images, u32::try_from(object).unwrap(), ImageKind::Image);
        assert_eq!(u64::from(image.width), row["width"].as_u64().unwrap(), "{object}");
        assert_eq!(u64::from(image.height), row["height"].as_u64().unwrap(), "{object}");
        assert_eq!(u64::from(image.bits_per_component), row["bpc"].as_u64().unwrap(), "{object}");
        assert_eq!(image.color_space, row["color"].as_str().unwrap(), "{object}");
    }
}

#[test]
fn extracts_an_image_once_on_the_first_page_that_draws_it() {
    let images = extracted();
    let flate: Vec<_> = images.images.iter().filter(|i| i.object.is_some_and(|r| r.num == 15)).collect();
    assert_eq!(flate.len(), 1);
    assert_eq!(flate[0].page, 0);
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
    let inline = images.images.iter().find(|i| i.object.is_none()).unwrap();
    assert_eq!(inline.filters, vec!["ASCIIHexDecode"]);
    assert_eq!(pixels(&inline.data).3, vec![0x00, 0x40, 0x80, 0xc0, 0xff, 0xc0, 0x80, 0x40]);
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
    assert_eq!(fax, vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0]);
    // A stencil: its 0 bits painted, black.
    let stencil = by_object(&images, 28, ImageKind::Stencil);
    assert_eq!(pixels(&stencil.data).3[..8], [255, 0, 255, 0, 255, 0, 255, 0]);
}
