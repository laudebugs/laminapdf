//! The full-rewrite writer (ADR 0007 §5): reachability, numbering, layout and the rebase onto
//! a readable sink.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::corpus;
use common::save::{corpus_doc, editable, facts, page_ids, reopen, rotate, set_title, two_pages};
use lamina_cos::{Document, ObjRef, Object, OpenOptions};
use lamina_model::Rotation;
use lamina_save::{FullWriter, Renumber, Save, SaveMode, SaveOptions, XrefKind};

fn full(doc: &mut lamina_ops::Document, options: SaveOptions) -> Vec<u8> {
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

#[test]
fn compact_returns_a_complete_map_and_ids_follow_it() {
    let mut doc = editable(&two_pages());
    rotate(&mut doc, 1, Rotation::None);
    let before: Vec<u32> = page_ids(&doc)
        .iter()
        .map(|id| id.to_string()[1..].split('.').next().unwrap().parse().unwrap())
        .collect();
    let report = doc
        .save_full_and_rebase(Vec::new(), &SaveOptions::new().with_renumber(Renumber::Compact))
        .map(|r| r.renumbered().cloned());
    let map = report.unwrap().unwrap();
    let after: Vec<String> = page_ids(&doc).iter().map(ToString::to_string).collect();
    let expected: Vec<String> = before.iter().map(|n| format!("p{}.0", map[n])).collect();
    assert_eq!(
        (
            map.keys().copied().collect::<Vec<_>>(),
            map.values().copied().collect::<Vec<_>>(),
            after
        ),
        (vec![1, 2, 3, 4, 5], vec![1, 2, 3, 4, 5], expected)
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
        let model = lamina_model::Document::from(doc);
        let expected = (model.page_count().unwrap(), facts(&model));
        for options in [
            SaveOptions::new(),
            SaveOptions::new().with_xref(XrefKind::Classic),
            SaveOptions::new().with_renumber(Renumber::Compact),
        ] {
            let mut out = Vec::new();
            let report = FullWriter::new(&model, options).write(&mut out).unwrap();
            let reopened = lamina_model::Document::open(out.clone(), OpenOptions::default()).unwrap();
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
