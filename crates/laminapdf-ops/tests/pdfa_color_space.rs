//! `pdfa-color-space` (bug `pdfa-added-color-space`): on a file that claims PDF/A, an image placed
//! as content, an image stamp or a page of another PDF placed as content that paints in a device
//! space the output intent doesn't allow warns, naming its page or its stamp (ISO 19005-2
//! §6.2.4.3). Swept over intent (none, grey, RGB, CMYK) × image (grey, RGB, CMYK, ICC) × path,
//! then a copied page's operators, inline images, patterns, shadings, groups and nested forms.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{classic, fixture, open, stream_body};
use laminapdf_build::AnnotationAppearances;
use laminapdf_model::{Matrix, PageId, PdfDate, UserSpaceRect};
use laminapdf_ops::{
    AddAnnot, AddContentObject, Alt, AnnotSpec, ConformanceWarning, ContentId, ContentObjectSpec, Document, Op, Origin,
    Stamp, StampKind, pending_conformance_warnings,
};

/// The output intents of the sweep, by their profile's `/N`: none, grey, RGB, CMYK.
const INTENTS: [Option<u8>; 4] = [None, Some(1), Some(3), Some(4)];

/// The colour spaces of the sweep, with the JPEG fixture in that space (no JPEG Lamina PDF reads
/// carries an ICC space into the file, so ICC is swept only through a copied page).
const SPACES: [(&str, Option<&str>); 4] = [
    ("/DeviceGray", Some("images/photo-gray.jpg")),
    ("/DeviceRGB", Some("images/photo-rgb.jpg")),
    ("/DeviceCMYK", Some("images/photo-cmyk.jpg")),
    ("[/ICCBased 5 0 R]", None),
];

/// Whether `space` breaks a PDF/A file whose intent's profile has `intent` components.
fn breaks(intent: Option<u8>, space: &str) -> bool {
    match space {
        "/DeviceGray" => intent.is_none(),
        "/DeviceRGB" => intent != Some(3),
        "/DeviceCMYK" => intent != Some(4),
        _ => false,
    }
}

/// A one-page PDF/A-2b file whose `GTS_PDFA1` intent's profile has `components`, or no intent.
fn pdfa(components: Option<u8>) -> Document {
    let profile = components.map(|n| (format!("/N {n}"), b"profile".to_vec()));
    pdfa_with(2, profile)
}

/// A one-page PDF/A-`part`b file whose `GTS_PDFA1` intent's profile stream has `profile`'s
/// dictionary entries and bytes, or no intent.
fn pdfa_with(part: u8, profile: Option<(String, Vec<u8>)>) -> Document {
    let xmp = format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/" pdfaid:part="{part}" pdfaid:conformance="B"/></rdf:RDF></x:xmpmeta>"#
    );
    let metadata = stream_body("/Type /Metadata /Subtype /XML", xmp.as_bytes());
    let catalog = match profile {
        Some(_) => "<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R /OutputIntents [5 0 R] >>",
        None => "<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R >>",
    };
    let (dict, bytes) = profile.unwrap_or_else(|| ("/N 3".to_owned(), b"profile".to_vec()));
    let profile = stream_body(&dict, &bytes);
    open(&classic(
        &[
            catalog.as_bytes(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>",
            &metadata,
            b"<< /Type /OutputIntent /S /GTS_PDFA1 /OutputConditionIdentifier (test) /DestOutputProfile 6 0 R >>",
            &profile,
        ],
        "<< /Size 7 /Root 1 0 R >>",
    ))
    .with_appearances(AnnotationAppearances)
}

/// An ICC profile's first 20 bytes with `space` as its colour space field (bytes 16–19).
fn header(space: [u8; 4]) -> Vec<u8> {
    let mut bytes = vec![0; 16];
    bytes.extend_from_slice(&space);
    bytes
}

/// A one-page PDF whose page runs `content` with the resource dictionary entries `resources`.
/// Object 5 is an RGB ICC profile; `extra` are objects 6 on.
fn page_pdf(content: &[u8], resources: &str, extra: &[&[u8]]) -> Vec<u8> {
    let content = stream_body("", content);
    let page =
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << {resources} >> /Contents 4 0 R >>");
    let icc = stream_body("/N 3", b"profile");
    let mut objects: Vec<&[u8]> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page.as_bytes(),
        &content,
        &icc,
    ];
    objects.extend_from_slice(extra);
    let trailer = format!("<< /Size {} /Root 1 0 R >>", objects.len() + 1);
    classic(&objects, &trailer)
}

/// A 1×1 image XObject in `space`.
fn image_object(space: &str) -> Vec<u8> {
    let components = match space {
        "/DeviceGray" => 1,
        "/DeviceCMYK" => 4,
        _ => 3,
    };
    stream_body(
        &format!("/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace {space} /BitsPerComponent 8"),
        &vec![0; components],
    )
}

/// A one-page PDF whose page draws a 1×1 image in `space` (object 6).
fn source(space: &str) -> Vec<u8> {
    page_pdf(
        b"q 10 0 0 10 0 0 cm /Im0 Do Q",
        "/XObject << /Im0 6 0 R >>",
        &[&image_object(space)],
    )
}

/// Whether placing page 0 of `pdf` on a PDF/A-2b file with an RGB intent warns.
fn warns_on_rgb(pdf: Vec<u8>) -> bool {
    let mut doc = pdfa(Some(3));
    let blob = doc.add_blob(pdf);
    place(&mut doc, ContentObjectSpec::Form { blob, page: 0 });
    color_space(&doc).is_some()
}

fn page(doc: &Document) -> PageId {
    doc.pages().unwrap().get(0).unwrap().id().clone()
}

fn date() -> PdfDate {
    "D:20261119120000Z".parse().unwrap()
}

fn apply(doc: &mut Document, op: impl Into<Op>) -> laminapdf_ops::Applied {
    doc.apply_all(Origin::Local, [op.into()]).unwrap()
}

/// Places `object` on the first page as content.
fn place(doc: &mut Document, object: ContentObjectSpec) {
    let op = AddContentObject::new(
        ContentId::new("added").unwrap(),
        page(doc),
        object,
        Matrix::new([100.0, 0.0, 0.0, 100.0, 72.0, 72.0]),
        date(),
    )
    .with_alt(Alt::Decorative);
    apply(doc, op);
}

/// Stamps the image `bytes` on the first page; the stamp's ID.
fn stamp(doc: &mut Document, bytes: Vec<u8>) -> String {
    let blob = doc.add_blob(bytes);
    let spec = AnnotSpec::Stamp(Stamp::new(
        UserSpaceRect::from_corners(72.0, 600.0, 172.0, 700.0),
        StampKind::Image(blob),
    ));
    let page = page(doc);
    apply(doc, AddAnnot::new(page, spec))
        .created_annotations
        .pop()
        .unwrap()
        .to_string()
}

/// The `pdfa-color-space` warning's annotations and pages, if the plan lists it.
fn color_space(doc: &Document) -> Option<(Vec<String>, Vec<String>)> {
    let warnings = pending_conformance_warnings(doc, doc.pending_changes()).unwrap();
    let mut found = warnings.into_iter().filter_map(|w| match w {
        ConformanceWarning::PdfaColorSpace { annotations, pages } => Some((annotations, pages)),
        _ => None,
    });
    let first = found.next();
    assert!(found.next().is_none(), "one pdfa-color-space warning");
    first
}

#[test]
fn an_image_placed_as_content_warns_with_its_page_when_the_intent_doesnt_fit() {
    for intent in INTENTS {
        for (space, jpeg) in SPACES {
            let Some(jpeg) = jpeg else { continue };
            let mut doc = pdfa(intent);
            let blob = doc.add_blob(fixture(jpeg));
            place(&mut doc, ContentObjectSpec::Image { blob, crop: None });
            let expected = breaks(intent, space).then(|| (Vec::new(), vec![page(&doc).to_string()]));
            assert_eq!(color_space(&doc), expected, "intent {intent:?}, image {space}");
        }
    }
}

#[test]
fn an_image_stamp_warns_with_its_id_when_the_intent_doesnt_fit() {
    for intent in INTENTS {
        for (space, jpeg) in SPACES {
            let Some(jpeg) = jpeg else { continue };
            let mut doc = pdfa(intent);
            let id = stamp(&mut doc, fixture(jpeg));
            let expected = breaks(intent, space).then(|| (vec![id], Vec::new()));
            assert_eq!(color_space(&doc), expected, "intent {intent:?}, image {space}");
        }
    }
}

#[test]
fn a_copied_page_warns_with_its_page_when_its_images_dont_fit_the_intent() {
    for intent in INTENTS {
        for (space, _) in SPACES {
            let mut doc = pdfa(intent);
            let blob = doc.add_blob(source(space));
            place(&mut doc, ContentObjectSpec::Form { blob, page: 0 });
            let expected = breaks(intent, space).then(|| (Vec::new(), vec![page(&doc).to_string()]));
            assert_eq!(color_space(&doc), expected, "intent {intent:?}, image {space}");
        }
    }
}

#[test]
fn a_copied_pages_operators_count_only_where_they_paint() {
    let cases: [(&[u8], bool); 7] = [
        (b"1 0 0 rg 0 0 5 5 re f", false),
        (b"0 0 0 1 k 0 0 5 5 re f", true),
        (b"0 0 0 1 K 0 0 m 5 5 l S", true),
        (b"/DeviceCMYK cs 0 0 0 1 sc 0 0 5 5 re f", true),
        (b"0.5 g 0 0 5 5 re f", false),
        // A colour set but never painted with.
        (b"0 0 0 1 k 0 0 0 1 K 1 0 0 rg 0 0 5 5 re f", false),
        (b"0 0 0 1 k 0 0 5 5 re n", false),
    ];
    let got = cases.map(|(content, _)| warns_on_rgb(page_pdf(content, "", &[])));
    assert_eq!(got, cases.map(|(_, warns)| warns));
}

#[test]
fn a_copied_pages_text_counts_by_its_render_mode() {
    let font = "/Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >>";
    let got = [
        b"BT /F1 1 Tf 0 0 0 1 k (a) Tj ET".as_slice(),
        b"BT /F1 1 Tf 3 Tr 0 0 0 1 k (a) Tj ET",
        b"BT /F1 1 Tf 1 Tr 0 0 0 1 K (a) Tj ET",
    ]
    .map(|content| warns_on_rgb(page_pdf(content, font, &[])));
    assert_eq!(got, [true, false, true]);
}

#[test]
fn a_copied_pages_inline_image_counts_by_its_space() {
    let got = [
        b"BI /W 1 /H 1 /CS /CMYK /BPC 8 ID \x00\x00\x00\x00 EI".as_slice(),
        b"BI /W 1 /H 1 /CS /RGB /BPC 8 ID \x00\x00\x00 EI",
        b"BI /W 1 /H 1 /CS /CS0 /BPC 8 ID \x00\x00\x00\x00 EI",
        // A stencil mask paints in the fill colour.
        b"0 0 0 1 k BI /W 1 /H 1 /IM true ID \x00 EI",
    ]
    .map(|content| warns_on_rgb(page_pdf(content, "/ColorSpace << /CS0 /DeviceCMYK >>", &[])));
    assert_eq!(got, [true, false, true, true]);
}

#[test]
fn a_copied_pages_named_space_counts_when_painted_with_and_by_its_base() {
    let spaces = "/ColorSpace << /Ix [/Indexed /DeviceCMYK 0 <00000000>] /Icc [/Indexed [/ICCBased 5 0 R] 0 <000000>] \
                  /SepK [/Separation /Spot /DeviceCMYK 6 0 R] /SepRgb [/Separation /Spot /DeviceRGB 6 0 R] \
                  /DevN [/DeviceN [/Spot /Other] /DeviceCMYK 6 0 R] >>";
    let tint = b"<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 1] /N 1 >>".as_slice();
    let got = [
        b"/Ix cs 0 sc 0 0 5 5 re f".as_slice(),
        b"/Icc cs 0 sc 0 0 5 5 re f",
        b"/SepK cs 1 sc 0 0 5 5 re f",
        b"/SepRgb cs 1 sc 0 0 5 5 re f",
        b"/DevN cs 1 1 sc 0 0 5 5 re f",
        // Named, but never painted with.
        b"0 0 5 5 re f",
    ]
    .map(|content| warns_on_rgb(page_pdf(content, spaces, &[tint])));
    assert_eq!(got, [true, false, true, false, true, false]);
}

#[test]
fn a_copied_pages_patterns_and_shadings_count() {
    let shading = "<< /ShadingType 2 /ColorSpace /DeviceCMYK /Coords [0 0 1 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 1] /N 1 >> >>";
    let tiling = |image: &str| {
        let tile = stream_body(
            "/PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 1 1] /XStep 1 /YStep 1 /Resources << /XObject << /Im0 7 0 R >> >>",
            b"/Im0 Do",
        );
        (tile, image_object(image))
    };
    let (cmyk_tile, cmyk_image) = tiling("/DeviceCMYK");
    let (icc_tile, icc_image) = tiling("[/ICCBased 5 0 R]");
    let fill = b"/Pattern cs /P0 scn 0 0 5 5 re f".as_slice();
    let got = [
        warns_on_rgb(page_pdf(fill, "/Pattern << /P0 6 0 R >>", &[&cmyk_tile, &cmyk_image])),
        warns_on_rgb(page_pdf(fill, "/Pattern << /P0 6 0 R >>", &[&icc_tile, &icc_image])),
        warns_on_rgb(page_pdf(
            fill,
            &format!("/Pattern << /P0 << /PatternType 2 /Shading {shading} >> >>"),
            &[],
        )),
        warns_on_rgb(page_pdf(b"/Sh0 sh", &format!("/Shading << /Sh0 {shading} >>"), &[])),
    ];
    assert_eq!(got, [true, false, true, true]);
}

#[test]
fn a_nested_forms_paint_and_group_space_count() {
    let inner = |content: &[u8], group: &str| {
        stream_body(
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 10 10] {group} /Resources << >>"),
            content,
        )
    };
    let outer = stream_body(
        "/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << /XObject << /Fm1 7 0 R >> >>",
        b"/Fm1 Do",
    );
    let draw = b"/Fm0 Do".as_slice();
    let resources = "/XObject << /Fm0 6 0 R >>";
    let got = [
        warns_on_rgb(page_pdf(
            draw,
            resources,
            &[&outer, &inner(b"0 0 0 1 k 0 0 5 5 re f", "")],
        )),
        warns_on_rgb(page_pdf(
            draw,
            resources,
            &[&outer, &inner(b"1 0 0 rg 0 0 5 5 re f", "")],
        )),
        warns_on_rgb(page_pdf(
            draw,
            resources,
            &[&outer, &inner(b"", "/Group << /S /Transparency /CS /DeviceCMYK >>")],
        )),
    ];
    assert_eq!(got, [true, false, true]);
}

#[test]
fn an_image_mask_takes_the_fill_colour_and_a_jpx_image_its_own_file() {
    let mask = stream_body(
        "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ImageMask true",
        &[0],
    );
    let jpx = stream_body(
        "/Type /XObject /Subtype /Image /Width 1 /Height 1 /Filter /JPXDecode",
        b"jpx",
    );
    let resources = "/XObject << /Im0 6 0 R >>";
    let got = [
        warns_on_rgb(page_pdf(b"0 0 0 1 k /Im0 Do", resources, &[&mask])),
        warns_on_rgb(page_pdf(b"1 0 0 rg /Im0 Do", resources, &[&mask])),
        warns_on_rgb(page_pdf(b"/Im0 Do", resources, &[&jpx])),
    ];
    assert_eq!(got, [true, false, false]);
}

#[test]
fn a_default_space_stands_in_for_its_device_space_on_pdf_a_2_but_not_pdf_a_1() {
    let content = b"0 0 0 1 k 0 0 5 5 re f".as_slice();
    let got = [
        ("/ColorSpace << /DefaultCMYK [/ICCBased 5 0 R] >>", 2),
        ("/ColorSpace << /DefaultCMYK /DeviceCMYK >>", 2),
        ("/ColorSpace << /DefaultCMYK [/ICCBased 5 0 R] >>", 1),
    ]
    .map(|(resources, part)| {
        let mut doc = pdfa_with(part, Some(("/N 3".to_owned(), b"profile".to_vec())));
        let blob = doc.add_blob(page_pdf(content, resources, &[]));
        place(&mut doc, ContentObjectSpec::Form { blob, page: 0 });
        color_space(&doc).is_some()
    });
    assert_eq!(got, [false, true, true]);
}

#[test]
fn the_intents_kind_comes_from_its_profiles_header_then_its_n() {
    let profiles = [
        // A CMYK header outranks a wrong /N.
        ("/N 3".to_owned(), header(*b"CMYK")),
        // A Lab output profile allows only grey.
        ("/N 3".to_owned(), header(*b"Lab ")),
        // Neither a header nor /N: nothing is judged.
        (String::new(), b"profile".to_vec()),
    ];
    let got = profiles.map(|profile| {
        let mut doc = pdfa_with(2, Some(profile));
        let blob = doc.add_blob(fixture("images/photo-rgb.jpg"));
        place(&mut doc, ContentObjectSpec::Image { blob, crop: None });
        color_space(&doc).is_some()
    });
    assert_eq!(got, [true, true, false]);
}

#[test]
fn undoing_the_edit_takes_the_warning_away() {
    let mut doc = pdfa(Some(3));
    let blob = doc.add_blob(fixture("images/photo-cmyk.jpg"));
    place(&mut doc, ContentObjectSpec::Image { blob, crop: None });
    stamp(&mut doc, fixture("images/photo-cmyk.jpg"));
    let both = color_space(&doc).map(|(a, p)| (a.len(), p.len()));
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!((both, color_space(&doc)), (Some((1, 1)), None));
}
