//! `EditContent` (todo E7d, ADR 0022): moving, deleting, replacing, cropping and reordering
//! the objects a page already draws, through `laminapdf-content`'s IR.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{classic, classic_with_root, fixture, open, stream_body};
use laminapdf_content::{ContentObjectKind, ContentOps, ContentOwner, NotEditable, interpret_page};
use laminapdf_cos::Object;
use laminapdf_model::{Matrix, PageId, RectPt};
use laminapdf_ops::{ContentEdit, Document, EditContent, Error, ObjectRef, Op, Origin, StackPosition};
use laminapdf_save::{Save, SaveOptions};

/// The content both pages draw: a blue square, an image and a line of red text.
const CONTENT: &[u8] =
    b"0 0 1 rg 10 10 50 50 re f\nq 40 0 0 40 100 100 cm /Im1 Do Q\nBT /F1 12 Tf 1 0 0 rg 20 150 Td (Hi) Tj ET\n";

/// Two 200 × 200 pages that share one content stream (object 5), each with its own resources
/// naming the image (6) and the font (7).
fn shared() -> Document {
    let content = stream_body("", CONTENT);
    let image = stream_body(
        "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",
        b"\x80",
    );
    let resources = "/Resources << /XObject << /Im1 6 0 R >> /Font << /F1 7 0 R >> >>";
    let page = format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 5 0 R {resources} >>");
    let objects: [&[u8]; 7] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        page.as_bytes(),
        page.as_bytes(),
        &content,
        &image,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ];
    open(&classic(&objects, "<< /Size 8 /Root 1 0 R >>"))
}

fn page(doc: &Document, index: usize) -> PageId {
    doc.pages().unwrap().get(index).unwrap().id().clone()
}

/// The page's objects as an editor lists them: kind, lasting name and box.
fn objects(doc: &Document, index: usize) -> Vec<(ContentObjectKind, ObjectRef, Option<RectPt>)> {
    let pages = doc.pages().unwrap();
    let page = pages.get(index).unwrap();
    let mut ir = ContentOps::read_page(doc, page.cos(), ContentOwner::Page(page.id().clone())).unwrap();
    interpret_page(doc, &page, &mut ir.recorder(page.frame().unwrap())).unwrap();
    ir.objects()
        .iter()
        .map(|o| (o.kind(), ObjectRef::from(&ir.reference(o.id()).unwrap()), o.bounds()))
        .collect()
}

fn kinds(doc: &Document, index: usize) -> Vec<ContentObjectKind> {
    objects(doc, index).into_iter().map(|(kind, ..)| kind).collect()
}

fn object(doc: &Document, index: usize, ordinal: usize) -> ObjectRef {
    objects(doc, index)[ordinal].1
}

/// Applies the edits `edits` makes, from the page as it is, to page `index`.
fn edit(
    doc: &mut Document,
    index: usize,
    edits: impl FnOnce(&Document) -> Vec<ContentEdit>,
) -> laminapdf_ops::Result<()> {
    let op = EditContent::new(page(doc, index), edits(doc));
    doc.apply_all(Origin::Local, [Op::from(op)]).map(drop)
}

fn content_of(doc: &Document, index: usize) -> Vec<u8> {
    let pages = doc.pages().unwrap();
    doc.page_content(pages.get(index).unwrap().cos()).unwrap()
}

fn resources(doc: &Document, index: usize, category: &str) -> Vec<String> {
    let pages = doc.pages().unwrap();
    let Some(dict) = pages.get(index).unwrap().cos().resources_dict().cloned() else {
        return Vec::new();
    };
    match dict.get(category).map(|o| doc.deref(o).unwrap()) {
        Some(sub) => match &*sub {
            Object::Dict(d) => d
                .iter()
                .map(|(k, _)| String::from_utf8_lossy(k.as_bytes()).into_owned())
                .collect(),
            _ => Vec::new(),
        },
        None => Vec::new(),
    }
}

#[test]
fn the_fixture_should_hold_a_path_an_image_and_text() {
    assert_eq!(
        kinds(&shared(), 0),
        [
            ContentObjectKind::Path,
            ContentObjectKind::Image,
            ContentObjectKind::Text
        ]
    );
}

#[test]
fn moving_an_object_should_move_its_box_by_the_matrix() {
    let mut doc = shared();
    let before = objects(&doc, 0)[0].2.unwrap();
    edit(&mut doc, 0, |doc| {
        vec![ContentEdit::Transform {
            object: object(doc, 0, 0),
            matrix: Matrix::translate(5.0, 7.0),
        }]
    })
    .unwrap();
    let after = objects(&doc, 0)[0].2.unwrap();
    let moved = [after.x0.get() - before.x0.get(), after.y0.get() - before.y0.get()];
    assert!(
        (moved[0] - 5.0).abs() < 1e-6 && (moved[1] - 7.0).abs() < 1e-6,
        "{moved:?}"
    );
}

#[test]
fn an_edit_should_leave_a_shared_stream_as_it_is_on_the_other_page() {
    let mut doc = shared();
    edit(&mut doc, 0, |doc| {
        vec![ContentEdit::Delete {
            object: object(doc, 0, 0),
        }]
    })
    .unwrap();
    assert_eq!(content_of(&doc, 1), CONTENT);
    assert_ne!(content_of(&doc, 0), CONTENT);
    assert_eq!(kinds(&doc, 1).len(), 3);
}

#[test]
fn deleting_an_image_should_drop_the_resource_nothing_names_any_more() {
    let mut doc = shared();
    edit(&mut doc, 0, |doc| {
        vec![ContentEdit::Delete {
            object: object(doc, 0, 1),
        }]
    })
    .unwrap();
    assert_eq!(kinds(&doc, 0), [ContentObjectKind::Path, ContentObjectKind::Text]);
    assert_eq!(resources(&doc, 0, "XObject"), Vec::<String>::new());
    assert_eq!(resources(&doc, 0, "Font"), ["F1"]);
    assert_eq!(resources(&doc, 1, "XObject"), ["Im1"]);
}

#[test]
fn deleting_text_should_keep_the_state_it_left_for_later_content() {
    let mut doc = shared();
    edit(&mut doc, 0, |doc| {
        vec![ContentEdit::Delete {
            object: object(doc, 0, 2),
        }]
    })
    .unwrap();
    let content = String::from_utf8(content_of(&doc, 0)).unwrap();
    assert!(
        content.contains("/F1 12 Tf\n1 0 0 rg\n") && !content.contains("Tj"),
        "{content}"
    );
}

#[test]
fn replacing_an_image_should_draw_the_new_one_in_its_place() {
    let mut doc = shared().with_images(laminapdf_build::ImageXObjectWriter);
    let blob = doc.add_blob(fixture("images/photo-rgb.jpg"));
    edit(&mut doc, 0, |doc| {
        vec![ContentEdit::ReplaceImage {
            object: object(doc, 0, 1),
            blob,
        }]
    })
    .unwrap();
    assert_eq!(kinds(&doc, 0)[1], ContentObjectKind::Image);
    assert_eq!(resources(&doc, 0, "XObject"), ["Im2"]);
    assert_eq!(resources(&doc, 1, "XObject"), ["Im1"]);
}

#[test]
fn replacing_an_image_without_its_bytes_or_a_writer_should_fail() {
    let mut doc = shared();
    let blob = doc.add_blob(fixture("images/photo-rgb.jpg"));
    let image = object(&doc, 0, 1);
    let replace = |blob| vec![ContentEdit::ReplaceImage { object: image, blob }];
    assert_eq!(
        edit(&mut doc, 0, |_| replace(blob)).map_err(|e| e.code()),
        Err("images-unavailable")
    );
    let mut doc = shared().with_images(laminapdf_build::ImageXObjectWriter);
    let unknown = laminapdf_ops::BlobId::of(b"never added");
    assert_eq!(
        edit(&mut doc, 0, |_| replace(unknown)).map_err(|e| e.code()),
        Err("unknown-blob")
    );
}

#[test]
fn replacing_a_path_with_an_image_should_be_refused() {
    let mut doc = shared().with_images(laminapdf_build::ImageXObjectWriter);
    let blob = doc.add_blob(fixture("images/photo-rgb.jpg"));
    let path = object(&doc, 0, 0);
    let result = edit(&mut doc, 0, |_| vec![ContentEdit::ReplaceImage { object: path, blob }]);
    assert_eq!(result.map_err(|e| e.code()), Err("invalid-op"));
}

#[test]
fn cropping_an_image_should_clip_it() {
    let mut doc = shared();
    edit(&mut doc, 0, |doc| {
        vec![ContentEdit::Crop {
            object: object(doc, 0, 1),
            rect: RectPt::from_points(100.0, 70.0, 20.0, 20.0),
        }]
    })
    .unwrap();
    let content = String::from_utf8(content_of(&doc, 0)).unwrap();
    assert!(content.contains("h\nW\nn\n/Im1 Do\nQ\n"), "{content}");
}

#[test]
fn sending_text_to_the_back_should_paint_it_first() {
    let mut doc = shared();
    edit(&mut doc, 0, |doc| {
        vec![ContentEdit::Reorder {
            object: object(doc, 0, 2),
            to: StackPosition::Back,
        }]
    })
    .unwrap();
    assert_eq!(
        kinds(&doc, 0),
        [
            ContentObjectKind::Text,
            ContentObjectKind::Path,
            ContentObjectKind::Image
        ]
    );
}

#[test]
fn every_edit_of_an_op_should_name_its_object_as_the_page_was_before_the_op() {
    let mut doc = shared();
    let [path, image, text] = [0, 1, 2].map(|i| object(&doc, 0, i));
    edit(&mut doc, 0, |_| {
        vec![
            ContentEdit::Delete { object: path },
            ContentEdit::Reorder {
                object: text,
                to: StackPosition::Before(image),
            },
        ]
    })
    .unwrap();
    assert_eq!(kinds(&doc, 0), [ContentObjectKind::Text, ContentObjectKind::Image]);
}

#[test]
fn a_stale_name_should_be_refused() {
    let mut doc = shared();
    let stale = ObjectRef::new(0, object(&doc, 0, 0).hash ^ 1);
    let result = edit(&mut doc, 0, |_| vec![ContentEdit::Delete { object: stale }]);
    assert_eq!(result, Err(Error::ContentObjectStale { ordinal: 0 }));
    assert!(!doc.is_dirty());
}

#[test]
fn an_object_that_clips_should_be_refused() {
    let content = stream_body("", b"0 0 100 100 re W n 0 0 1 rg 10 10 50 50 re f");
    let mut doc = open(&classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>",
            std::str::from_utf8(&content).unwrap(),
        ],
        "",
    ));
    let result = edit(&mut doc, 0, |doc| {
        vec![ContentEdit::Delete {
            object: object(doc, 0, 0),
        }]
    });
    assert_eq!(
        result,
        Err(Error::ContentNotEditable {
            reason: NotEditable::ClippingPath
        })
    );
}

#[test]
fn an_op_with_no_edits_should_be_refused() {
    let mut doc = shared();
    assert_eq!(
        edit(&mut doc, 0, |_| Vec::new()).map_err(|e| e.code()),
        Err("invalid-op")
    );
}

#[test]
fn edit_content_should_be_class_content_and_save_incrementally() {
    let op = Op::from(EditContent::new("p3.0".parse().unwrap(), Vec::new()));
    assert_eq!(op.class(), laminapdf_ops::ModificationClass::Content);
    assert!(!op.requires_rewrite());
}

/// Three edits of one op: a scale, a crop and a reorder.
fn three_edits(doc: &Document) -> Vec<ContentEdit> {
    let [path, image, text] = [0, 1, 2].map(|i| object(doc, 0, i));
    vec![
        ContentEdit::Transform {
            object: path,
            matrix: Matrix::scale(2.0, 2.0),
        },
        ContentEdit::Crop {
            object: image,
            rect: RectPt::from_points(0.0, 0.0, 50.0, 50.0),
        },
        ContentEdit::Reorder {
            object: text,
            to: StackPosition::Back,
        },
    ]
}

fn saved(doc: &mut Document) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::default()).unwrap();
    out
}

#[test]
fn undoing_an_edit_should_leave_the_overlay_empty_and_the_save_unchanged() {
    let before = saved(&mut shared());
    let mut doc = shared();
    edit(&mut doc, 0, three_edits).unwrap();
    doc.undo().unwrap();
    assert!(!doc.is_dirty());
    assert_eq!(saved(&mut doc), before);
}

#[test]
fn redoing_an_edit_should_write_the_same_bytes_again() {
    let mut once = shared();
    edit(&mut once, 0, three_edits).unwrap();
    let mut redone = shared();
    edit(&mut redone, 0, three_edits).unwrap();
    redone.undo().unwrap();
    redone.redo().unwrap();
    assert_eq!(saved(&mut redone), saved(&mut once));
}

#[test]
fn a_deletion_should_say_the_earlier_revision_still_holds_it() {
    let mut doc = shared();
    let op = EditContent::new(
        page(&doc, 0),
        vec![ContentEdit::Delete {
            object: object(&doc, 0, 0),
        }],
    );
    let applied = doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    assert!(applied.still_in_earlier_revisions);
}

/// The frozen wire sample (`fixtures/ops/v1/edit-content.json`), written for [`shared`]'s first
/// page, applies: the square moves, the image is cropped and goes to the back, the text goes.
#[test]
fn the_frozen_sample_applies_to_the_page_it_was_written_for() {
    let mut doc = shared();
    let op: Op = serde_json::from_slice(&fixture("ops/v1/edit-content.json")).unwrap();
    let Op::EditContent(ref edit) = op else {
        panic!("not an editContent op: {op:?}");
    };
    let refs: Vec<ObjectRef> = objects(&doc, 0).into_iter().map(|(_, r, _)| r).collect();
    assert!(edit.edits.iter().all(|e| refs.contains(&e.object())), "{refs:?}");
    doc.apply_all(Origin::Local, [op]).unwrap();
    // Read again, the crop's clipping path is an object of its own, before the image.
    assert_eq!(
        kinds(&doc, 0),
        [
            ContentObjectKind::Path,
            ContentObjectKind::Image,
            ContentObjectKind::Path
        ]
    );
}

/// A form without resources of its own draws with the page's (§7.8.3), so deleting the page's
/// own use of an image the form also draws keeps the image in the resources.
#[test]
fn resources_a_form_borrows_from_the_page_should_stay() {
    let form = stream_body("/Type /XObject /Subtype /Form /BBox [0 0 1 1]", b"/Im1 Do");
    let image = stream_body(
        "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",
        b"\x80",
    );
    let content = stream_body("", b"q 40 0 0 40 0 0 cm /Fm1 Do Q\nq 40 0 0 40 100 100 cm /Im1 Do Q\n");
    let objects: [&[u8]; 6] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources << /XObject << /Fm1 5 0 R /Im1 6 0 R >> >> >>",
        &content,
        &form,
        &image,
    ];
    let mut doc = open(&classic(&objects, "<< /Size 7 /Root 1 0 R >>"));
    edit(&mut doc, 0, |doc| {
        vec![ContentEdit::Delete {
            object: object(doc, 0, 1),
        }]
    })
    .unwrap();
    assert_eq!(resources(&doc, 0, "XObject"), ["Fm1", "Im1"]);
}
