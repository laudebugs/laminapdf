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

/// Data that inflates past the default `Limits::max_stream_decoded`.
fn bomb_data() -> Vec<u8> {
    flate_encode(&vec![0u8; Limits::default().max_stream_decoded() + 1])
}

/// A page whose content stream (object 4) inflates past the default limit.
fn bomb() -> Vec<u8> {
    let stream = stream_body("/Filter /FlateDecode", &bomb_data());
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

/// A file whose catalog's metadata stream (object 2) is `xmp`, with `info` as its Info dictionary.
fn with_xmp(xmp: &str, info: &str) -> Document {
    let metadata = stream_body("/Type /Metadata /Subtype /XML", xmp.as_bytes());
    open(classic(
        &[b"<< /Type /Catalog /Metadata 2 0 R >>", &metadata, info.as_bytes()],
        "<< /Size 4 /Root 1 0 R /Info 3 0 R >>",
    ))
}

/// An XMP packet with one description holding `properties` (`ns:name="value"` attributes).
fn packet(properties: &str) -> String {
    format!(
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?><x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" xmlns:pdfxid="http://www.npes.org/pdfx/ns/id/" {properties}/></rdf:RDF></x:xmpmeta><?xpacket end="w"?>"#
    )
}

/// capability: J.07 core read
#[test]
fn a_pdfa_file_claims_its_part_and_level() {
    let doc = open(fixture("golden/examples/pdf_a_archive-2b.pdf"));
    let pdfa = doc.conformance().unwrap().pdfa.unwrap();
    assert_eq!((pdfa.part, pdfa.conformance.as_deref()), (2, Some("B")));
}

#[test]
fn the_header_and_catalog_versions_are_read_as_written() {
    let doc = open(classic_with_root(&["<< /Type /Catalog /Version /2.0 >>"], ""));
    let claims = doc.conformance().unwrap();
    assert_eq!(
        (claims.header_version.as_deref(), claims.catalog_version.as_deref()),
        (Some("1.7"), Some("2.0"))
    );
}

/// capability: J.07 core read
#[test]
fn a_pdfua_file_claims_its_part() {
    let doc = open(fixture("golden/examples/pdf_ua_report.pdf"));
    assert_eq!(doc.conformance().unwrap().pdfua, Some(1));
}

/// capability: J.07 core read
#[test]
fn a_pdfx_version_in_the_xmp_metadata_is_claimed() {
    let doc = with_xmp(&packet(r#"pdfxid:GTS_PDFXVersion="PDF/X-4""#), "<< >>");
    let claims = doc.conformance().unwrap();
    assert_eq!(
        (claims.pdfx.as_deref(), claims.pdfx_info_conflict),
        (Some("PDF/X-4"), None)
    );
}

#[test]
fn a_pdfx_version_in_the_info_dictionary_is_claimed() {
    let doc = open(classic_with_root(
        &["<< /Type /Catalog >>", "<< /GTS_PDFXVersion (PDF/X-1a:2003) >>"],
        "/Info 2 0 R",
    ));
    assert_eq!(doc.conformance().unwrap().pdfx.as_deref(), Some("PDF/X-1a:2003"));
}

/// capability: J.07 core read
#[test]
fn xmp_and_info_pdfx_versions_that_differ_are_both_reported() {
    let doc = with_xmp(
        &packet(r#"pdfxid:GTS_PDFXVersion="PDF/X-4""#),
        "<< /GTS_PDFXVersion (PDF/X-1a:2003) >>",
    );
    let claims = doc.conformance().unwrap();
    assert_eq!(claims.pdfx.as_deref(), Some("PDF/X-4"));
    assert_eq!(claims.pdfx_info_conflict.as_deref(), Some("PDF/X-1a:2003"));
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
    assert_eq!(
        (
            profile.header.as_ref().unwrap().color_space.as_str(),
            profile.components
        ),
        ("RGB", Some(3))
    );
}

#[test]
fn an_icc_profile_that_inflates_past_the_limit_is_listed_without_its_header() {
    let profile = stream_body("/N 3 /Filter /FlateDecode", &bomb_data());
    let doc = open(classic(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [] /Count 0 /CS [/ICCBased 3 0 R] >>",
            &profile,
        ],
        "<< /Size 4 /Root 1 0 R >>",
    ));
    let colour = doc.color_profiles().unwrap();
    assert_eq!(colour.profiles.len(), 1);
    assert_eq!(colour.profiles[0].header, None);
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

/// Two pages, each drawing in a font of its own (objects 5 and 6), and on page 2 a note whose
/// appearance (object 8) uses a third font (object 9).
fn two_fonts() -> Document {
    let appearance = stream_body(
        "/Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << /Font << /F 9 0 R >> >>",
        b"BT /F 9 Tf (A) Tj ET",
    );
    open(classic(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /Font << /F1 5 0 R >> >> >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /Font << /F2 6 0 R >> >> /Annots [7 0 R] >>",
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>",
            b"<< /Type /Annot /Subtype /Text /Rect [0 0 5 5] /AP << /N 8 0 R >> >>",
            &appearance,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman >>",
        ],
        "<< /Size 10 /Root 1 0 R >>",
    ))
}

/// capability: J.08 core read
#[test]
fn the_fonts_of_one_page_are_exactly_that_pages_fonts_and_its_annotations() {
    let names: Vec<_> = two_fonts()
        .fonts(Some(&[1]))
        .unwrap()
        .into_iter()
        .filter_map(|f| f.name)
        .collect();
    assert_eq!(names, ["Courier", "Times-Roman"]);
}

#[test]
fn properties_list_the_content_fonts_without_coverage() {
    let fonts = two_fonts().properties().unwrap().fonts;
    let names: Vec<_> = fonts.iter().filter_map(|f| f.name.as_deref()).collect();
    assert_eq!(names, ["Helvetica", "Courier"]);
}

#[test]
fn a_tounicode_map_that_inflates_past_the_limit_leaves_the_coverage_out() {
    let cmap = stream_body("/Filter /FlateDecode", &bomb_data());
    let doc = open(classic(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /Font << /F1 4 0 R >> >> >>",
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 65 /Widths [500] /ToUnicode 5 0 R >>",
            &cmap,
        ],
        "<< /Size 6 /Root 1 0 R >>",
    ));
    let font = doc.fonts(None).unwrap().remove(0);
    assert_eq!((font.to_unicode, font.to_unicode_coverage), (true, None));
}

#[test]
fn fonts_of_a_page_past_the_last_are_refused() {
    let error = open(fixture("corpus/hello.pdf")).fonts(Some(&[9])).unwrap_err();
    assert_eq!(error.error_code(), ErrorCode::PageIndexOutOfRange);
}

/// capability: J.09 core read
#[test]
fn every_object_is_listed_with_its_kind_type_and_place() {
    let objects = open(fixture("corpus/hello-xrefstm.pdf")).objects().unwrap();
    let catalog = objects
        .iter()
        .find(|o| o.type_name.as_deref() == Some("Catalog"))
        .unwrap();
    assert_eq!(catalog.kind, ObjectKind::Dictionary);
    assert!(
        objects
            .iter()
            .any(|o| matches!(o.location, ObjectLocation::ObjectStream { .. }))
    );
}

#[test]
fn objects_are_listed_by_ascending_number() {
    let objects = open(fixture("corpus/hello-xrefstm.pdf")).objects().unwrap();
    assert!(objects.windows(2).all(|w| w[0].id.num < w[1].id.num));
}

/// A file whose object 5 sits in object stream 4, whose data inflates past the limit; the
/// cross-reference stream is object 6.
fn object_stream_bomb() -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    let bodies: [(u32, Vec<u8>); 3] = [
        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
        (2, b"<< /Type /Pages /Kids [] /Count 0 >>".to_vec()),
        (4, stream_body("/Type /ObjStm /N 1 /First 4 /Filter /FlateDecode", &bomb_data())),
    ];
    for (num, body) in &bodies {
        offsets.push(out.len());
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    // Rows for objects 0–6 of /W [1 4 2]: free, 1 and 2 in the file, 3 free, 4 in the file,
    // 5 in object stream 4 at index 0, 6 (this stream) in the file.
    let xref_at = out.len();
    let row = |kind: u8, field: usize, index: u16| {
        let mut r = vec![kind];
        r.extend_from_slice(&u32::try_from(field).unwrap().to_be_bytes());
        r.extend_from_slice(&index.to_be_bytes());
        r
    };
    let rows: Vec<u8> = [
        row(0, 0, 0xffff),
        row(1, offsets.first().copied().unwrap(), 0),
        row(1, offsets.get(1).copied().unwrap(), 0),
        row(0, 0, 0),
        row(1, offsets.get(2).copied().unwrap(), 0),
        row(2, 4, 0),
        row(1, xref_at, 0),
    ]
    .concat();
    out.extend_from_slice(b"6 0 obj\n");
    out.extend_from_slice(&stream_body("/Type /XRef /Size 7 /W [1 4 2] /Root 1 0 R", &rows));
    out.extend_from_slice(format!("\nendobj\nstartxref\n{xref_at}\n%%EOF\n").as_bytes());
    out
}

/// capability: J.09 core read
#[test]
fn an_object_in_an_object_stream_past_the_limit_is_listed_as_null() {
    let objects = open(object_stream_bomb()).objects().unwrap();
    let member = objects.iter().find(|o| o.id.num == 5).unwrap();
    assert_eq!(member.kind, ObjectKind::Null);
    assert!(matches!(
        member.location,
        ObjectLocation::ObjectStream { stream: 4, index: 0 }
    ));
}

#[test]
fn an_object_gives_its_value() {
    let doc = open(classic_with_root(&["<< /Type /Catalog /Open true >>"], ""));
    let CosValue::Dictionary(entries) = doc.object(1).unwrap().unwrap().value else {
        panic!("a dictionary")
    };
    assert!(
        entries
            .iter()
            .any(|(k, v)| k == "Open" && *v == CosValue::Boolean(true))
    );
}

#[test]
fn an_object_the_document_lacks_is_none() {
    assert_eq!(
        open(classic_with_root(&["<< /Type /Catalog >>"], ""))
            .object(7)
            .unwrap(),
        None
    );
}

/// capability: J.09 core read
#[test]
fn an_object_gives_the_revision_that_last_wrote_it() {
    let base = classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
        ],
        "",
    );
    let file = update(
        &base,
        &[(2, b"<< /Type /Pages /Kids [] /Count 0 >>")],
        "/Size 3 /Root 1 0 R",
    );
    let doc = open(file);
    assert_eq!(
        (
            doc.object(1).unwrap().unwrap().revision,
            doc.object(2).unwrap().unwrap().revision
        ),
        (Some(0), Some(1))
    );
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
}

#[test]
fn the_trailer_refers_to_the_catalog() {
    let doc = open(classic_with_root(&["<< /Type /Catalog >>"], ""));
    let root = doc.references_to(1).unwrap();
    assert_eq!((root[0].from, root[0].path.as_str()), (None, "/Root"));
}

#[test]
fn the_trailer_is_the_root_of_the_object_tree() {
    let CosValue::Dictionary(entries) = open(classic_with_root(&["<< /Type /Catalog >>"], "")).trailer() else {
        panic!("a dictionary")
    };
    let root = entries.iter().find(|(k, _)| k == "Root").map(|(_, v)| v.clone());
    assert_eq!(root, Some(CosValue::Reference { num: 1, generation: 0 }));
}

/// A catalog and a Flate stream (object 2) of `content`.
fn one_stream(content: &[u8]) -> Document {
    open(classic(
        &[
            b"<< /Type /Catalog >>",
            &stream_body("/Filter /FlateDecode", &flate_encode(content)),
        ],
        "<< /Size 3 /Root 1 0 R >>",
    ))
}

#[test]
fn a_stream_decodes_up_to_the_bytes_asked_for() {
    let read = one_stream(b"BT /F1 12 Tf (Hello) Tj ET")
        .stream_bytes(2, StreamView::Decoded, 5)
        .unwrap()
        .unwrap();
    assert_eq!((read.bytes.as_slice(), read.total), (&b"BT /F"[..], None));
}

#[test]
fn a_stream_read_whole_gives_its_length() {
    let read = one_stream(b"BT ET")
        .stream_bytes(2, StreamView::Decoded, 100)
        .unwrap()
        .unwrap();
    assert_eq!((read.bytes.as_slice(), read.total), (&b"BT ET"[..], Some(5)));
}

#[test]
fn a_stream_reads_raw_as_stored() {
    let doc = one_stream(b"BT ET");
    let read = doc.stream_bytes(2, StreamView::Raw, usize::MAX).unwrap().unwrap();
    assert_eq!(read.bytes, flate_encode(b"BT ET"));
}

#[test]
fn an_object_that_isnt_a_stream_has_no_stream_bytes() {
    assert_eq!(one_stream(b"").stream_bytes(1, StreamView::Raw, 5).unwrap(), None);
}

/// capability: J.09 core read
#[test]
fn a_decompression_bomb_decodes_only_the_bytes_shown() {
    let read = open(bomb())
        .stream_bytes(4, StreamView::Decoded, 1 << 20)
        .unwrap()
        .unwrap();
    assert_eq!((read.bytes.len(), read.total), (1 << 20, None));
}

/// capability: J.09 core read
#[test]
fn a_decompression_bomb_read_whole_is_refused_at_the_limit() {
    let error = open(bomb())
        .stream_bytes(4, StreamView::Decoded, usize::MAX)
        .unwrap_err();
    assert_eq!(error.error_code(), ErrorCode::LimitExceeded);
    let limit = error
        .diagnostic()
        .data
        .get("limit")
        .and_then(|l| l.as_str().map(str::to_owned));
    assert_eq!(limit.as_deref(), Some(Limit::StreamDecoded.name()));
}

#[test]
fn a_decompression_bomb_still_reads_raw() {
    let read = open(bomb()).stream_bytes(4, StreamView::Raw, 16).unwrap().unwrap();
    assert_eq!(read.bytes.len(), 16);
}

#[test]
fn a_decompression_bomb_is_listed_and_shown_by_its_dictionary() {
    let doc = open(bomb());
    assert!(doc.objects().unwrap().iter().any(|o| o.kind == ObjectKind::Stream));
    assert!(doc.object(4).unwrap().unwrap().stream.is_some());
}

#[test]
fn an_object_is_written_as_pdf_syntax_with_a_streams_data_left_out() {
    let text = open(bomb()).object_syntax(4).unwrap().unwrap();
    assert!(text.starts_with("4 0 obj\n<<"), "{text}");
    assert!(
        text.contains("/FlateDecode") && text.ends_with("endstream\nendobj\n"),
        "{text}"
    );
}

/// capability: J.09 core read
#[test]
fn an_incremental_update_is_a_second_revision_listing_what_it_changed() {
    let revisions = open(fixture("corpus/hello-incremental.pdf")).revisions().unwrap();
    assert_eq!(revisions.len(), 2);
    assert!(revisions[0].changed.is_empty() && !revisions[0].added.is_empty());
    assert_ne!(revisions[1].changed.len(), 0);
    assert!(!revisions.iter().any(|r| r.signed));
}

#[test]
fn a_signed_revision_is_marked_signed() {
    let revisions = open(fixture("signed/approval.pdf")).revisions().unwrap();
    assert!(revisions.last().unwrap().signed);
}
