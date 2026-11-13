//! The XMP packet model (`laminapdf_model::xmp`) over the producer packets in
//! `fixtures/golden/xmp/` and the packet of every committed PDF: unchanged packets write back
//! byte for byte, a title edit keeps every other property and schema, and the edited packets
//! match their goldens.
//!
//! The goldens are never rewritten by a test run: `UPDATE_GOLDEN=1 cargo test -p
//! laminapdf-model --test xmp` writes them, for review like code.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use std::path::{Path, PathBuf};

use laminapdf_model::xmp::{MAPPINGS, MappedField, Packet, PacketState, ParseError, RdfProperty, Value};
use laminapdf_model::{Document, OpenOptions, PdfDate};
use laminapdf_test_fixtures::{classic, fixture, fixtures_dir, stream_body};

/// The producers' packets: `fixtures/golden/xmp/<name>.xmp` (provenance in fixtures/README.md).
const PRODUCERS: [&str; 5] = ["acrobat", "word", "libreoffice", "pdftex", "ghostscript"];

const NEW_TITLE: &str = "Edited title";

fn input(name: &str) -> Packet {
    Packet::parse(&fixture(&format!("golden/xmp/{name}.xmp"))).unwrap()
}

/// Compares `actual` with `fixtures/golden/xmp/<name>`, or writes it under `UPDATE_GOLDEN=1`.
fn golden(name: &str, actual: &[u8]) {
    let path = fixtures_dir().join("golden/xmp").join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read(&path).unwrap_or_default();
    assert!(
        expected == actual,
        "{name} differs from its golden: rerun with UPDATE_GOLDEN=1 and review the diff"
    );
}

/// Every property but the title, in order.
fn all_but_title(p: &Packet) -> Vec<RdfProperty> {
    p.properties()
        .filter(|q| !(q.namespace == laminapdf_model::xmp::DC && q.name == "title"))
        .cloned()
        .collect()
}

/// The namespace URIs the packet's text declares, sorted, once each.
fn declared_namespaces(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut out: Vec<String> = text
        .split("xmlns:")
        .skip(1)
        .filter_map(|rest| {
            let rest = rest.split_once('=')?.1.trim_start();
            let quote = rest.chars().next()?;
            Some(rest[1..].split(quote).next()?.to_owned())
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

#[test]
fn every_producer_packet_writes_back_unchanged() {
    for name in PRODUCERS {
        let bytes = fixture(&format!("golden/xmp/{name}.xmp"));
        assert_eq!(Packet::parse(&bytes).unwrap().write().bytes, bytes, "{name}");
    }
}

#[test]
fn changing_the_title_keeps_every_other_property_and_schema() {
    for name in PRODUCERS {
        let mut packet = input(name);
        packet.set_title(Some(NEW_TITLE));
        let written = packet.write();
        let edited = Packet::parse(&written.bytes).unwrap();
        assert_eq!(
            (
                written.canonical,
                edited.title().as_deref(),
                all_but_title(&edited),
                declared_namespaces(&written.bytes)
            ),
            (
                false,
                Some(NEW_TITLE),
                all_but_title(&packet),
                declared_namespaces(packet.original())
            ),
            "{name}"
        );
    }
}

#[test]
fn title_edits_match_their_goldens() {
    for name in PRODUCERS {
        let mut packet = input(name);
        packet.set_title(Some(NEW_TITLE));
        golden(&format!("{name}.title.xmp"), &packet.write().bytes);
    }
}

#[test]
fn an_attribute_form_edit_matches_its_golden() {
    let mut packet = input("ghostscript");
    packet.set_producer(Some("Lamina PDF"));
    packet.set_authors(&["Mary Somerville"]);
    golden("ghostscript.producer.xmp", &packet.write().bytes);
}

#[test]
fn adding_and_removing_properties_matches_its_golden() {
    let mut packet = input("libreoffice");
    packet.set_authors(&["Katherine Johnson", "Dorothy Vaughan"]);
    packet.set_subject(None);
    packet.set_keywords(Some("budget, 2027"));
    packet.set_modify_date(Some("D:20261029090000Z".parse().unwrap()));
    golden("libreoffice.fields.xmp", &packet.write().bytes);
}

#[test]
fn a_utf16_packet_is_rewritten_canonically_and_keeps_its_properties() {
    let mut packet = input("utf16");
    packet.set_title(Some(NEW_TITLE));
    let written = packet.write();
    let edited = Packet::parse(&written.bytes).unwrap();
    assert_eq!(
        (written.canonical, all_but_title(&edited)),
        (true, all_but_title(&packet))
    );
    golden("utf16.title.xmp", &written.bytes);
}

#[test]
fn a_minimal_packet_with_every_field_matches_its_golden() {
    let date: PdfDate = "D:20261028143000+01'00".parse().unwrap();
    let mut packet = Packet::new_minimal();
    for mapping in MAPPINGS {
        let value = match mapping.kind {
            laminapdf_model::xmp::ValueKind::Seq => Value::List(vec!["Ada Lovelace".into(), "Charles Babbage".into()]),
            laminapdf_model::xmp::ValueKind::Date => Value::Date(date),
            _ => Value::Text(format!("{} <&> value", mapping.name)),
        };
        packet.set(mapping.field, Some(value)).unwrap();
    }
    golden("minimal.xmp", &packet.write().bytes);
}

#[test]
fn a_packet_with_a_dtd_is_refused() {
    let error = Packet::parse(&fixture("golden/xmp/invalid-dtd.xmp")).unwrap_err();
    assert!(matches!(error, ParseError::Dtd { offset: 54 }), "{error:?}");
}

fn metadata_pdf(packet: &[u8]) -> Vec<u8> {
    let stream = stream_body("/Type /Metadata /Subtype /XML", packet);
    let bodies: [&[u8]; 3] = [
        b"<< /Type /Catalog /Pages 2 0 R /Metadata 3 0 R >>",
        b"<< /Type /Pages /Kids [] /Count 0 >>",
        &stream,
    ];
    classic(&bodies, "<< /Size 4 /Root 1 0 R >>")
}

/// capability: J.04 core read
#[test]
fn a_documents_packet_is_parsed_and_its_fields_read() {
    let doc = Document::open(fixture("golden/metadata.pdf"), OpenOptions::default()).unwrap();
    let state = doc.xmp().unwrap();
    let packet = state.packet().unwrap();
    let info = doc.info().unwrap().unwrap();
    assert_eq!(
        (
            packet.title(),
            packet.authors().join("; "),
            packet.producer(),
            packet.create_date()
        ),
        (
            info.title().unwrap(),
            info.author().unwrap().unwrap_or_default(),
            info.producer().unwrap(),
            info.creation_date().unwrap().and_then(|d| d.parsed)
        )
    );
}

#[test]
fn a_malformed_packet_raises_xmp_invalid_and_reads_as_absent() {
    let doc = Document::open(
        metadata_pdf(&fixture("golden/xmp/invalid-dtd.xmp")),
        OpenOptions::default(),
    )
    .unwrap();
    let state = doc.xmp().unwrap();
    let codes: Vec<&str> = doc.warnings().iter().map(laminapdf_model::Warning::code).collect();
    assert_eq!(
        (state.is_malformed(), state.packet().is_none(), codes),
        (true, true, vec!["xmp-invalid"])
    );
}

/// A packet with a mangled header byte-order mark (EF BB 3F) that writes `dc:title` without
/// declaring `dc:`, as PDFium's XFA test files and pdf.js's issue20453.pdf do between them.
const DAMAGED: &[u8] = b"<?xpacket begin=\"\xEF\xBB?\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?><x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\"><dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Read</rdf:li></rdf:Alt></dc:title></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";

#[test]
fn a_repaired_packet_is_read_and_raises_xmp_repaired_for_each_repair() {
    let doc = Document::open(metadata_pdf(DAMAGED), OpenOptions::default()).unwrap();
    let state = doc.xmp().unwrap();
    let reasons: Vec<&str> = doc
        .warnings()
        .iter()
        .filter_map(|w| match w {
            laminapdf_model::Warning::XmpRepaired { reason, .. } => Some(*reason),
            _ => None,
        })
        .collect();
    assert_eq!(state.packet().and_then(Packet::title).as_deref(), Some("Read"));
    assert_eq!(reasons, ["header-encoding", "undeclared-prefix"]);
    assert_eq!(doc.warnings().len(), 2);
}

#[test]
fn strict_mode_refuses_a_repaired_packet() {
    let doc = Document::open(metadata_pdf(DAMAGED), OpenOptions::default().with_strict(true)).unwrap();
    assert!(doc.xmp().is_err());
}

#[test]
fn strict_mode_refuses_a_malformed_packet() {
    let doc = Document::open(metadata_pdf(b"<rdf:RDF>"), OpenOptions::default().with_strict(true)).unwrap();
    assert!(doc.xmp().is_err());
}

#[test]
fn a_document_without_metadata_has_no_packet() {
    let doc = Document::open(fixture("corpus/hello.pdf"), OpenOptions::default()).unwrap();
    assert!(matches!(doc.xmp().unwrap(), PacketState::Absent));
}

/// Every `*.pdf` under `dir`, recursively, sorted.
fn pdfs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "pdf") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_committed_pdfs_packet_round_trips_and_survives_a_title_edit() {
    let mut parsed = 0;
    for path in pdfs(&fixtures_dir()) {
        let Ok(doc) = Document::open(std::fs::read(&path).unwrap(), OpenOptions::default()) else {
            continue;
        };
        let Ok(Some(bytes)) = doc.xmp_packet() else { continue };
        let Ok(PacketState::Parsed(mut packet)) = doc.xmp() else {
            continue;
        };
        parsed += 1;
        assert_eq!(packet.write().bytes, bytes, "{}", path.display());
        packet
            .set(MappedField::Title, Some(Value::Text(NEW_TITLE.into())))
            .unwrap();
        let edited = Packet::parse(&packet.write().bytes).unwrap();
        assert_eq!(
            (edited.title().as_deref(), all_but_title(&edited)),
            (Some(NEW_TITLE), all_but_title(&packet)),
            "{}",
            path.display()
        );
    }
    // The builder's goldens and examples, and the corpus files with a packet.
    assert!(parsed >= 15, "only {parsed} packets parsed");
}
