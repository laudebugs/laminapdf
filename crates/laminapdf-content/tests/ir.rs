//! The editable content IR (ADR 0022) on built pages and on every fixture: grouping, the
//! recorder's states, kinds and boxes, and the encoder copying every unedited byte.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{dict, font_resources, form, page_doc};
use laminapdf_content::{ContentObject, ContentObjectKind, ContentOps, ContentOwner, EncodedPart, interpret_page};
use laminapdf_cos::{Object, Stream};
use laminapdf_model::{Document, OpenOptions, PointPt, RectPt};
use laminapdf_test_fixtures::fixtures_dir;

/// The first page's IR, recorded by the interpreter.
fn recorded(doc: &Document) -> ContentOps {
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let owner = ContentOwner::Page(page.id().clone());
    let mut ops = ContentOps::read_page(doc, page.cos(), owner).unwrap();
    interpret_page(doc, &page, &mut ops.recorder(page.frame().unwrap())).unwrap();
    ops
}

/// A page with an image XObject `/Im1`, a form `/Fm1` and the font `/F1`.
fn page_with(content: &[u8]) -> Document {
    page_doc(|w| {
        let image = w.add(Stream::new(
            dict([
                ("Type", Object::name("XObject")),
                ("Subtype", Object::name("Image")),
                ("Width", 1.into()),
                ("Height", 1.into()),
                ("BitsPerComponent", 8.into()),
                ("ColorSpace", Object::name("DeviceGray")),
            ]),
            vec![0],
        ));
        let form = w.add(form(b"0 0 10 10 re f", laminapdf_cos::Dict::new()));
        let mut resources = font_resources(w);
        resources.insert(
            "XObject",
            Object::from(dict([("Im1", Object::from(image)), ("Fm1", Object::from(form))])),
        );
        (content.to_vec(), resources)
    })
}

fn bounds(ops: &ContentOps, ordinal: usize) -> RectPt {
    ops.objects()[ordinal].bounds().unwrap()
}

/// `(left, top, right, bottom)` in page space, rounded to a hundredth of a point.
fn corners(rect: RectPt) -> [f64; 4] {
    [rect.x0, rect.y0, rect.x1, rect.y1].map(|v| (v.get() * 100.0).round() / 100.0)
}

#[test]
fn the_recorder_should_tell_an_image_from_a_form() {
    let ops = recorded(&page_with(b"/Im1 Do /Fm1 Do /Missing Do"));
    let kinds: Vec<_> = ops.objects().iter().map(ContentObject::kind).collect();
    assert_eq!(
        kinds,
        [
            ContentObjectKind::Image,
            ContentObjectKind::Form,
            ContentObjectKind::XObject
        ]
    );
}

#[test]
fn a_filled_rectangle_should_be_boxed_in_page_space() {
    // US Letter: user space y 10..60 is page space y 732..782.
    let ops = recorded(&page_with(b"10 10 100 50 re f"));
    assert_eq!(corners(bounds(&ops, 0)), [10.0, 732.0, 110.0, 782.0]);
}

#[test]
fn a_stroked_line_should_be_boxed_with_half_its_width() {
    let ops = recorded(&page_with(b"4 w 10 10 m 110 10 l S"));
    assert_eq!(corners(bounds(&ops, 0)), [8.0, 780.0, 112.0, 784.0]);
}

#[test]
fn an_image_should_be_boxed_by_the_unit_square_it_fills() {
    let ops = recorded(&page_with(b"q 50 0 0 20 100 200 cm /Im1 Do Q"));
    assert_eq!(corners(bounds(&ops, 0)), [100.0, 572.0, 150.0, 592.0]);
}

#[test]
fn a_form_should_be_boxed_by_what_it_paints() {
    let ops = recorded(&page_with(b"q 1 0 0 1 300 400 cm /Fm1 Do Q"));
    assert_eq!(corners(bounds(&ops, 0)), [300.0, 382.0, 310.0, 392.0]);
}

#[test]
fn an_object_should_keep_the_state_before_it() {
    let ops = recorded(&page_with(b"1 0 0 rg 0 0 5 5 re f 0 g 5 5 5 5 re f"));
    let fill = |i: usize| ops.objects()[i].state().unwrap().fill.components.clone();
    assert_eq!((fill(0), fill(1)), (vec![1.0, 0.0, 0.0], vec![0.0]));
}

#[test]
fn text_should_be_boxed_where_its_line_starts() {
    // On its own the recorder measures no glyphs: the box has the line's height and no width.
    let ops = recorded(&page_with(b"BT /F1 10 Tf 72 700 Td (Hi) Tj ET"));
    assert_eq!(corners(bounds(&ops, 0)), [72.0, 84.0, 72.0, 94.0]);
}

#[test]
fn hit_testing_should_find_the_object_under_a_point() {
    let ops = recorded(&page_with(b"10 10 100 50 re f q 50 0 0 20 30 30 cm /Im1 Do Q"));
    let hits = ops.hit_test(PointPt::from_points(40.0, 755.0));
    let ids: Vec<_> = ops.objects().iter().rev().map(ContentObject::id).collect();
    assert_eq!(hits, ids);
}

#[test]
fn an_edit_should_leave_the_recorded_states_of_other_objects() {
    let mut ops = recorded(&page_with(b"1 0 0 rg 0 0 5 5 re f 0 g 5 5 5 5 re f"));
    let first = ops.objects()[0].id();
    assert!(ops.remove(first));
    assert_eq!(ops.objects()[0].state().unwrap().fill.components, [0.0]);
}

/// Every PDF in the fixture folders the corpus tests read.
fn fixture_files() -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for dir in ["corpus", "golden", "golden/examples", "content"] {
        for entry in std::fs::read_dir(fixtures_dir().join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|x| x == "pdf") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn every_fixture_page_should_be_written_back_byte_for_byte() {
    let mut pages = 0;
    for path in fixture_files() {
        let Ok(doc) = Document::open(std::fs::read(&path).unwrap(), OpenOptions::default()) else {
            continue;
        };
        let Ok(list) = doc.pages() else { continue };
        for page in list.iter() {
            let owner = ContentOwner::Page(page.id().clone());
            let Ok(mut ops) = ContentOps::read_page(&doc, page.cos(), owner) else {
                continue;
            };
            pages += 1;
            let name = path.display();
            assert_eq!(
                Some(ops.source()),
                doc.page_content(page.cos()).ok().as_deref(),
                "{name}: the IR reads the content the interpreter reads"
            );
            let encoded = ops.encode_every_part();
            for (part, written) in ops.parts().iter().zip(&encoded.parts) {
                assert_eq!(
                    written.bytes(),
                    ops.source().get(part.start..part.end),
                    "{name}: part {}",
                    part.contents_index
                );
            }
            assert!(encoded.parts.iter().all(|p| matches!(p, EncodedPart::Written { .. })));
            // Recording reads the same content and must not trip over it.
            if let Ok(frame) = page.frame() {
                let _ = interpret_page(&doc, &page, &mut ops.recorder(frame));
            }
        }
    }
    assert!(pages > 100, "only {pages} fixture pages were read");
}

#[test]
fn every_saved_crash_input_of_the_ir_target_replays_without_a_panic() {
    let Ok(files) = std::fs::read_dir(fixtures_dir().join("regressions/content_ir")) else {
        return;
    };
    for file in files {
        common::ir_fuzz_entry::content_ir(&std::fs::read(file.unwrap().path()).unwrap());
    }
}

#[test]
fn the_ir_fuzz_entry_should_take_content_in_several_parts() {
    common::ir_fuzz_entry::content_ir(
        b"\x03q 1 0 0 RG 0 0 m 5 5 l S BT /F1 9 Tf (a) Tj ET /P <</MCID 0>> BDC /Im1 Do EMC BI /W 1 /H 1 /BPC 8 /CS /G ID \x00 EI Q",
    );
}

#[test]
fn a_recorder_should_record_nothing_once_the_ir_is_edited() {
    let doc = page_with(b"1 0 0 rg 0 0 5 5 re f 0 g 5 5 5 5 re f");
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let mut ops = ContentOps::read_page(&doc, page.cos(), ContentOwner::Page(page.id().clone())).unwrap();
    let first = ops.objects()[0].id();
    assert!(ops.remove(first));
    interpret_page(&doc, &page, &mut ops.recorder(page.frame().unwrap())).unwrap();
    assert!(
        ops.objects()
            .iter()
            .all(|o| o.state().is_none() && o.bounds().is_none())
    );
}
