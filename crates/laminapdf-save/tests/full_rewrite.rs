//! The full-rewrite writer (ADR 0007 §5): reachability, numbering, layout and the rebase onto
//! a readable sink.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::save::ModeSaves as _;

use common::corpus;
use common::save::{corpus_doc, editable, facts, page_ids, reopen, rotate, set_title, two_pages};
use laminapdf_cos::{Document, ObjRef, Object, OpenOptions};
use laminapdf_model::Rotation;
use laminapdf_save::{FullWriter, Renumber, SaveMode, SaveOptions, XrefKind};

fn full(doc: &mut laminapdf_ops::Document, options: SaveOptions) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save_full(&mut out, &options).unwrap();
    out
}

fn numbers(doc: &Document) -> Vec<u32> {
    doc.object_numbers().collect()
}

#[test]
fn rotate_then_keep_keeps_numbers_and_page_ids_and_drops_unreachable_objects() {
    let mut doc = editable(&two_pages());
    let ids = page_ids(&doc);
    rotate(&mut doc, 0, Rotation::Clockwise90);
    let out = full(&mut doc, SaveOptions::new());
    let reopened = reopen(&out);
    let kept: Vec<u32> = numbers(&reopened).into_iter().filter(|&n| n <= 6).collect();
    assert_eq!(
        (kept, page_ids(&reopened), facts(&reopened).rotations),
        (
            vec![1, 2, 3, 4, 5],
            ids,
            vec![Rotation::Clockwise90, Rotation::Clockwise90]
        )
    );
}

#[test]
fn keep_numbers_object_and_xref_streams_after_every_number_in_use() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "T");
    let out = full(&mut doc, SaveOptions::new());
    let reopened = reopen(&out);
    assert_eq!(numbers(&reopened), vec![1, 2, 3, 4, 5, 7, 8]);
}

#[test]
fn earlier_revisions_disappear() {
    let mut doc = corpus_doc("hello-incremental.pdf");
    rotate(&mut doc, 0, Rotation::Half);
    let out = full(&mut doc, SaveOptions::new());
    let reopened = reopen(&out);
    assert_eq!(
        (reopened.revisions(), facts(&reopened).rotations),
        (1, vec![Rotation::Half])
    );
}

/// Object 1 unused, catalog 2, pages 3 with pages 4 and 5: compacting moves every number.
fn shifted() -> Vec<u8> {
    common::save::classic(
        &[
            "(orphan)",
            "<< /Type /Catalog /Pages 3 0 R >>",
            "<< /Type /Pages /Kids [4 0 R 5 0 R] /Count 2 >>",
            "<< /Type /Page /Parent 3 0 R >>",
            "<< /Type /Page /Parent 3 0 R /Rotate 90 >>",
        ],
        "",
    )
    .replace_root()
}

#[test]
fn compact_returns_a_complete_map_and_page_ids_follow_it() {
    let mut doc = editable(&shifted());
    rotate(&mut doc, 0, Rotation::Half);
    let before = page_ids(&doc);
    let report = doc
        .save_full_and_rebase(Vec::new(), &SaveOptions::new().with_renumber(Renumber::Compact))
        .unwrap();
    let map = report.renumbered().cloned().unwrap();
    let expected: Vec<laminapdf_model::PageId> = before
        .iter()
        .map(|id| {
            let old: u32 = id.to_string()[1..].split('.').next().unwrap().parse().unwrap();
            laminapdf_model::PageId::object(ObjRef::new(map[&old], 0))
        })
        .collect();
    assert_eq!(
        (
            map.into_iter().collect::<Vec<_>>(),
            page_ids(&doc),
            facts(&doc).rotations
        ),
        (
            vec![(2, 1), (3, 2), (4, 3), (5, 4)],
            expected,
            vec![Rotation::Half, Rotation::Clockwise90]
        )
    );
}

#[test]
fn compact_closes_gaps() {
    // Object 1 unused: the catalog is 2, pages 3, page 4.
    let bytes = common::save::classic(
        &[
            "(orphan)",
            "<< /Type /Catalog /Pages 3 0 R >>",
            "<< /Type /Pages /Kids [4 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 3 0 R >>",
        ],
        "",
    )
    .replace_root();
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    let report = FullWriter::new(&doc, SaveOptions::new().with_renumber(Renumber::Compact))
        .write(&mut out)
        .unwrap();
    let reopened = reopen(&out);
    assert_eq!(
        (
            report.renumbered().cloned().unwrap().into_iter().collect::<Vec<_>>(),
            reopened.page_count().unwrap(),
            numbers(&reopened),
        ),
        (vec![(2, 1), (3, 2), (4, 3)], 1, vec![1, 2, 3, 4, 5])
    );
}

#[test]
fn classic_output_has_no_streams_and_reads_back() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "Classic");
    let expected = facts(&doc);
    let out = full(&mut doc, SaveOptions::new().with_xref(XrefKind::Classic));
    let reopened = reopen(&out);
    assert_eq!(
        (
            reopened.has_xref_stream(),
            out.starts_with(b"%PDF-1.7\n"),
            facts(&reopened)
        ),
        (false, true, expected)
    );
}

#[test]
fn object_streams_raise_the_header_to_1_5() {
    let mut old = two_pages();
    old[7] = b'3'; // %PDF-1.3
    let doc = Document::open(old, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new()).write(&mut out).unwrap();
    assert!(out.starts_with(b"%PDF-1.5\n"));
}

#[test]
fn dangling_and_stale_references_are_written_as_null() {
    let bytes = common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R /Missing 99 0 R /Stale 3 1 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
        ],
        "",
    );
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new()).write(&mut out).unwrap();
    let catalog = reopen(&out).catalog().unwrap();
    assert_eq!(
        (catalog.get("Missing"), catalog.get("Stale")),
        (Some(&Object::Null), Some(&Object::Null))
    );
}

#[test]
fn generations_are_kept_with_keep() {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (num, generation, body) in [
        (1, 0, "<< /Type /Catalog /Pages 2 3 R >>"),
        (2, 3, "<< /Type /Pages /Kids [] /Count 0 >>"),
    ] {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{num} {generation} obj\n{body}\nendobj\n").as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!(
            "xref\n0 3\n0000000000 65535 f\r\n{:010} 00000 n\r\n{:010} 00003 n\r\ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets[0], offsets[1]
        )
        .as_bytes(),
    );
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new()).write(&mut out).unwrap();
    let reopened = reopen(&out);
    assert_eq!(
        (reopened.base_generation(2), reopened.page_count().unwrap()),
        (Some(3), 0)
    );
}

#[test]
fn every_unencrypted_corpus_file_rewrites_with_the_same_facts() {
    for (name, bytes) in corpus() {
        let doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
        if doc.is_encrypted() {
            continue;
        }
        let model = laminapdf_model::Document::from(doc);
        let expected = (model.page_count().unwrap(), facts(&model));
        for options in [
            SaveOptions::new(),
            SaveOptions::new().with_xref(XrefKind::Classic),
            SaveOptions::new().with_renumber(Renumber::Compact),
        ] {
            let mut out = Vec::new();
            let report = FullWriter::new(&model, options).write(&mut out).unwrap();
            let reopened = laminapdf_model::Document::open(out.clone(), OpenOptions::default()).unwrap();
            assert!(!reopened.was_repaired(), "{name} {options:?}");
            assert_eq!(
                (reopened.page_count().unwrap(), facts(&reopened)),
                expected,
                "{name} {options:?}"
            );
            assert_eq!(report.bytes_written(), out.len() as u64);
        }
    }
}

#[test]
fn full_rewrites_are_deterministic() {
    for options in [
        SaveOptions::new(),
        SaveOptions::new().with_xref(XrefKind::Classic),
        SaveOptions::new().with_renumber(Renumber::Compact),
    ] {
        let run = || {
            let mut doc = corpus_doc("outline.pdf");
            set_title(&mut doc, "Again");
            full(&mut doc, options)
        };
        assert_eq!(run(), run(), "{options:?}");
    }
}

#[test]
fn the_report_names_the_mode_and_size() {
    let mut doc = editable(&two_pages());
    let mut out = Vec::new();
    let report = doc.save_full(&mut out, &SaveOptions::new()).unwrap();
    assert_eq!(
        (report.mode(), report.bytes_written(), report.append_offset()),
        (SaveMode::FullRewrite, out.len() as u64, None)
    );
}

#[test]
fn a_streams_indirect_length_is_written_directly() {
    let bytes = common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>",
            "<< /Length 5 0 R >>\nstream\nBT ET\nendstream",
            "5",
        ],
        "",
    );
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new()).write(&mut out).unwrap();
    let reopened = reopen(&out);
    let page = reopened.cos().pages().unwrap().remove(0);
    assert_eq!(
        (reopened.page_content(&page).unwrap(), reopened.base_generation(5)),
        (b"BT ET".to_vec(), None)
    );
}

trait ReplaceRoot {
    fn replace_root(self) -> Vec<u8>;
}

impl ReplaceRoot for Vec<u8> {
    /// `classic` points `/Root` at 1; these files have their catalog at 2.
    fn replace_root(self) -> Vec<u8> {
        let text = String::from_utf8(self).unwrap();
        text.replace("/Root 1 0 R", "/Root 2 0 R").into_bytes()
    }
}

#[test]
fn objects_with_a_reference_value_are_kept() {
    let bytes = common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R /Alias 4 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "3 0 R",
        ],
        "",
    );
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new()).write(&mut out).unwrap();
    assert_eq!(*reopen(&out).object(4).unwrap(), Object::Ref(ObjRef::new(3, 0)));
}

#[test]
fn keep_lists_only_numbers_that_hold_something_whatever_the_input_size() {
    // A tiny file claiming two billion objects: the xref must not grow with the claim.
    let bytes = String::from_utf8(two_pages())
        .unwrap()
        .replace("/Size 7", "/Size 2000000000")
        .into_bytes();
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new()).write(&mut out).unwrap();
    let reopened = reopen(&out);
    assert_eq!((reopened.trailer().get_int("Size"), out.len() < 4096), (Some(9), true));
}

#[test]
fn keep_frees_unwritten_base_objects_with_the_next_generation() {
    let doc = Document::open(two_pages(), OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new().with_xref(XrefKind::Classic))
        .write(&mut out)
        .unwrap();
    let text = String::from_utf8_lossy(&out).into_owned();
    // 6 (the orphan) is freed: object 0 links to it, it ends the list, generation 1.
    assert!(
        text.contains("xref\n0 7\n0000000006 65535 f\r\n") && text.contains("0000000000 00001 f\r\ntrailer"),
        "{text}"
    );
}

#[test]
fn an_object_that_cannot_be_loaded_fails_the_rewrite() {
    let bytes = common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R /Broken 4 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Unterminated [1 2",
        ],
        "",
    );
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    let result = FullWriter::new(&doc, SaveOptions::new()).write(&mut out);
    assert_eq!((result.is_err(), out.len()), (true, 0));
}

#[test]
fn a_nested_length_reference_is_kept() {
    let bytes = common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R /Extra << /Length 4 0 R >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "42",
        ],
        "",
    );
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new()).write(&mut out).unwrap();
    let reopened = reopen(&out);
    let extra = reopened.catalog().unwrap().get("Extra").cloned().unwrap();
    let length = extra.as_dict().and_then(|d| d.get("Length")).cloned().unwrap();
    assert_eq!(
        (length.clone(), reopened.deref(&length).unwrap().as_int()),
        (Object::Ref(ObjRef::new(4, 0)), Some(42))
    );
}

#[test]
fn an_indirect_trailer_id_is_not_written_as_an_orphan() {
    let bytes = common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "[<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>]",
        ],
        "/ID 4 0 R",
    );
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new()).write(&mut out).unwrap();
    let reopened = reopen(&out);
    assert_eq!(
        (reopened.base_generation(4), common::save::ids(&reopened).0.len()),
        (None, 16)
    );
}

#[test]
fn indirect_id_elements_are_kept() {
    let bytes = common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<00112233445566778899AABBCCDDEEFF>",
        ],
        "/ID [4 0 R 4 0 R]",
    );
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let mut out = Vec::new();
    FullWriter::new(&doc, SaveOptions::new()).write(&mut out).unwrap();
    let expected: Vec<u8> = (0..16u8).map(|i| i * 0x11).collect();
    assert_eq!(common::save::ids(&reopen(&out)).0, expected);
}

#[test]
fn nesting_exactly_at_the_limit_is_accepted_like_the_reader_does() {
    let bytes = common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R /A [[1]] >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
        ],
        "",
    );
    let limits = laminapdf_cos::Limits::default().with_max_nesting_depth(3);
    let doc = Document::open(bytes, OpenOptions::default().with_limits(limits)).unwrap();
    let mut out = Vec::new();
    assert!(FullWriter::new(&doc, SaveOptions::new()).write(&mut out).is_ok());
}

#[test]
fn numbers_past_max_object_number_are_refused_before_writing() {
    // Objects 1–6; the object stream would be 7.
    let limits = laminapdf_cos::Limits::default().with_max_object_number(6);
    let doc = Document::open(two_pages(), OpenOptions::default().with_limits(limits)).unwrap();
    let mut out = Vec::new();
    let result = FullWriter::new(&doc, SaveOptions::new()).write(&mut out);
    assert_eq!(
        (result.map(|_| ()).map_err(|e| e.code()), out.len()),
        (Err("invalid-structure"), 0)
    );
}

#[test]
fn a_sink_that_already_holds_bytes_is_refused() {
    let doc = Document::open(two_pages(), OpenOptions::default()).unwrap();
    let mut out = b"%".to_vec();
    assert_eq!(
        FullWriter::new(&doc, SaveOptions::new()).write(&mut out).err(),
        Some(laminapdf_save::SaveError::Sink(laminapdf_save::SinkError::NotEmpty {
            held: 1
        }))
    );
}
