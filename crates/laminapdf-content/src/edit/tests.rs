use laminapdf_cos::Limits;
use laminapdf_syntax::Object;

use super::*;

fn owner() -> ContentOwner {
    ContentOwner::Page("p3.0".parse().unwrap())
}

fn ir(parts: &[&[u8]]) -> ContentOps {
    let parts = parts.iter().map(|p| (None, p.to_vec())).collect();
    ContentOps::from_parts(owner(), parts, &Limits::default()).unwrap()
}

fn kinds(ops: &ContentOps) -> Vec<ContentObjectKind> {
    ops.objects().iter().map(ContentObject::kind).collect()
}

/// Every part written again, joined as the interpreter reads them.
fn written(ops: &ContentOps) -> Vec<u8> {
    let encoded = ops.encode_every_part();
    let parts: Vec<&[u8]> = encoded.parts.iter().filter_map(EncodedPart::bytes).collect();
    parts.join(&b'\n')
}

fn object(ops: &ContentOps, ordinal: usize) -> ContentObjectId {
    ops.objects()[ordinal].id()
}

/// Gives the object at `ordinal` a box from its left, top, width and height.
fn place(ops: &mut ContentOps, ordinal: usize, [x, y, width, height]: [f64; 4]) {
    ops.objects[ordinal].bounds = Some(RectPt::from_points(x, y, width, height));
}

#[test]
fn objects_should_group_text_paths_images_xobjects_and_shadings() {
    let ops = ir(&[
        b"q 0 g 10 10 m 20 20 l S BT /F1 9 Tf (a) Tj ET /Im1 Do BI /W 1 /H 1 /BPC 8 /CS /G ID \x00 EI /Sh1 sh Q",
    ]);
    assert_eq!(
        kinds(&ops),
        [
            ContentObjectKind::Path,
            ContentObjectKind::Text,
            ContentObjectKind::XObject,
            ContentObjectKind::Image,
            ContentObjectKind::Shading
        ]
    );
}

#[test]
fn a_path_should_take_its_clip_and_painting_operators() {
    let ops = ir(&[b"0 0 10 10 re W n 1 g 5 5 m 6 6 l S"]);
    let ranges: Vec<_> = ops.objects().iter().map(ContentObject::operators).collect();
    assert_eq!(ranges, [0..3, 4..7]);
}

#[test]
fn a_path_another_operator_interrupts_should_end_at_its_last_segment() {
    let ops = ir(&[b"0 0 m 1 1 l BT ET"]);
    let ranges: Vec<_> = ops.objects().iter().map(ContentObject::operators).collect();
    assert_eq!(ranges, [0..2, 2..4]);
}

#[test]
fn an_object_should_know_the_marked_content_around_it() {
    let ops = ir(&[b"/P <</MCID 4>> BDC BT (a) Tj ET EMC 0 0 m 1 1 l S"]);
    let marked = ops.marked();
    assert_eq!((marked[0].mcid, marked[0].begin, marked[0].end), (Some(4), 0, Some(4)));
    assert_eq!(ops.objects()[0].marked(), [0]);
    assert_eq!(ops.objects()[1].marked(), [0_usize; 0]);
}

#[test]
fn marked_content_inside_a_text_object_should_not_surround_it() {
    let ops = ir(&[b"BT /Span <</MCID 1>> BDC (a) Tj EMC ET 0 0 m S"]);
    assert_eq!((ops.marked()[0].begin, ops.marked()[0].end), (1, Some(3)));
    assert!(ops.objects().iter().all(|o| o.marked().is_empty()));
}

#[test]
fn spans_should_tile_the_source_from_its_start_to_the_last_operator() {
    let source: &[u8] = b"  % a comment\n q 1 0 0 1 5 5 cm BI /W 1 /H 1 /BPC 8 /CS /G ID \x07 EI Q trailing";
    let ops = ir(&[source]);
    let spans: Vec<_> = ops.operators().iter().filter_map(ContentOp::span).collect();
    assert_eq!(spans.first().map(|s| s.start), Some(0));
    assert!(spans.windows(2).all(|w| w[0].end == w[1].start));
}

#[test]
fn an_inline_image_should_keep_its_bytes_through_ei() {
    let ops = ir(&[b"q BI /W 1 /H 1 /BPC 8 /CS /G ID \x07 EI Q"]);
    let image = &ops.operators()[1];
    assert_eq!(
        image.inline_image(),
        Some(&b"BI /W 1 /H 1 /BPC 8 /CS /G ID \x07 EI"[..])
    );
}

#[test]
fn an_unedited_ir_should_leave_every_part_unchanged() {
    let ops = ir(&[b"q 1 0 0 RG", b"0 0 m 5 5 l S Q"]);
    assert!(ops.encode().parts.iter().all(|p| p.bytes().is_none()));
}

#[test]
fn every_part_written_from_an_unedited_ir_should_equal_its_source() {
    let parts: [&[u8]; 3] = [b"q 1 0 0 RG\n", b"%c\n0 0 m 5 5 l S Q 7 ", b"(stray"];
    let ops = ir(&parts);
    let encoded = ops.encode_every_part();
    let written: Vec<_> = encoded.parts.iter().filter_map(EncodedPart::bytes).collect();
    assert_eq!(written, parts);
}

#[test]
fn replacing_an_object_should_write_only_its_operators_again() {
    let mut ops = ir(&[b"q 0.8509804 g 0 0 m 5 5 l f Q\nBT  (a)   Tj ET"]);
    let path = object(&ops, 0);
    let replacement = vec![
        ContentOp::new(Operator::MoveTo, vec![Object::Integer(1), Object::Integer(1)]),
        ContentOp::new(Operator::LineTo, vec![Object::Real(2.5), Object::Integer(2)]),
        ContentOp::new(Operator::Fill, Vec::new()),
    ];
    assert!(ops.replace(path, replacement));
    assert_eq!(written(&ops), b"q 0.8509804 g 1 1 m\n2.5 2 l\nf\n Q\nBT  (a)   Tj ET");
}

#[test]
fn removing_an_object_should_drop_its_bytes_and_keep_the_rest() {
    let mut ops = ir(&[b"BT (a) Tj ET\n0 0 m 5 5 l S\nBT (b) Tj ET"]);
    assert!(ops.remove(object(&ops, 1)));
    assert_eq!(written(&ops), b"BT (a) Tj ET\n\nBT (b) Tj ET");
    assert_eq!(kinds(&ops), [ContentObjectKind::Text, ContentObjectKind::Text]);
}

#[test]
fn an_edit_should_rewrite_only_the_parts_it_touches() {
    let mut ops = ir(&[b"BT (a) Tj ET", b"0 0 m 5 5 l S", b"BT (b) Tj ET"]);
    assert!(ops.remove(object(&ops, 1)));
    let touched: Vec<_> = ops.encode().parts.iter().map(|p| p.bytes().is_some()).collect();
    assert_eq!(touched, [false, true, false]);
}

#[test]
fn a_written_operator_should_not_run_into_the_token_before_it() {
    let mut ops = ir(&[b"/F1 9 Tf/P BMC 0 0 m S EMC"]);
    let path = object(&ops, 0);
    assert!(ops.replace(path, vec![ContentOp::new(Operator::EndPath, Vec::new())]));
    assert_eq!(written(&ops), b"/F1 9 Tf/P BMC n\n EMC");
}

#[test]
fn ids_should_survive_an_edit_to_an_earlier_object() {
    let mut ops = ir(&[b"0 0 m S BT (a) Tj ET 1 1 m S"]);
    let last = object(&ops, 2);
    assert!(ops.remove(object(&ops, 0)));
    assert_eq!(ops.object(last).map(ContentObject::operators), Some(3..5));
}

#[test]
fn a_reference_should_find_its_object_in_a_rebuilt_ir() {
    let ops = ir(&[b"0 0 m S BT (a) Tj ET"]);
    let reference = ops.reference(object(&ops, 1)).unwrap();
    let rebuilt = ir(&[b"0 0 m S BT (a) Tj ET"]);
    assert_eq!(rebuilt.find(&reference), Some(object(&rebuilt, 1)));
}

#[test]
fn a_reference_to_changed_bytes_should_be_stale() {
    let ops = ir(&[b"0 0 m S BT (a) Tj ET"]);
    let reference = ops.reference(object(&ops, 1)).unwrap();
    let changed = ir(&[b"0 0 m S BT (b) Tj ET"]);
    assert_eq!(changed.find(&reference), None);
}

#[test]
fn a_reference_should_not_depend_on_the_white_space_before_the_object() {
    let ops = ir(&[b"0 0 m S BT (a) Tj ET"]);
    let reference = ops.reference(object(&ops, 1)).unwrap();
    let spaced = ir(&[b"0 0 m S\n\n   BT (a) Tj ET"]);
    assert_eq!(spaced.find(&reference), Some(object(&spaced, 1)));
}

#[test]
fn a_rewritten_object_should_be_found_again_after_its_content_is_read_back() {
    let mut ops = ir(&[b"q 0 0 m 5 5 l S Q"]);
    let path = object(&ops, 0);
    let replacement = vec![
        ContentOp::new(Operator::MoveTo, vec![Object::Integer(1), Object::Integer(1)]),
        ContentOp::new(Operator::Stroke, Vec::new()),
    ];
    assert!(ops.replace(path, replacement));
    let reference = ops.reference(path).unwrap();
    let reread = ir(&[&written(&ops)]);
    assert_eq!(reread.find(&reference), Some(object(&reread, 0)));
}

#[test]
fn a_reference_from_another_owner_should_be_stale() {
    let ops = ir(&[b"0 0 m S"]);
    let mut reference = ops.reference(object(&ops, 0)).unwrap();
    reference.owner = ContentOwner::Page("p4.0".parse().unwrap());
    assert_eq!(ops.find(&reference), None);
}

#[test]
fn hit_testing_should_list_the_topmost_object_first() {
    let mut ops = ir(&[b"0 0 m S 1 1 m S"]);
    place(&mut ops, 0, [0.0, 0.0, 100.0, 100.0]);
    place(&mut ops, 1, [50.0, 50.0, 10.0, 10.0]);
    let ids = [object(&ops, 1), object(&ops, 0)];
    assert_eq!(ops.hit_test(PointPt::from_points(55.0, 55.0)), ids);
}

#[test]
fn hit_testing_should_miss_an_object_whose_box_is_elsewhere() {
    let mut ops = ir(&[b"0 0 m S 1 1 m S"]);
    place(&mut ops, 0, [0.0, 0.0, 100.0, 100.0]);
    place(&mut ops, 1, [50.0, 50.0, 10.0, 10.0]);
    assert_eq!(ops.hit_test(PointPt::from_points(5.0, 5.0)), [object(&ops, 0)]);
}

#[test]
fn hit_testing_a_point_should_reach_a_hairline_close_by() {
    let mut ops = ir(&[b"0 0 m S"]);
    place(&mut ops, 0, [10.0, 20.0, 80.0, 0.0]);
    assert_eq!(ops.hit_test(PointPt::from_points(50.0, 21.5)), [object(&ops, 0)]);
}

#[test]
fn hit_testing_a_rectangle_should_list_the_objects_it_meets() {
    let mut ops = ir(&[b"0 0 m S 1 1 m S"]);
    place(&mut ops, 0, [0.0, 0.0, 10.0, 10.0]);
    place(&mut ops, 1, [50.0, 50.0, 10.0, 10.0]);
    let hits = ops.hit_test_rect(&RectPt::from_points(5.0, 5.0, 15.0, 15.0));
    assert_eq!(hits, [object(&ops, 0)]);
}

#[test]
fn too_many_operators_should_exceed_the_limit() {
    let limits = Limits::default().with_max_content_operators(2);
    let result = ContentOps::from_parts(owner(), vec![(None, b"q Q q".to_vec())], &limits);
    assert!(matches!(
        result,
        Err(Error::LimitExceeded {
            limit: Limit::ContentOperators,
            ..
        })
    ));
}

#[test]
fn the_source_hash_should_be_fnv1a() {
    // The published FNV-1a 64 test vectors for "" and "a".
    assert_eq!(
        (source_hash(b""), source_hash(b"a")),
        (0xcbf2_9ce4_8422_2325, 0xaf63_dc4c_8601_ec8c)
    );
}

#[test]
fn every_reason_should_have_its_name() {
    let names = [
        NotEditable::Type3Font,
        NotEditable::VerticalText,
        NotEditable::NoUnicodeMap,
        NotEditable::RotatedRun,
        NotEditable::GlyphsOutOfOrder,
        NotEditable::ActualText,
        NotEditable::RightToLeft,
        NotEditable::TooDeep,
        NotEditable::ContentUnreadable,
    ]
    .map(NotEditable::as_str);
    assert_eq!(
        names,
        [
            "type3-font",
            "vertical-text",
            "no-unicode-map",
            "rotated-run",
            "glyphs-out-of-order",
            "actual-text",
            "right-to-left",
            "too-deep",
            "content-unreadable"
        ]
    );
}

#[test]
fn an_object_left_empty_at_the_end_should_take_new_operators_again() {
    let mut ops = ir(&[b"0 0 m S BT (a) Tj ET"]);
    let text = object(&ops, 1);
    assert!(ops.replace(text, Vec::new()));
    assert!(ops.replace(text, vec![ContentOp::new(Operator::EndPath, Vec::new())]));
    let encoded = ops.encode();
    assert_eq!(encoded.parts[0].bytes(), Some(&b"0 0 m S n\n"[..]));
}

#[test]
fn an_object_replaced_twice_should_keep_only_the_second_operators() {
    let mut ops = ir(&[b"0 0 m S BT (a) Tj ET"]);
    let path = object(&ops, 0);
    assert!(ops.replace(path, vec![ContentOp::new(Operator::EndPath, Vec::new())]));
    assert!(ops.replace(path, vec![ContentOp::new(Operator::Fill, Vec::new())]));
    assert_eq!(written(&ops), b"f\n BT (a) Tj ET");
}

#[test]
fn removing_an_object_should_keep_the_end_of_a_sequence_begun_before_it() {
    let mut ops = ir(&[b"/P <</MCID 0>> BDC BT (a) Tj EMC ET 0 0 m S"]);
    assert!(ops.remove(object(&ops, 0)));
    assert_eq!(written(&ops), b"/P <</MCID 0>> BDC EMC\n 0 0 m S");
}

#[test]
fn a_sequence_reaching_outside_a_removed_object_should_stay_in_the_ir() {
    let mut ops = ir(&[b"/P <</MCID 0>> BDC BT (a) Tj EMC ET 0 0 m S"]);
    assert!(ops.remove(object(&ops, 0)));
    let seq = &ops.marked()[0];
    assert_eq!((seq.begin, seq.end), (0, Some(1)));
}

#[test]
fn replacing_an_object_should_write_a_sequence_it_opens_after_the_new_operators() {
    let mut ops = ir(&[b"BT /P <</MCID 2>> BDC (a) Tj ET EMC"]);
    let text = object(&ops, 0);
    let replacement = vec![
        ContentOp::new(Operator::BeginText, Vec::new()),
        ContentOp::new(Operator::EndText, Vec::new()),
    ];
    assert!(ops.replace(text, replacement));
    assert_eq!(written(&ops), b"BT\nET\n/P << /MCID 2 >> BDC\n EMC");
}

#[test]
fn an_object_left_empty_should_hold_the_operators_it_takes_again() {
    let mut ops = ir(&[b"0 0 m S BT (a) Tj ET"]);
    let path = object(&ops, 0);
    assert!(ops.replace(path, Vec::new()));
    assert!(ops.replace(path, vec![ContentOp::new(Operator::EndPath, Vec::new())]));
    assert_eq!(ops.object(path).map(ContentObject::operators), Some(0..1));
}
