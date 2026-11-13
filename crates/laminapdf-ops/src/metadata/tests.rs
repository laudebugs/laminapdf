//! Unit tests of the metadata writer and reader: the mapping both ways, the patch semantics,
//! the read precedence, the PDF 2.0 rule and each conformance warning.
#![expect(
    deprecated,
    reason = "these tests check that SetInfo, deprecated but still on the wire, keeps working"
)]

use laminapdf_cos::OpenOptions;
use laminapdf_model::xmp::PacketState;
use laminapdf_test_fixtures::{classic, stream_body};

use super::*;
use crate::Error;

const NS: &str = r#"xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:pdf="http://ns.adobe.com/pdf/1.3/" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/""#;

/// A packet with `body` inside one description.
pub(super) fn packet(body: &str) -> String {
    format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?><x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" {NS}>{body}</rdf:Description></rdf:RDF></x:xmpmeta>{}<?xpacket end=\"w\"?>",
        " ".repeat(200)
    )
}

/// A one-page-tree document: `catalog_extra` in the catalog, `info` as object 3 (when given)
/// and `xmp` as object 4 (when given).
pub(super) fn doc(catalog_extra: &str, info: Option<&str>, xmp: Option<&[u8]>) -> laminapdf_model::Document {
    let metadata = if xmp.is_some() { "/Metadata 4 0 R" } else { "" };
    let catalog = format!("<< /Type /Catalog /Pages 2 0 R {metadata} {catalog_extra} >>");
    let info_body = info.unwrap_or("<< >>");
    let xmp_body = stream_body("/Type /Metadata /Subtype /XML", xmp.unwrap_or_default());
    let bodies: [&[u8]; 4] = [
        catalog.as_bytes(),
        b"<< /Type /Pages /Kids [] /Count 0 >>",
        info_body.as_bytes(),
        &xmp_body,
    ];
    let trailer = if info.is_some() {
        "<< /Size 5 /Root 1 0 R /Info 3 0 R >>"
    } else {
        "<< /Size 5 /Root 1 0 R >>"
    };
    laminapdf_model::Document::open(classic(&bodies, trailer), OpenOptions::default()).unwrap()
}

fn date(s: &str) -> PdfDate {
    s.parse().unwrap()
}

fn set(doc: &mut laminapdf_model::Document, patch: MetadataPatch) -> Effect {
    SetMetadata::new(patch, None).apply_to(doc).unwrap()
}

pub(super) fn xmp(doc: &laminapdf_model::Document) -> laminapdf_model::xmp::Packet {
    match doc.xmp().unwrap() {
        PacketState::Parsed(p) => p,
        other => panic!("no packet: {other:?}"),
    }
}

pub(super) fn info_text(doc: &laminapdf_model::Document, key: &str) -> Option<String> {
    doc.info().unwrap().and_then(|i| i.text(key).unwrap())
}

mod mapping {
    use super::*;

    #[test]
    fn a_patch_writes_every_field_to_both_sides() {
        let mut d = doc("", Some("<< >>"), None);
        let created = date("D:20260101120000+01'00'");
        let modified = date("D:20261110093000Z");
        set(
            &mut d,
            MetadataPatch::new()
                .title(Some("Figures"))
                .authors(&["Ada Lovelace", "Charles Babbage"])
                .subject(Some("Q3"))
                .keywords(Some("a, b"))
                .creator(Some("Writer"))
                .producer(Some("Lamina PDF"))
                .created(Some(created))
                .modified(Some(modified)),
        );
        let p = xmp(&d);
        assert_eq!(
            (
                p.title(),
                p.authors(),
                p.subject(),
                p.keywords(),
                p.creator_tool(),
                p.producer(),
                p.create_date(),
                p.modify_date()
            ),
            (
                Some("Figures".to_owned()),
                vec!["Ada Lovelace".to_owned(), "Charles Babbage".to_owned()],
                Some("Q3".to_owned()),
                Some("a, b".to_owned()),
                Some("Writer".to_owned()),
                Some("Lamina PDF".to_owned()),
                Some(created),
                Some(modified)
            )
        );
        assert_eq!(
            [
                "Title",
                "Author",
                "Subject",
                "Keywords",
                "Creator",
                "Producer",
                "CreationDate",
                "ModDate"
            ]
            .map(|k| info_text(&d, k)),
            [
                Some("Figures".to_owned()),
                Some("Ada Lovelace; Charles Babbage".to_owned()),
                Some("Q3".to_owned()),
                Some("a, b".to_owned()),
                Some("Writer".to_owned()),
                Some("Lamina PDF".to_owned()),
                Some("D:20260101120000+01'00'".to_owned()),
                Some("D:20261110093000Z".to_owned()),
            ]
        );
    }

    #[test]
    fn reading_maps_info_and_xmp_back_to_the_same_record() {
        let mut d = doc("", None, None);
        set(
            &mut d,
            MetadataPatch::new()
                .title(Some("T"))
                .authors(&["A", "B"])
                .producer(Some("P")),
        );
        let meta = read_metadata(&d).unwrap();
        assert_eq!(
            (
                meta.title.as_deref(),
                meta.authors.len(),
                meta.producer.as_deref(),
                meta.disagreements
            ),
            (Some("T"), 2, Some("P"), Vec::new())
        );
    }

    #[test]
    fn characters_xml_cannot_hold_are_left_out_of_both_sides() {
        let mut d = doc("", None, None);
        set(&mut d, MetadataPatch::new().title(Some("a\u{1}b")));
        assert_eq!(
            (xmp(&d).title(), info_text(&d, "Title")),
            (Some("ab".to_owned()), Some("ab".to_owned()))
        );
    }

    #[test]
    fn every_key_has_its_own_coalescing_key() {
        let keys: std::collections::BTreeSet<&str> = MetadataKey::ALL.iter().map(|k| k.coalescing_key()).collect();
        assert_eq!(keys.len(), MetadataKey::ALL.len());
    }

    #[test]
    fn every_key_names_its_own_row() {
        assert!(MetadataKey::ALL.iter().all(|k| k.field().mapping().info_key.is_some()));
    }
}

mod patch {
    use super::*;

    #[test]
    fn an_absent_field_is_left_alone() {
        let mut d = doc("", Some("<< /Title (Old) /Author (Someone) >>"), None);
        set(&mut d, MetadataPatch::new().title(Some("New")));
        assert_eq!(info_text(&d, "Author").as_deref(), Some("Someone"));
    }

    #[test]
    fn null_removes_the_field_from_both_sides() {
        let body = "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Old</rdf:li></rdf:Alt></dc:title>";
        let mut d = doc("", Some("<< /Title (Old) >>"), Some(packet(body).as_bytes()));
        let effect = set(&mut d, MetadataPatch::new().title(None));
        assert_eq!(
            (info_text(&d, "Title"), xmp(&d).title(), effect.removed),
            (None, None, true)
        );
    }

    #[test]
    fn an_empty_patch_changes_nothing_and_makes_no_packet() {
        let mut d = doc("", None, None);
        set(&mut d, MetadataPatch::new());
        assert!(!d.is_dirty());
    }

    #[test]
    fn a_file_without_a_packet_gets_a_minimal_one() {
        let mut d = doc("", None, None);
        set(&mut d, MetadataPatch::new().title(Some("T")));
        assert_eq!(xmp(&d).title().as_deref(), Some("T"));
    }

    #[test]
    fn at_sets_the_modification_and_metadata_dates() {
        let mut d = doc("", None, None);
        let at = date("D:20261110093000Z");
        SetMetadata::new(MetadataPatch::new().title(Some("T")), Some(at))
            .apply_to(&mut d)
            .unwrap();
        let p = xmp(&d);
        assert_eq!(
            (p.modify_date(), p.metadata_date(), info_text(&d, "ModDate")),
            (Some(at), Some(at), Some("D:20261110093000Z".to_owned()))
        );
    }

    #[test]
    fn a_malformed_packet_is_refused_unless_the_op_replaces_it() {
        let mut d = doc("", None, Some(b"<x:xmpmeta"));
        let refused = SetMetadata::new(MetadataPatch::new().title(Some("T")), None).apply_to(&mut d);
        assert_eq!(refused, Err(Error::MalformedXmp));
        SetMetadata::new(MetadataPatch::new().title(Some("T")), None)
            .replacing_malformed()
            .apply_to(&mut d)
            .unwrap();
        assert_eq!(xmp(&d).title().as_deref(), Some("T"));
    }

    #[test]
    fn the_stream_is_written_unfiltered() {
        let mut d = doc("", None, Some(packet("").as_bytes()));
        set(&mut d, MetadataPatch::new().title(Some("T")));
        let catalog = d.catalog().unwrap();
        let stream = d.deref(catalog.get("Metadata").unwrap()).unwrap();
        let laminapdf_cos::Object::Stream(s) = &*stream else {
            panic!("not a stream")
        };
        assert!(!s.dict.contains_key("Filter"));
    }

    #[test]
    fn pdfaid_stays_in_the_packet() {
        let body = "<pdfaid:part>2</pdfaid:part><pdfaid:conformance>B</pdfaid:conformance>";
        let mut d = doc("", None, Some(packet(body).as_bytes()));
        set(&mut d, MetadataPatch::new().title(Some("T")));
        let p = xmp(&d);
        assert_eq!((p.pdfa_part(), p.pdfa_conformance().as_deref()), (Some(2), Some("B")));
    }
}

mod precedence {
    use super::*;

    fn titled(info_mod: &str, xmp_mod: &str, catalog_extra: &str, claim: &str) -> DocumentMetadata {
        let body = format!(
            "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">From XMP</rdf:li></rdf:Alt></dc:title><xmp:ModifyDate>{xmp_mod}</xmp:ModifyDate>{claim}"
        );
        let info = format!("<< /Title (From Info) /ModDate ({info_mod}) >>");
        read_metadata(&doc(catalog_extra, Some(&info), Some(packet(&body).as_bytes()))).unwrap()
    }

    #[test]
    fn the_newer_modification_date_wins() {
        let meta = titled("D:20260101000000Z", "2026-06-01T00:00:00Z", "", "");
        assert_eq!(
            (meta.title.as_deref(), meta.source),
            (Some("From XMP"), MetadataSource::Xmp)
        );
    }

    #[test]
    fn info_wins_a_tie() {
        let meta = titled("D:20260101000000Z", "2026-01-01T00:00:00Z", "", "");
        assert_eq!(meta.title.as_deref(), Some("From Info"));
    }

    #[test]
    fn the_same_instant_in_two_zones_is_a_tie_info_wins() {
        let meta = titled("D:20260101120000Z", "2026-01-01T13:00:00+01:00", "", "");
        assert_eq!(meta.source, MetadataSource::Info);
    }

    #[test]
    fn info_wins_when_it_is_newer() {
        let meta = titled("D:20270101000000Z", "2026-01-01T00:00:00Z", "", "");
        assert_eq!(meta.source, MetadataSource::Info);
    }

    #[test]
    fn xmp_wins_in_a_pdf_2_file() {
        let meta = titled("D:20270101000000Z", "2026-01-01T00:00:00Z", "/Version /2.0", "");
        assert_eq!(meta.title.as_deref(), Some("From XMP"));
    }

    #[test]
    fn xmp_wins_in_a_pdfa_file() {
        let meta = titled(
            "D:20270101000000Z",
            "2026-01-01T00:00:00Z",
            "",
            "<pdfaid:part>2</pdfaid:part>",
        );
        assert_eq!(meta.title.as_deref(), Some("From XMP"));
    }

    #[test]
    fn a_disagreement_is_reported() {
        let meta = titled("D:20260101000000Z", "2026-01-01T00:00:00Z", "", "");
        assert_eq!(meta.disagreements, [MetadataKey::Title]);
    }

    #[test]
    fn info_alone_is_read_without_a_disagreement() {
        let meta = read_metadata(&doc("", Some("<< /Author (A; B) >>"), None)).unwrap();
        assert_eq!(
            (meta.authors, meta.disagreements, meta.packet),
            (vec!["A".to_owned(), "B".to_owned()], Vec::new(), PacketStatus::Absent)
        );
    }
}

mod pdf2 {
    use super::*;

    #[test]
    fn an_edit_writes_only_the_dates_to_info() {
        let mut d = doc("/Version /2.0", Some("<< >>"), None);
        let created = date("D:20260101000000Z");
        set(&mut d, MetadataPatch::new().title(Some("T")).created(Some(created)));
        let info = d.info().unwrap().unwrap();
        assert_eq!(
            (info.keys(), xmp(&d).title().as_deref()),
            (vec!["CreationDate".to_owned()], Some("T"))
        );
    }

    #[test]
    fn old_info_entries_are_kept_and_flagged() {
        let mut d = doc("/Version /2.0", Some("<< /Title (Old) >>"), None);
        set(&mut d, MetadataPatch::new().title(Some("New")));
        let meta = read_metadata(&d).unwrap();
        assert_eq!(
            (
                info_text(&d, "Title").as_deref(),
                meta.title.as_deref(),
                meta.kept_info_keys
            ),
            (Some("Old"), Some("New"), vec![MetadataKey::Title])
        );
    }

    #[test]
    fn a_removal_removes_the_old_info_entry_too() {
        let mut d = doc("/Version /2.0", Some("<< /Title (Old) >>"), None);
        RemoveMetadata::new(vec![MetadataKey::Title]).apply_to(&mut d).unwrap();
        assert_eq!(info_text(&d, "Title"), None);
    }
}

mod conformance {
    use super::*;

    fn warnings(body: &str, info: Option<&str>) -> Vec<ConformanceWarning> {
        conformance_warnings(&doc("", info, Some(packet(body).as_bytes()))).unwrap()
    }

    #[test]
    fn two_authors_in_a_pdfa_1_file() {
        let body = "<pdfaid:part>1</pdfaid:part><dc:creator><rdf:Seq><rdf:li>A</rdf:li><rdf:li>B</rdf:li></rdf:Seq></dc:creator>";
        assert_eq!(
            warnings(body, Some("<< /Author (A; B) >>")),
            [ConformanceWarning::PdfaOneAuthor { authors: 2 }]
        );
    }

    #[test]
    fn two_authors_in_a_pdfa_2_file_are_fine() {
        let body = "<pdfaid:part>2</pdfaid:part><dc:creator><rdf:Seq><rdf:li>A</rdf:li><rdf:li>B</rdf:li></rdf:Seq></dc:creator>";
        assert_eq!(warnings(body, Some("<< /Author (A; B) >>")), []);
    }

    #[test]
    fn no_title_in_a_pdfua_1_file() {
        assert_eq!(
            warnings("<pdfuaid:part>1</pdfuaid:part>", None),
            [ConformanceWarning::PdfuaTitleMissing]
        );
    }

    #[test]
    fn info_and_xmp_disagreeing_in_a_pdfa_file() {
        assert_eq!(
            warnings("<pdfaid:part>2</pdfaid:part>", Some("<< /Title (Only in Info) >>")),
            [ConformanceWarning::PdfaInfoMismatch {
                keys: vec![MetadataKey::Title]
            }]
        );
    }

    #[test]
    fn a_claim_without_a_problem_raises_nothing() {
        let body = "<pdfuaid:part>1</pdfuaid:part><dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">T</rdf:li></rdf:Alt></dc:title>";
        assert_eq!(warnings(body, None), []);
    }

    #[test]
    fn codes_lists_the_code_of_every_variant_in_order() {
        let samples = [
            ConformanceWarning::PdfaOneAuthor { authors: 2 },
            ConformanceWarning::PdfuaTitleMissing,
            ConformanceWarning::PdfaInfoMismatch { keys: Vec::new() },
            ConformanceWarning::DuplicateTagsIncomplete,
            ConformanceWarning::AnnotWithoutContents {
                annotations: Vec::new(),
            },
            ConformanceWarning::AnnotNotTagged {
                annotations: Vec::new(),
            },
            ConformanceWarning::PdfaAnnotFlags {
                annotations: Vec::new(),
            },
            ConformanceWarning::PdfaTransparency {
                annotations: Vec::new(),
            },
        ];
        assert_eq!(samples.map(|w| w.code()), ConformanceWarning::CODES);
    }

    #[test]
    fn every_warning_has_the_kind_codes_json_lists() {
        let samples = [
            ConformanceWarning::PdfaOneAuthor { authors: 2 },
            ConformanceWarning::PdfuaTitleMissing,
            ConformanceWarning::PdfaInfoMismatch { keys: Vec::new() },
            ConformanceWarning::DuplicateTagsIncomplete,
            ConformanceWarning::AnnotWithoutContents {
                annotations: Vec::new(),
            },
            ConformanceWarning::AnnotNotTagged {
                annotations: Vec::new(),
            },
            ConformanceWarning::PdfaAnnotFlags {
                annotations: Vec::new(),
            },
            ConformanceWarning::PdfaTransparency {
                annotations: Vec::new(),
            },
        ];
        crate::test_codes::assert_kinds_match(samples.iter().map(|w| (w.code(), w.kind().as_str())));
    }
}

mod set_info {
    use super::*;
    use crate::{InfoPatch, SetInfo};

    #[test]
    fn characters_xml_cannot_hold_leave_both_sides_alike() {
        let mut d = doc("", None, Some(packet("").as_bytes()));
        SetInfo::patch(InfoPatch::new().title(Some("a\u{1}b")))
            .apply_to(&mut d)
            .unwrap();
        assert_eq!(
            (info_text(&d, "Title"), xmp(&d).title()),
            (Some("ab".to_owned()), Some("ab".to_owned()))
        );
    }

    #[test]
    fn in_a_pdf_2_file_it_writes_only_the_dates_to_info_and_the_rest_to_a_new_packet() {
        let mut d = doc("/Version /2.0", Some("<< >>"), None);
        SetInfo::new(InfoPatch::new().title(Some("T")), Some(date("D:20261110093000Z")))
            .apply_to(&mut d)
            .unwrap();
        let info = d.info().unwrap().unwrap();
        assert_eq!(
            (info.keys(), xmp(&d).title().as_deref()),
            (vec!["ModDate".to_owned()], Some("T"))
        );
    }

    #[test]
    fn in_an_older_file_without_a_packet_it_adds_none() {
        let mut d = doc("", None, None);
        SetInfo::patch(InfoPatch::new().title(Some("T")))
            .apply_to(&mut d)
            .unwrap();
        assert!(matches!(d.xmp().unwrap(), PacketState::Absent));
    }
}
