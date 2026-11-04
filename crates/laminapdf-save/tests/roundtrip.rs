//! SPEC case 4: write → read → re-serialize is identical, in classic, compact and
//! incremental form.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use common::{open, update};
use laminapdf_cos::{Dict, Name, ObjRef, Object, PdfString, Stream};
use laminapdf_save::XrefEol;
use laminapdf_save::test_support::Writer;
use laminapdf_syntax::{Limits, parse_object};

const MAX_DEPTH: usize = Limits::new().max_nesting_depth();

fn dict<const N: usize>(entries: [(&str, Object); N]) -> Dict {
    entries.into_iter().collect()
}

fn nested_array(depth: usize) -> Object {
    (0..depth).fold(Object::Array(Vec::new()), |inner, _| Object::Array(vec![inner]))
}

/// One of every object type, plus escaping edge cases.
fn sample_objects() -> Vec<Object> {
    let all_bytes: Vec<u8> = (0..=255).collect();
    let name_bytes: Vec<u8> = (1..=255).collect();
    vec![
        Object::Null,
        true.into(),
        false.into(),
        0.into(),
        (-42).into(),
        Object::Integer(i64::MAX),
        Object::Integer(i64::MIN),
        0.5.into(),
        (-0.25).into(),
        Object::Real(123_456.789_012),
        Object::Real(1e-7),
        PdfString::literal(all_bytes.clone()).into(),
        PdfString::literal(&b"unbalanced ) ( \\ \r\n"[..]).into(),
        PdfString::literal(Vec::new()).into(),
        PdfString::hex(all_bytes).into(),
        PdfString::hex(Vec::new()).into(),
        Object::Name(Name::new(name_bytes)),
        Object::Name(Name::new(&b"A#B/C(D)E<F>G[H]I{J}K%L M"[..])),
        Object::Name(Name::new(Vec::new())),
        vec![1.into(), Object::name("x"), vec![Object::Null].into(), dict([]).into()].into(),
        dict([
            ("Type", Object::name("Test")),
            ("Nested", dict([("Deep", ObjRef::new(1, 0).into())]).into()),
        ])
        .into(),
        ObjRef::new(12, 3).into(),
        Stream::new(
            dict([("Type", Object::name("XObject"))]),
            b"raw\nbytes\r\nendstream-lookalike".to_vec(),
        )
        .into(),
        Stream::flate(dict([]), b"compressed payload").into(),
        nested_array(MAX_DEPTH - 1),
    ]
}

/// A one-page document with the sample objects as extra indirect objects (numbers 4..).
fn build(writer: &mut Writer) -> Dict {
    let pages = writer.reserve();
    let page = writer.add(dict([
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("MediaBox", vec![0.into(), 0.into(), 595.into(), 842.into()].into()),
    ]));
    writer.set(
        pages,
        dict([
            ("Type", Object::name("Pages")),
            ("Kids", vec![page.into()].into()),
            ("Count", 1.into()),
        ]),
    );
    let root = writer.add(dict([("Type", Object::name("Catalog")), ("Pages", pages.into())]));
    for obj in sample_objects() {
        writer.add(obj);
    }
    dict([("Root", root.into())])
}

fn reserialized(bytes: &[u8]) -> Vec<Vec<u8>> {
    let doc = open(bytes).unwrap();
    assert!(!doc.was_repaired(), "writer output should not need repair");
    (4..4 + u32::try_from(sample_objects().len()).unwrap())
        .map(|n| doc.object(n).unwrap().to_bytes())
        .collect()
}

fn originals() -> Vec<Vec<u8>> {
    sample_objects().iter().map(Object::to_bytes).collect()
}

/// Asserts equality object by object, so a failure names the objects that differ.
fn assert_same_objects(ours: &[Vec<u8>], expected: &[Vec<u8>]) {
    let show = |b: &[u8]| String::from_utf8_lossy(&b[..b.len().min(80)]).into_owned();
    let diffs: Vec<String> = ours
        .iter()
        .zip(expected)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| format!("#{i}: got {:?} want {:?}", show(a), show(b)))
        .collect();
    assert!(diffs.is_empty() && ours.len() == expected.len(), "{}", diffs.join("\n"));
}

#[test]
fn direct_objects_reparse_to_identical_bytes() {
    let failures: Vec<String> = sample_objects()
        .iter()
        .filter(|o| !matches!(o, Object::Stream(_)))
        .filter_map(|o| {
            let bytes = o.to_bytes();
            let again = parse_object(&bytes).map(|p| p.to_bytes());
            (again.as_ref() != Ok(&bytes)).then(|| format!("{:?}", String::from_utf8_lossy(&bytes)))
        })
        .collect();
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn nesting_at_the_limit_round_trips() {
    let bytes = nested_array(MAX_DEPTH - 1).to_bytes();
    assert_eq!(parse_object(&bytes).map(|o| o.to_bytes()), Ok(bytes));
}

#[test]
fn classic_file_round_trips_every_object() {
    let mut w = Writer::new(1, 7);
    let trailer = build(&mut w);
    assert_same_objects(&reserialized(&w.finish(&trailer).unwrap()), &originals());
}

#[test]
fn classic_file_round_trips_with_space_lf_xref() {
    let mut w = Writer::new(1, 4).with_xref_eol(XrefEol::SpaceLf);
    let trailer = build(&mut w);
    assert_same_objects(&reserialized(&w.finish(&trailer).unwrap()), &originals());
}

#[test]
fn compact_file_round_trips_every_object() {
    let mut w = Writer::new(1, 7);
    let trailer = build(&mut w);
    assert_same_objects(&reserialized(&w.finish_compact(&trailer).unwrap()), &originals());
}

#[test]
fn compact_file_uses_xref_stream_and_object_stream() {
    let mut w = Writer::new(1, 7);
    let trailer = build(&mut w);
    let bytes = w.finish_compact(&trailer).unwrap();
    let doc = open(&bytes).unwrap();
    let has_objstm = bytes.windows(13).any(|w| w == b"/Type /ObjStm");
    assert!(doc.has_xref_stream() && has_objstm && doc.page_count() == Ok(1));
}

#[test]
fn compact_file_raises_header_to_1_5() {
    let mut w = Writer::new(1, 4);
    let trailer = build(&mut w);
    assert!(w.finish_compact(&trailer).unwrap().starts_with(b"%PDF-1.5\n"));
}

fn value_of(bytes: &[u8], num: u32) -> Object {
    std::sync::Arc::unwrap_or_clone(open(bytes).unwrap().resolve(ObjRef::new(num, 0)).unwrap())
}

fn two_and_three_revisions() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut w = Writer::new(1, 7);
    let trailer = build(&mut w);
    let base = w.finish(&trailer).unwrap();
    let rev2 = update(&base, &[(ObjRef::new(4, 0), Object::from(2))], XrefEol::CrLf).unwrap();
    let rev3 = update(
        &rev2,
        &[
            (ObjRef::new(4, 0), Object::from(3)),
            (ObjRef::new(100, 0), PdfString::literal("new").into()),
        ],
        XrefEol::SpaceLf,
    )
    .unwrap();
    (base, rev2, rev3)
}

#[test]
fn incremental_update_leaves_original_bytes_untouched() {
    let (base, rev2, _) = two_and_three_revisions();
    assert_eq!(&rev2[..base.len()], &base[..]);
}

#[test]
fn incremental_update_counts_two_revisions() {
    let (_, rev2, _) = two_and_three_revisions();
    assert_eq!(open(&rev2).unwrap().revisions(), 2);
}

#[test]
fn incremental_update_counts_three_revisions() {
    let (_, _, rev3) = two_and_three_revisions();
    assert_eq!(open(&rev3).unwrap().revisions(), 3);
}

#[test]
fn incremental_update_newest_value_wins_after_two_revisions() {
    let (_, rev2, _) = two_and_three_revisions();
    assert_eq!(value_of(&rev2, 4), Object::Integer(2));
}

#[test]
fn incremental_update_newest_value_wins_after_three_revisions() {
    let (_, _, rev3) = two_and_three_revisions();
    assert_eq!(value_of(&rev3, 4), Object::Integer(3));
}

#[test]
fn incremental_update_adds_new_object_and_grows_size() {
    let (_, _, rev3) = two_and_three_revisions();
    let doc = open(&rev3).unwrap();
    assert_eq!(
        (
            doc.resolve(ObjRef::new(100, 0)).unwrap().as_ref().clone(),
            doc.trailer().get_int("Size")
        ),
        (PdfString::literal("new").into(), Some(101))
    );
}

#[test]
fn incremental_update_keeps_untouched_objects() {
    let (_, _, rev3) = two_and_three_revisions();
    let doc = open(&rev3).unwrap();
    let all: Vec<Vec<u8>> = (5..4 + u32::try_from(sample_objects().len()).unwrap())
        .map(|n| doc.object(n).unwrap().to_bytes())
        .collect();
    assert_same_objects(&all, &originals()[1..]);
}

#[test]
fn incremental_update_on_compact_file_reads_back() {
    let mut w = Writer::new(1, 7);
    let trailer = build(&mut w);
    let base = w.finish_compact(&trailer).unwrap();
    let updated = update(&base, &[(ObjRef::new(5, 0), Object::from(7))], XrefEol::CrLf).unwrap();
    let doc = open(&updated).unwrap();
    assert_eq!(
        (
            doc.resolve(ObjRef::new(5, 0)).unwrap().as_ref().clone(),
            doc.revisions(),
            doc.page_count()
        ),
        (Object::Integer(7), 2, Ok(1))
    );
}

#[test]
fn incremental_update_refuses_repaired_base() {
    let broken = common::fixture("corpus/hello-broken.pdf");
    let result = update(&broken, &[(ObjRef::new(5, 0), Object::Null)], XrefEol::CrLf);
    assert_eq!(result, Err(laminapdf_cos::Error::RepairedBase));
}
