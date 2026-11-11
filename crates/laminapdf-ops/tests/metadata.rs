//! The metadata ops (E1c): the frozen v1 samples, Info and XMP agreeing after every op, undo
//! and redo, coalescing, and the codes the ops add.
#![expect(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

mod common;

use common::{corpus, fixture, fixture_text, open};
use laminapdf_model::xmp::{Packet, PacketState};
use laminapdf_ops::{
    ConformanceWarning, Document, MetadataKey, MetadataPatch, Op, OpLog, Origin, RemoveMetadata, SetMetadata,
    read_metadata,
};

fn metadata_pdf() -> Document {
    open(&fixture("golden/metadata.pdf"))
}

fn packet(doc: &Document) -> Packet {
    match doc.xmp().unwrap() {
        PacketState::Parsed(p) => p,
        other => panic!("no packet: {other:?}"),
    }
}

fn sample(name: &str) -> Op {
    serde_json::from_slice(&fixture(&format!("ops/v1/{name}.json"))).unwrap()
}

/// Info and XMP hold the same values: no field disagrees.
fn assert_in_step(doc: &Document) {
    let meta = read_metadata(doc).unwrap();
    assert_eq!(meta.disagreements, [], "{meta:?}");
}

mod samples {
    use super::*;

    #[test]
    fn set_metadata_writes_info_and_xmp() {
        let mut doc = metadata_pdf();
        doc.apply_all(Origin::Local, [sample("set-metadata")]).unwrap();
        let p = packet(&doc);
        let info = doc.info().unwrap().unwrap();
        assert_eq!(
            (
                p.title().as_deref(),
                p.authors(),
                info.author().unwrap().as_deref(),
                p.subject(),
                p.keywords().as_deref(),
            ),
            (
                Some("Quarterly figures"),
                vec!["Ada Lovelace".to_owned(), "Charles Babbage".to_owned()],
                Some("Ada Lovelace; Charles Babbage"),
                None,
                Some("Привет, 世界"),
            )
        );
        assert_in_step(&doc);
    }

    #[test]
    fn remove_metadata_removes_from_both() {
        let mut doc = metadata_pdf();
        doc.apply_all(Origin::Local, [sample("remove-metadata")]).unwrap();
        let p = packet(&doc);
        let info = doc.info().unwrap().unwrap();
        assert_eq!(
            (
                p.authors().len(),
                p.producer(),
                info.author().unwrap(),
                info.producer().unwrap()
            ),
            (0, None, None, None)
        );
        assert_in_step(&doc);
    }

    #[test]
    fn set_info_now_keeps_an_existing_packet_in_step() {
        let mut doc = metadata_pdf();
        doc.apply_all(Origin::Local, [sample("set-info")]).unwrap();
        assert_eq!(packet(&doc).title().as_deref(), Some("Quarterly report"));
        assert_in_step(&doc);
    }

    #[test]
    fn set_info_adds_no_packet_to_a_file_without_one() {
        let mut doc = corpus("outline.pdf");
        doc.apply_all(Origin::Local, [sample("set-info")]).unwrap();
        assert!(matches!(doc.xmp().unwrap(), PacketState::Absent));
    }

    #[test]
    fn a_log_mixing_set_info_and_set_metadata_applies() {
        let log = OpLog::from_json(&fixture_text("ops/v1/mixed-log.json")).unwrap();
        let mut doc = metadata_pdf();
        doc.apply_all(Origin::Local, log.into_ops()).unwrap();
        let meta = read_metadata(&doc).unwrap();
        assert_eq!(
            (
                meta.title.as_deref(),
                meta.authors,
                meta.producer,
                doc.history().undo_len()
            ),
            (Some("Quarterly figures"), vec!["Ada Lovelace".to_owned()], None, 1)
        );
        assert_in_step(&doc);
    }
}

mod undo {
    use super::*;

    #[test]
    fn undoing_an_edit_empties_the_overlay_and_redo_writes_the_same_objects() {
        let mut doc = metadata_pdf();
        let op: Op = SetMetadata::new(MetadataPatch::new().title(Some("New")).authors(&["A", "B"]), None).into();
        doc.apply_all(Origin::Local, [op]).unwrap();
        let applied: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
        doc.undo().unwrap();
        assert!(!doc.is_dirty());
        doc.redo().unwrap();
        let redone: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
        assert_eq!(redone, applied);
    }

    #[test]
    fn typing_into_one_field_coalesces_into_one_undo_step() {
        let mut doc = metadata_pdf();
        let key = MetadataKey::Title.coalescing_key();
        for text in ["Q", "Qu", "Quarterly"] {
            doc.transact_coalescing(key, |tx| {
                tx.apply(SetMetadata::new(MetadataPatch::new().title(Some(text)), None))?;
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(doc.history().undo_len(), 1);
        doc.undo().unwrap();
        assert!(!doc.is_dirty());
    }

    #[test]
    fn a_removal_says_earlier_revisions_still_hold_the_values() {
        let mut doc = metadata_pdf();
        let applied = doc
            .apply_all(Origin::Local, [RemoveMetadata::new(vec![MetadataKey::Title]).into()])
            .unwrap();
        assert!(applied.still_in_earlier_revisions);
    }

    #[test]
    fn removing_what_isnt_there_changes_nothing() {
        let mut doc = corpus("hello.pdf");
        let applied = doc
            .apply_all(Origin::Local, [RemoveMetadata::new(vec![MetadataKey::Title]).into()])
            .unwrap();
        assert_eq!(
            (applied.undo.is_empty(), applied.still_in_earlier_revisions),
            (true, false)
        );
    }
}

mod pdf2 {
    use super::*;

    fn pdf20() -> Document {
        open(&fixture("metadata/pdf20-xmp-only.pdf"))
    }

    #[test]
    fn an_edit_adds_no_deprecated_info_key() {
        let mut doc = pdf20();
        let before = doc.info().unwrap().unwrap().keys();
        let op: Op = SetMetadata::new(
            MetadataPatch::new().title(Some("T")).subject(Some("S")),
            Some("D:20261110093000Z".parse().unwrap()),
        )
        .into();
        doc.apply_all(Origin::Local, [op]).unwrap();
        assert_eq!(doc.info().unwrap().unwrap().keys(), before);
        assert_eq!(packet(&doc).title().as_deref(), Some("T"));
    }

    #[test]
    fn its_metadata_reads_from_xmp() {
        let meta = read_metadata(&pdf20()).unwrap();
        assert_eq!(
            (meta.title.as_deref(), meta.authors.len(), meta.disagreements),
            (Some("Quarterly figures"), 2, Vec::new())
        );
    }
}

#[test]
fn op_warning_codes_match_the_fixture() {
    let codes: serde_json::Value = serde_json::from_slice(&fixture("codes.json")).unwrap();
    let listed: Vec<&str> = codes["opWarnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["code"].as_str().unwrap())
        .collect();
    assert_eq!(listed, ConformanceWarning::CODES);
}
