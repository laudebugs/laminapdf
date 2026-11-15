//! The `content_ir` fuzz target's entry point, shared with `tests/ir.rs`'s regression replay:
//! the content IR (ADR 0022) over fuzzed content, its invariants checked, a few objects edited
//! and the result read back.

use laminapdf_content::{ContentObject, ContentOp, ContentOps, ContentOwner, EncodedPart, Operator, interpret_page};
use laminapdf_cos::Limits;
use laminapdf_model::{Document, OpenOptions, PointPt, RectPt};

use super::fuzz_entry::fixture_file;

/// The first byte picks how many parts the rest is cut into (1–4) and which objects are
/// edited; the rest is the content.
pub fn content_ir(data: &[u8]) {
    let Some((&control, content)) = data.split_first() else {
        return;
    };
    let count = usize::from(control % 4) + 1;
    let size = content.len().div_ceil(count).max(1);
    let parts: Vec<_> = content.chunks(size).map(|c| (None, c.to_vec())).collect();
    let owner = ContentOwner::Page("p1.0".parse().unwrap());
    let Ok(mut ops) = ContentOps::from_parts(owner.clone(), parts.clone(), &Limits::default()) else {
        return;
    };
    // Unedited, every part is written back as it was.
    let written: Vec<_> = ops
        .encode_every_part()
        .parts
        .iter()
        .filter_map(EncodedPart::bytes)
        .map(<[u8]>::to_vec)
        .collect();
    let sources: Vec<_> = parts.into_iter().map(|(_, bytes)| bytes).collect();
    assert_eq!(written, sources, "an unedited IR writes its parts back");
    check_objects(&ops);
    // Each object finds itself by its lasting name.
    for object in ops.objects() {
        let name = ops.reference(object.id());
        assert!(
            name.is_some_and(|n| ops.find(&n).is_some()),
            "an object finds its own name"
        );
    }
    // Edit every few objects: remove one, replace the next with a path, as `control` picks.
    let ids: Vec<_> = ops.objects().iter().map(ContentObject::id).collect();
    for (i, id) in ids.into_iter().enumerate() {
        match (i + usize::from(control / 4)) % 3 {
            0 => {
                assert!(ops.remove(id));
            }
            1 => {
                let path = vec![
                    ContentOp::new(Operator::MoveTo, vec![1.into(), 2.into()]),
                    ContentOp::new(Operator::Stroke, Vec::new()),
                ];
                assert!(ops.replace(id, path));
            }
            _ => {}
        }
    }
    check_objects(&ops);
    let joined: Vec<u8> = ops
        .encode_every_part()
        .parts
        .iter()
        .filter_map(EncodedPart::bytes)
        .collect::<Vec<_>>()
        .join(&b'\n');
    // What was written reads again, within the same limits.
    let _ = ContentOps::from_parts(owner, vec![(None, joined)], &Limits::default());
    record(content);
}

/// Objects in order, apart, each inside the operators; marked content in range.
fn check_objects(ops: &ContentOps) {
    let mut end = 0;
    for object in ops.objects() {
        let range = object.operators();
        assert!(
            range.start >= end && range.end <= ops.operators().len(),
            "objects in order, apart"
        );
        end = range.end;
        assert!(
            object.marked().iter().all(|&m| m < ops.marked().len()),
            "marked content in range"
        );
    }
}

/// The content as a page of a small file, recorded by the interpreter and hit-tested.
fn record(content: &[u8]) {
    let Ok(doc) = Document::open(fixture_file(content), OpenOptions::default()) else {
        return;
    };
    let Ok(pages) = doc.pages() else { return };
    let Some(page) = pages.get(0) else { return };
    let (Ok(mut ops), Ok(frame)) = (
        ContentOps::read_page(&doc, page.cos(), ContentOwner::Page(page.id().clone())),
        page.frame(),
    ) else {
        return;
    };
    let _ = interpret_page(&doc, &page, &mut ops.recorder(frame));
    let _ = ops.hit_test(PointPt::from_points(10.0, 10.0));
    let _ = ops.hit_test_rect(&RectPt::from_points(0.0, 0.0, 300.0, 300.0));
}
