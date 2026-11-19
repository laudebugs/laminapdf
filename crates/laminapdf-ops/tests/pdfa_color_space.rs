//! `pdfa-color-space` (bug `pdfa-added-color-space`): on a file that claims PDF/A, an image placed
//! as content, an image stamp or a page of another PDF placed as content that paints in a device
//! space the output intent doesn't allow warns, naming its page or its stamp (ISO 19005-2
//! §6.2.4.3). Swept over intent (none, RGB, CMYK) × image (gray, RGB, CMYK, ICC) × path.
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

/// The output intents of the sweep: none, an RGB profile, a CMYK one.
const INTENTS: [Option<u8>; 3] = [None, Some(3), Some(4)];

/// The colour spaces of the sweep, with the JPEG fixture in that space (no JPEG Lamina PDF reads
/// carries an ICC space into the file, so ICC is swept only through a copied page).
const SPACES: [(&str, Option<&str>); 4] = [
    ("/DeviceGray", Some("images/photo-gray.jpg")),
    ("/DeviceRGB", Some("images/photo-rgb.jpg")),
    ("/DeviceCMYK", Some("images/photo-cmyk.jpg")),
    ("[/ICCBased 6 0 R]", None),
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
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/" pdfaid:part="2" pdfaid:conformance="B"/></rdf:RDF></x:xmpmeta>"#;
    let metadata = stream_body("/Type /Metadata /Subtype /XML", xmp.as_bytes());
    let catalog = match components {
        Some(_) => "<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R /OutputIntents [5 0 R] >>",
        None => "<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R >>",
    };
    let profile = stream_body(&format!("/N {}", components.unwrap_or(3)), b"profile");
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

/// A one-page PDF whose page draws a 1×1 image in `space`, and names `extra`'s space under its key
/// among its colour spaces when given. Object 6 is an RGB ICC profile.
fn source(space: &str, extra: Option<(&str, &str)>) -> Vec<u8> {
    let components = match space {
        "/DeviceGray" => 1,
        "/DeviceCMYK" => 4,
        _ => 3,
    };
    let image = stream_body(
        &format!("/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace {space} /BitsPerComponent 8"),
        &vec![0; components],
    );
    let content = stream_body("", b"q 10 0 0 10 0 0 cm /Im0 Do Q");
    let resources = match extra {
        Some((key, cs)) => format!("<< /XObject << /Im0 5 0 R >> /ColorSpace << /{key} {cs} >> >>"),
        None => "<< /XObject << /Im0 5 0 R >> >>".to_owned(),
    };
    let page = format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources {resources} /Contents 4 0 R >>");
    let icc = stream_body("/N 3", b"profile");
    classic(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            page.as_bytes(),
            &content,
            &image,
            &icc,
        ],
        "<< /Size 7 /Root 1 0 R >>",
    )
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
            let blob = doc.add_blob(source(space, None));
            place(&mut doc, ContentObjectSpec::Form { blob, page: 0 });
            let expected = breaks(intent, space).then(|| (Vec::new(), vec![page(&doc).to_string()]));
            assert_eq!(color_space(&doc), expected, "intent {intent:?}, image {space}");
        }
    }
}

#[test]
fn a_copied_pages_named_colour_space_counts_and_a_default_space_replaces_a_device_one() {
    let cases = [
        // An indexed space is judged by its base: CMYK under the RGB intent.
        ("/DeviceGray", ("CS0", "[/Indexed /DeviceCMYK 0 <00000000>]"), true),
        ("/DeviceGray", ("CS0", "[/Indexed [/ICCBased 6 0 R] 0 <000000>]"), false),
        // A DefaultCMYK in ICC lets the page paint DeviceCMYK, its image included, under the
        // RGB intent (ISO 32000-2 §8.6.5.6); a device DefaultCMYK doesn't.
        ("/DeviceCMYK", ("DefaultCMYK", "[/ICCBased 6 0 R]"), false),
        ("/DeviceCMYK", ("DefaultCMYK", "/DeviceCMYK"), true),
    ];
    for (image, extra, warns) in cases {
        let mut doc = pdfa(Some(3));
        let blob = doc.add_blob(source(image, Some(extra)));
        place(&mut doc, ContentObjectSpec::Form { blob, page: 0 });
        assert_eq!(color_space(&doc).is_some(), warns, "{image} with {extra:?}");
    }
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
