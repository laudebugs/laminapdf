//! The inspector's reads: the claims a file makes, its fonts and colour profiles, every object
//! with its value and stream data, what refers to an object, and the revisions. Each read
//! changes nothing, and decoding a stream never goes past `Limits`.
#![expect(clippy::unwrap_used, reason = "test code: a panic is a test failure")]

use laminapdf::{CosValue, Document, ErrorCode, Limit, Limits, ObjectKind, ObjectLocation, OpenOptions, StreamView};
use laminapdf_syntax::filters::flate_encode;
use laminapdf_test_fixtures::{classic, classic_with_root, fixture, stream_body, update};

fn open(bytes: Vec<u8>) -> Document {
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

/// A page whose content stream inflates past the default `Limits::max_stream_decoded`.
fn bomb() -> Vec<u8> {
    let payload = flate_encode(&vec![0u8; Limits::default().max_stream_decoded() + 1]);
    let stream = stream_body("/Filter /FlateDecode", &payload);
    classic(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>",
            &stream,
        ],
        "<< /Size 5 /Root 1 0 R >>",
    )
}

/// capability: J.07 core read
#[test]
fn a_pdfa_file_claims_its_part_and_level_and_the_header_version() {
    let doc = open(fixture("golden/examples/pdf_a_archive-2b.pdf"));
    let claims = doc.conformance().unwrap();
    let pdfa = claims.pdfa.unwrap();
    assert_eq!((pdfa.part, pdfa.conformance.as_deref()), (2, Some("B")));
    assert_eq!(claims.header_version.as_deref(), Some("1.7"));
    assert_eq!(claims.pdfua, None);
}

#[test]
fn a_pdfx_version_in_the_info_dictionary_is_claimed() {
    let doc = open(classic_with_root(
        &[
            "<< /Type /Catalog /Version /2.0 >>",
            "<< /GTS_PDFXVersion (PDF/X-1a:2003) >>",
        ],
        "/Info 2 0 R",
    ));
    let claims = doc.conformance().unwrap();
    assert_eq!(claims.pdfx.as_deref(), Some("PDF/X-1a:2003"));
    assert_eq!(claims.catalog_version.as_deref(), Some("2.0"));
}

/// capability: J.08 core read
#[test]
fn an_output_intents_profile_gives_its_icc_header() {
    let doc = open(fixture("golden/examples/pdf_a_archive-2b.pdf"));
    let colour = doc.color_profiles().unwrap();
    let intent = colour.output_intents.first().expect("PDF/A needs an output intent");
    assert_eq!(intent.subtype.as_deref(), Some("GTS_PDFA1"));
    let profile = colour
        .profiles
        .iter()
        .find(|p| Some(p.object) == intent.profile)
        .unwrap();
    assert!(profile.output_intent);
    let header = profile.header.as_ref().unwrap();
    assert_eq!(header.colour_space, "RGB");
    assert_eq!(profile.components, Some(3));
}

/// capability: J.08 core read
#[test]
fn fonts_say_whether_they_are_embedded_and_how_much_their_tounicode_covers() {
    let doc = open(fixture("golden/examples/multilingual.pdf"));
    let fonts = doc.fonts(None).unwrap();
    let embedded = fonts.iter().find(|f| f.embedded).expect("the builder embeds its fonts");
    assert!(embedded.subset && embedded.to_unicode && embedded.object.is_some());
    let coverage = embedded.to_unicode_coverage.unwrap();
    assert_eq!(coverage.mapped, coverage.codes);
}

#[test]
fn fonts_of_a_page_past_the_last_are_refused() {
    let doc = open(fixture("corpus/hello.pdf"));
    let error = doc.fonts(Some(&[9])).unwrap_err();
    assert_eq!(error.error_code(), ErrorCode::PageIndexOutOfRange);
}

/// capability: J.09 core read
#[test]
fn every_object_is_listed_with_its_kind_type_and_place() {
    let doc = open(fixture("corpus/hello-xrefstm.pdf"));
    let objects = doc.objects().unwrap();
    let catalog = objects
        .iter()
        .find(|o| o.type_name.as_deref() == Some("Catalog"))
        .unwrap();
    assert_eq!(catalog.kind, ObjectKind::Dictionary);
    assert!(
        objects
            .iter()
            .any(|o| matches!(o.location, ObjectLocation::ObjectStream { .. })),
        "the file keeps objects in an object stream"
    );
    assert!(objects.windows(2).all(|w| w[0].id.num < w[1].id.num));
}

#[test]
fn an_object_gives_its_value_and_the_revision_that_last_wrote_it() {
    let base = classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
        ],
        "",
    );
    let file = update(
        &base,
        &[(2, b"<< /Type /Pages /Kids [] /Count 0 /Changed true >>")],
        "/Size 3 /Root 1 0 R",
    );
    let doc = open(file);
    let pages = doc.object(2).unwrap().unwrap();
    assert_eq!(pages.revision, Some(1));
    let CosValue::Dictionary(entries) = &pages.value else {
        panic!("a dictionary")
    };
    assert!(
        entries
            .iter()
            .any(|(k, v)| k == "Changed" && *v == CosValue::Boolean(true))
    );
    assert_eq!(doc.object(1).unwrap().unwrap().revision, Some(0));
    assert_eq!(doc.object(7).unwrap(), None);
}

/// capability: J.09 core read
#[test]
fn a_reference_is_found_from_where_it_is_held() {
    let doc = open(classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
        ],
        "",
    ));
    let referrers = doc.references_to(3).unwrap();
    assert_eq!(referrers.len(), 1);
    assert_eq!(
        (referrers[0].from.map(|r| r.num), referrers[0].path.as_str()),
        (Some(2), "/Kids[0]")
    );
    let root = doc.references_to(1).unwrap();
    assert_eq!((root[0].from, root[0].path.as_str()), (None, "/Root"));
}

#[test]
fn a_stream_reads_raw_and_decoded_up_to_the_bytes_asked_for() {
    let data = flate_encode(b"BT /F1 12 Tf (Hello) Tj ET");
    let doc = open(classic(
        &[b"<< /Type /Catalog >>", &stream_body("/Filter /FlateDecode", &data)],
        "<< /Size 3 /Root 1 0 R >>",
    ));
    let decoded = doc.stream_bytes(2, StreamView::Decoded, 5).unwrap().unwrap();
    assert_eq!((decoded.bytes.as_slice(), decoded.total), (&b"BT /F"[..], 26));
    let raw = doc.stream_bytes(2, StreamView::Raw, usize::MAX).unwrap().unwrap();
    assert_eq!(raw.bytes, data);
    assert_eq!(doc.stream_bytes(1, StreamView::Raw, 5).unwrap(), None);
}

/// capability: J.09 core read
#[test]
fn a_decompression_bomb_is_refused_at_the_limit_and_still_reads_raw() {
    let doc = open(bomb());
    let error = doc.stream_bytes(4, StreamView::Decoded, 1 << 20).unwrap_err();
    assert_eq!(error.error_code(), ErrorCode::LimitExceeded);
    assert_eq!(
        error.diagnostic().data.get("limit").and_then(|l| l.as_str()),
        Some(Limit::StreamDecoded.name())
    );
    let raw = doc.stream_bytes(4, StreamView::Raw, 16).unwrap().unwrap();
    assert_eq!(raw.bytes.len(), 16);
    // Listing and showing the object read its dictionary only.
    assert!(doc.objects().unwrap().iter().any(|o| o.kind == ObjectKind::Stream));
    assert!(doc.object(4).unwrap().unwrap().stream.is_some());
}

#[test]
fn an_object_is_written_as_pdf_syntax_with_a_streams_data_left_out() {
    let doc = open(bomb());
    let text = doc.object_syntax(4).unwrap().unwrap();
    assert!(text.starts_with("4 0 obj\n<<"), "{text}");
    assert!(
        text.contains("/FlateDecode") && text.ends_with("endstream\nendobj\n"),
        "{text}"
    );
}

/// capability: J.09 core read
#[test]
fn an_incremental_update_is_a_second_revision_listing_what_it_changed() {
    let doc = open(fixture("corpus/hello-incremental.pdf"));
    let revisions = doc.revisions().unwrap();
    assert_eq!(revisions.len(), 2);
    assert!(revisions[0].changed.is_empty() && !revisions[0].added.is_empty());
    assert_ne!(revisions[1].changed.len(), 0);
    assert!(revisions[0].bytes < revisions[1].bytes);
    assert!(!revisions.iter().any(|r| r.signed));
}

#[test]
fn a_signed_revision_is_marked_signed() {
    let doc = open(fixture("signed/approval.pdf"));
    let revisions = doc.revisions().unwrap();
    assert!(revisions.last().unwrap().signed);
}
