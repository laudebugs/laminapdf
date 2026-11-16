use laminapdf_cos::Limits;
use laminapdf_model::{Matrix, PageFrame, RectPt, Rotation, UserSpaceRect};
use laminapdf_syntax::Name;

use super::super::{ContentObject, ContentObjectKind, ContentOps, ContentOwner, EncodedPart, NotEditable};
use super::{Placement, ResourceName};
use crate::state::GraphicsState;

/// A US Letter page, unturned: page space is user space with y flipped.
fn frame() -> PageFrame {
    PageFrame::new(UserSpaceRect::from_corners(0.0, 0.0, 612.0, 792.0), Rotation::None, 1.0)
}

/// An IR whose objects all start and end in `state`, as a recorder would leave them on content
/// that sets no state between objects.
fn ir_in(content: &[u8], state: &GraphicsState) -> ContentOps {
    let owner = ContentOwner::Page("p3.0".parse().unwrap());
    let mut ops = ContentOps::from_parts(owner, vec![(None, content.to_vec())], &Limits::default()).unwrap();
    for object in &mut ops.objects {
        object.state = Some(Box::new(state.clone()));
        object.end_state = Some(Box::new(state.clone()));
    }
    ops
}

fn ir(content: &[u8]) -> ContentOps {
    ir_in(content, &GraphicsState::default())
}

fn written(ops: &ContentOps) -> String {
    let encoded = ops.encode_every_part();
    let parts: Vec<&[u8]> = encoded.parts.iter().filter_map(EncodedPart::bytes).collect();
    String::from_utf8(parts.join(&b'\n')).unwrap()
}

fn id(ops: &ContentOps, ordinal: usize) -> super::ContentObjectId {
    ops.objects()[ordinal].id()
}

fn kinds(ops: &ContentOps) -> Vec<ContentObjectKind> {
    ops.objects().iter().map(ContentObject::kind).collect()
}

#[test]
fn moving_an_object_should_wrap_it_in_a_cm_in_user_space() {
    let mut ops = ir(b"0 0 10 10 re f");
    let first = id(&ops, 0);
    // 5 points right and 7 down on the page: 5 right and 7 down in user space too (y is flipped).
    ops.transform(first, &Matrix::translate(5.0, 7.0), &frame()).unwrap();
    assert_eq!(written(&ops), "q\n1 0 0 1 5 -7 cm\n0 0 10 10 re\nf\nQ\n");
}

#[test]
fn moving_an_object_should_account_for_the_transformation_it_draws_in() {
    let state = GraphicsState {
        ctm: Matrix::scale(2.0, 2.0),
        ..GraphicsState::default()
    };
    let mut ops = ir_in(b"0 0 10 10 re f", &state);
    ops.transform(id(&ops, 0), &Matrix::translate(10.0, 0.0), &frame())
        .unwrap();
    // In the object's own space, which is scaled 2×, 10 points is 5 units.
    assert!(written(&ops).starts_with("q\n1 0 0 1 5 0 cm\n"), "{}", written(&ops));
}

#[test]
fn a_text_object_should_leave_its_state_behind_when_moved_or_deleted() {
    let mut moved = ir(b"BT /F1 9 Tf 1 0 0 rg (a) Tj ET BT (b) Tj ET");
    moved
        .transform(id(&moved, 0), &Matrix::translate(1.0, 0.0), &frame())
        .unwrap();
    assert!(
        written(&moved).contains("ET\nQ\n/F1 9 Tf\n1 0 0 rg\n"),
        "{}",
        written(&moved)
    );
    let mut deleted = ir(b"BT /F1 9 Tf 1 0 0 rg (a) Tj ET BT (b) Tj ET");
    deleted.delete(id(&deleted, 0)).unwrap();
    assert_eq!(written(&deleted), "/F1 9 Tf\n1 0 0 rg\n BT (b) Tj ET");
    assert_eq!(kinds(&deleted), [ContentObjectKind::Text]);
}

#[test]
fn an_object_that_clips_should_be_refused() {
    let mut path = ir(b"0 0 10 10 re W n 0 0 5 5 re f");
    assert_eq!(path.delete(id(&path, 0)), Err(NotEditable::ClippingPath));
    let mut text = ir(b"BT 7 Tr (a) Tj ET");
    assert_eq!(
        text.transform(id(&text, 0), &Matrix::IDENTITY, &frame()),
        Err(NotEditable::ClippingPath)
    );
}

#[test]
fn an_object_without_a_recorded_state_should_be_refused() {
    let owner = ContentOwner::Page("p3.0".parse().unwrap());
    let mut ops = ContentOps::from_parts(owner, vec![(None, b"0 0 1 1 re f".to_vec())], &Limits::default()).unwrap();
    let first = id(&ops, 0);
    assert_eq!(
        ops.transform(first, &Matrix::IDENTITY, &frame()),
        Err(NotEditable::ContentUnreadable)
    );
}

#[test]
fn cropping_an_image_should_clip_it_to_the_rectangle_in_its_own_space() {
    let mut ops = ir(b"/Im1 Do");
    ops.objects[0].kind = ContentObjectKind::Image;
    // The top 92 points of a Letter page are user space y from 700 to 792.
    let rect = RectPt::from_points(0.0, 0.0, 100.0, 92.0);
    ops.crop(id(&ops, 0), &rect, &frame()).unwrap();
    assert_eq!(
        written(&ops),
        "q\n0 700 m\n100 700 l\n100 792 l\n0 792 l\nh\nW\nn\n/Im1 Do\nQ\n"
    );
}

#[test]
fn cropping_a_path_should_be_refused() {
    let mut ops = ir(b"0 0 1 1 re f");
    let rect = RectPt::from_points(0.0, 0.0, 1.0, 1.0);
    assert_eq!(ops.crop(id(&ops, 0), &rect, &frame()), Err(NotEditable::ClippingPath));
}

#[test]
fn replacing_an_image_should_draw_the_new_xobject_through_the_unit_matrix() {
    let mut plain = ir(b"q 100 0 0 50 0 0 cm /Im1 Do Q");
    plain.objects[0].kind = ContentObjectKind::Image;
    plain
        .replace_image(id(&plain, 0), Name::from("Im2"), &Matrix::IDENTITY)
        .unwrap();
    assert_eq!(written(&plain), "q 100 0 0 50 0 0 cm /Im2 Do\n Q");
    let mut turned = ir(b"/Im1 Do");
    turned.objects[0].kind = ContentObjectKind::Image;
    turned
        .replace_image(
            id(&turned, 0),
            Name::from("Im2"),
            &Matrix::new([0.0, 1.0, -1.0, 0.0, 1.0, 0.0]),
        )
        .unwrap();
    assert_eq!(written(&turned), "q\n0 1 -1 0 1 0 cm\n/Im2 Do\nQ\n");
}

#[test]
fn replacing_an_image_after_moving_it_should_keep_the_move() {
    let mut ops = ir(b"/Im1 Do");
    ops.objects[0].kind = ContentObjectKind::Image;
    let image = id(&ops, 0);
    ops.transform(image, &Matrix::translate(5.0, 0.0), &frame()).unwrap();
    ops.replace_image(image, Name::from("Im2"), &Matrix::IDENTITY).unwrap();
    let out = written(&ops);
    assert!(out.contains("1 0 0 1 5 0 cm") && out.contains("/Im2 Do"), "{out}");
    assert!(!out.contains("/Im1"), "{out}");
}

#[test]
fn moving_an_object_after_reordering_it_should_leave_no_state_behind_at_its_new_place() {
    let mut ops = ir(b"1 0 0 rg 0 0 1 1 re f 0 0 1 rg 2 2 1 1 re f");
    let blue = id(&ops, 1);
    ops.reorder(blue, Placement::Back).unwrap();
    ops.transform(blue, &Matrix::translate(5.0, 0.0), &frame()).unwrap();
    let out = written(&ops);
    // The blue square's colour stays inside its own `q … Q`, so the red square stays red: the
    // last fill colour set outside every `q … Q` before it is red's.
    let red = out.find("0 0 1 1 re").unwrap();
    let (mut depth, mut fill) = (0_usize, Vec::new());
    let mut operands = Vec::new();
    for token in out[..red].split_whitespace() {
        match token {
            "q" => depth += 1,
            "Q" => depth -= 1,
            "rg" | "scn" | "sc" | "g" | "k" if depth == 0 => fill = std::mem::take(&mut operands),
            _ if token.parse::<f64>().is_ok() => {
                operands.push(token.to_owned());
                continue;
            }
            _ => {}
        }
        operands.clear();
    }
    assert_eq!(fill, ["1", "0", "0"], "{out}");
}

#[test]
fn replacing_something_other_than_an_image_should_be_refused() {
    let mut ops = ir(b"0 0 1 1 re f");
    assert_eq!(
        ops.replace_image(id(&ops, 0), Name::from("Im2"), &Matrix::IDENTITY),
        Err(NotEditable::ContentUnreadable)
    );
}

#[test]
fn bringing_an_object_to_the_front_should_paint_it_last_with_its_state() {
    let mut ops = ir(b"0 0 1 1 re f 2 2 1 1 re f 4 4 1 1 re f");
    let first = id(&ops, 0);
    ops.reorder(first, Placement::Front).unwrap();
    let out = written(&ops);
    assert!(out.starts_with(" 2 2 1 1 re f 4 4 1 1 re f"), "{out}");
    assert!(out.contains("\nq\n1 0 0 1 0 0 cm\n1 w\n"), "{out}");
    assert!(out.ends_with("0 0 1 1 re\nf\nQ\n"), "{out}");
    assert_eq!(ops.objects().last().map(ContentObject::id), Some(first));
}

#[test]
fn sending_an_object_to_the_back_should_paint_it_first() {
    let mut ops = ir(b"0 0 1 1 re f 2 2 1 1 re f");
    let last = id(&ops, 1);
    ops.reorder(last, Placement::Back).unwrap();
    assert!(written(&ops).starts_with("q\n"), "{}", written(&ops));
    assert_eq!(ops.objects().first().map(ContentObject::id), Some(last));
}

#[test]
fn reordering_should_correct_for_a_different_transformation_where_it_lands() {
    let mut ops = ir(b"0 0 1 1 re f 2 2 1 1 re f");
    ops.objects[1].end_state = Some(Box::new(GraphicsState {
        ctm: Matrix::translate(10.0, 0.0),
        ..GraphicsState::default()
    }));
    ops.reorder(id(&ops, 0), Placement::Front).unwrap();
    assert!(written(&ops).contains("q\n1 0 0 1 -10 0 cm\n"), "{}", written(&ops));
}

#[test]
fn reordering_where_the_clip_differs_should_be_refused() {
    let mut ops = ir(b"0 0 1 1 re f 2 2 1 1 re f");
    ops.objects[1].end_state = Some(Box::new(GraphicsState {
        clip_box: Some(UserSpaceRect::from_corners(0.0, 0.0, 5.0, 5.0)),
        ..GraphicsState::default()
    }));
    assert_eq!(
        ops.reorder(id(&ops, 0), Placement::Front),
        Err(NotEditable::StateNotPortable)
    );
}

#[test]
fn reordering_next_to_itself_should_change_nothing() {
    let mut ops = ir(b"0 0 1 1 re f 2 2 1 1 re f");
    let (first, second) = (id(&ops, 0), id(&ops, 1));
    ops.reorder(first, Placement::Before(second)).unwrap();
    ops.reorder(first, Placement::Back).unwrap();
    assert!(ops.encode().parts.iter().all(|p| p.bytes().is_none()));
}

#[test]
fn a_reordered_object_should_take_marked_content_that_holds_only_it() {
    let mut ops = ir(b"/P <</MCID 0>> BDC 0 0 1 1 re f EMC 2 2 1 1 re f");
    ops.reorder(id(&ops, 0), Placement::Front).unwrap();
    let out = written(&ops);
    assert!(out.starts_with(" 2 2 1 1 re f"), "{out}");
    assert!(out.contains("/P << /MCID 0 >> BDC\nq\n"), "{out}");
    assert!(out.ends_with("Q\nEMC\n"), "{out}");
}

#[test]
fn ids_should_find_their_objects_after_a_reorder() {
    let mut ops = ir(b"0 0 1 1 re f 2 2 1 1 re f 4 4 1 1 re f");
    let (first, third) = (id(&ops, 0), id(&ops, 2));
    ops.reorder(third, Placement::Back).unwrap();
    ops.delete(first).unwrap();
    assert!(ops.object(first).is_none());
    assert!(ops.object(third).is_some());
    assert_eq!(ops.objects().len(), 2);
}

#[test]
fn resource_names_should_follow_the_operators_that_name_them() {
    let ops = ir(b"/GS1 gs /Pattern cs /P1 scn BT /F1 9 Tf (a) Tj ET /Im1 Do /Sh1 sh /OC <</MCID 1>> BDC EMC");
    let names: Vec<(&str, String)> = ops
        .resource_names()
        .into_iter()
        .map(|ResourceName { category, name }| (category, String::from_utf8_lossy(name.as_bytes()).into_owned()))
        .collect();
    let mut expected = vec![
        ("ExtGState", "GS1".to_owned()),
        ("ColorSpace", "Pattern".to_owned()),
        ("Pattern", "P1".to_owned()),
        ("Font", "F1".to_owned()),
        ("XObject", "Im1".to_owned()),
        ("Shading", "Sh1".to_owned()),
    ];
    expected.sort();
    let mut names = names;
    names.sort();
    assert_eq!(names, expected);
    let first = id(&ops, 0);
    assert!(ops.object_resource_names(first).iter().all(|n| n.category != "XObject"));
}

#[test]
fn the_object_a_moved_one_lands_after_should_keep_only_its_own_operators() {
    let mut ops = ir(b"0 0 1 1 re f 2 2 1 1 re f 4 4 1 1 re f");
    let (first, last) = (id(&ops, 0), id(&ops, 2));
    ops.reorder(first, Placement::Front).unwrap();
    // The anchor, the old last object, still ends where it did: deleting it leaves the moved one.
    ops.delete(last).unwrap();
    let out = written(&ops);
    assert!(out.contains("0 0 1 1 re") && !out.contains("4 4 1 1 re"), "{out}");
}

#[test]
fn a_moved_object_should_land_outside_the_anchors_marked_content() {
    let mut ops = ir(b"/P <</MCID 0>> BDC 0 0 1 1 re f EMC /P <</MCID 1>> BDC 2 2 1 1 re f EMC");
    ops.reorder(id(&ops, 0), Placement::Front).unwrap();
    let out = written(&ops);
    let second_end = out.find("EMC").unwrap();
    let first_begin = out.find("/MCID 0").unwrap();
    assert!(
        out.find("/MCID 1").unwrap() < second_end && second_end < first_begin,
        "{out}"
    );
}

#[test]
fn moving_an_object_after_reordering_it_should_use_its_new_place() {
    let here = GraphicsState {
        ctm: Matrix::translate(100.0, 0.0),
        ..GraphicsState::default()
    };
    let mut ops = ir_in(
        b"q 1 0 0 1 100 0 cm 0 0 1 1 re f Q 2 2 1 1 re f",
        &GraphicsState::default(),
    );
    ops.objects[0].state = Some(Box::new(here));
    let moved = id(&ops, 0);
    ops.reorder(moved, Placement::Front).unwrap();
    assert_eq!(
        ops.object(moved).and_then(ContentObject::state).map(|s| s.ctm),
        Some(Matrix::IDENTITY)
    );
    ops.transform(moved, &Matrix::translate(5.0, 0.0), &frame()).unwrap();
    // Wrapped twice: the move by 5 in its new place's space, then the reorder's 100.
    let out = written(&ops);
    assert!(
        out.contains("1 0 0 1 5 0 cm") && out.contains("1 0 0 1 100 0 cm"),
        "{out}"
    );
}

#[test]
fn reordering_by_an_object_an_earlier_edit_deleted_should_be_refused() {
    let mut ops = ir(b"0 0 1 1 re f 2 2 1 1 re f 4 4 1 1 re f");
    let (first, second) = (id(&ops, 0), id(&ops, 1));
    ops.delete(second).unwrap();
    assert_eq!(
        ops.reorder(first, Placement::Before(second)),
        Err(NotEditable::ContentUnreadable)
    );
}

#[test]
fn nothing_should_be_placed_after_a_clipping_path() {
    let mut ops = ir(b"0 0 50 50 re W n 2 2 1 1 re f 4 4 1 1 re f");
    let (clip, last) = (id(&ops, 0), id(&ops, 2));
    assert_eq!(
        ops.reorder(last, Placement::After(clip)),
        Err(NotEditable::StateNotPortable)
    );
}

#[test]
fn an_inline_image_should_be_replaced_by_the_new_xobject() {
    let mut ops = ir(b"q 10 0 0 10 0 0 cm BI /W 1 /H 1 /CS /G /BPC 8 ID \x80 EI Q");
    ops.objects[0].kind = ContentObjectKind::Image;
    ops.replace_image(id(&ops, 0), Name::from("Im2"), &Matrix::IDENTITY)
        .unwrap();
    let out = written(&ops);
    assert!(out.contains("/Im2 Do") && !out.contains("BI"), "{out}");
}

#[test]
fn text_state_set_by_td_and_the_quote_operator_should_stay_behind() {
    let mut ops = ir(b"BT /F1 12 Tf 0 -14 TD 1 2 (a) \" ET BT (b) Tj T* (c) Tj ET");
    ops.delete(id(&ops, 0)).unwrap();
    let out = written(&ops);
    assert!(
        out.contains("14 TL") && out.contains("1 Tw") && out.contains("2 Tc"),
        "{out}"
    );
}

#[test]
fn an_inline_images_colour_space_should_count_as_named() {
    let ops = ir(b"BI /W 1 /H 1 /CS /CS0 /BPC 8 ID \x80 EI BI /W 1 /H 1 /CS /RGB /BPC 8 ID \x80\x80\x80 EI");
    let names: Vec<String> = ops
        .resource_names()
        .iter()
        .map(|n| format!("{}/{}", n.category, String::from_utf8_lossy(n.name.as_bytes())))
        .collect();
    assert_eq!(names, ["ColorSpace/CS0"]);
}
