//! Page marks (E6 plan §2.3, todo E6d): watermarks, backgrounds, headers and footers, and Bates
//! numbers: what they write on each page, their fields, hideable groups, tagging, updating and
//! deleting them, undo and redo, and the wire form.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open};
use laminapdf_build::AnnotationAppearances;
use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{PageId, PdfDate};
use laminapdf_ops::{
    AddBackground, AddContentObject, AddText, AddWatermark, AnnotColor, Applied, ApplyBates, ContentId,
    ContentObjectSpec, DeleteContentObject, DeletePageMark, Document, MarkContent, MarkMargins, Op, Origin, PageMark,
    SetHeaderFooter, Slot, UpdateContentObject, UpdatePageMark, added_objects, page_marks,
};

fn doc(name: &str) -> Document {
    open(&fixture(name)).with_appearances(AnnotationAppearances)
}

trait Apply {
    fn applied(&mut self, op: impl Into<Op>) -> Applied;
    fn refused(&mut self, op: impl Into<Op>) -> &'static str;
}

impl Apply for Document {
    fn applied(&mut self, op: impl Into<Op>) -> Applied {
        self.apply_all(Origin::Local, [op.into()]).unwrap()
    }

    fn refused(&mut self, op: impl Into<Op>) -> &'static str {
        self.apply_all(Origin::Local, [op.into()]).unwrap_err().code()
    }
}

fn pages(doc: &Document) -> Vec<PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

fn id(s: &str) -> ContentId {
    ContentId::new(s).unwrap()
}

fn date() -> PdfDate {
    "D:20261118120000Z".parse().unwrap()
}

fn text(s: &str, size: f64) -> MarkContent {
    MarkContent::Text {
        text: s.to_owned(),
        fonts: Vec::new(),
        size,
        color: None,
    }
}

fn dict(doc: &Document, r: ObjRef) -> Dict {
    match &*doc.resolve(r).unwrap() {
        Object::Dict(d) => d.clone(),
        Object::Stream(s) => s.dict.clone(),
        other => panic!("{other:?}"),
    }
}

/// The page's `/Contents` streams, decoded.
fn streams(doc: &Document, index: usize) -> Vec<String> {
    let all = doc.pages().unwrap();
    let page = all.get(index).unwrap();
    let items = match dict(doc, page.cos().obj_ref.unwrap()).get("Contents").cloned() {
        Some(Object::Array(items)) => items,
        Some(single) => vec![single],
        None => Vec::new(),
    };
    items
        .iter()
        .map(|i| match &*doc.resolve(i.as_ref().unwrap()).unwrap() {
            Object::Stream(s) => String::from_utf8_lossy(&doc.cos().decode_stream(s).unwrap()).into_owned(),
            other => panic!("{other:?}"),
        })
        .collect()
}

/// The decoded streams of every form the page draws that Lamina PDF added, outermost first:
/// the container, then the forms it draws.
fn forms(doc: &Document, index: usize) -> Vec<(Dict, String)> {
    let all = doc.pages().unwrap();
    let page = all.get(index).unwrap();
    let page_dict = dict(doc, page.cos().obj_ref.unwrap());
    let resources = doc
        .deref(page_dict.get("Resources").unwrap())
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let Some(xobjects) = resources.get("XObject") else {
        return Vec::new();
    };
    let xobjects = doc.deref(xobjects).unwrap().as_dict().unwrap().clone();
    let mut out = Vec::new();
    for (_, x) in xobjects.iter() {
        if let Object::Stream(s) = doc.deref(x).unwrap().into_owned()
            && s.dict.get("PieceInfo").is_some()
        {
            out.push((
                s.dict.clone(),
                String::from_utf8_lossy(&doc.cos().decode_stream(&s).unwrap()).into_owned(),
            ));
        }
    }
    out
}

/// The text of each text box a mark's container draws, read from the child forms' streams.
fn shown(doc: &Document, index: usize) -> Vec<String> {
    let all = doc.pages().unwrap();
    let page = all.get(index).unwrap();
    let page_dict = dict(doc, page.cos().obj_ref.unwrap());
    let resources = doc
        .deref(page_dict.get("Resources").unwrap())
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let xobjects = doc
        .deref(resources.get("XObject").unwrap())
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let mut out = Vec::new();
    for (_, x) in xobjects.iter() {
        let Object::Stream(container) = doc.deref(x).unwrap().into_owned() else {
            continue;
        };
        let Some(inner) = container
            .dict
            .get("Resources")
            .and_then(|r| doc.deref(r).ok())
            .and_then(|r| r.as_dict().and_then(|d| d.get("XObject").cloned()))
        else {
            continue;
        };
        let inner = doc.deref(&inner).unwrap().as_dict().unwrap().clone();
        for (_, child) in inner.iter() {
            if let Object::Stream(c) = doc.deref(child).unwrap().into_owned() {
                let body = String::from_utf8_lossy(&doc.cos().decode_stream(&c).unwrap()).into_owned();
                out.extend(literals(&body));
            }
        }
    }
    out
}

/// The strings in a content stream, literal and hexadecimal (a standard font's text in
/// `WinAnsiEncoding`, read as Latin-1).
fn literals(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => {
                let end = body[i..].find(')').unwrap() + i;
                out.push(body[i + 1..end].to_owned());
                i = end;
            }
            b'<' if bytes.get(i + 1) != Some(&b'<') => {
                let end = body[i..].find('>').unwrap() + i;
                let hex = &body[i + 1..end];
                out.push(
                    (0..hex.len() / 2)
                        .map(|k| char::from(u8::from_str_radix(&hex[2 * k..2 * k + 2], 16).unwrap()))
                        .collect(),
                );
                i = end;
            }
            b'<' => i += 1,
            _ => {}
        }
        i += 1;
    }
    out
}

fn watermark(doc: &Document, name: &str, which: &[usize]) -> AddWatermark {
    let all = pages(doc);
    AddWatermark::new(
        id(name),
        which.iter().map(|i| all[*i].clone()).collect(),
        text("DRAFT", 72.0),
        date(),
    )
}

mod watermarks {
    use super::*;

    #[test]
    fn a_watermark_is_one_form_a_page_drawn_in_front_and_listed() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        doc.applied(
            watermark(&doc, "draft", &[0, 2])
                .with_opacity(laminapdf_ops::Opacity::new(0.3).unwrap())
                .with_angle(45.0),
        );
        for index in [0, 2] {
            let page = streams(&doc, index);
            let last = page.last().unwrap();
            assert!(
                last.starts_with("q ") && last.contains("/Fm") && last.trim_end().ends_with('Q'),
                "{last}"
            );
            assert!(last.contains(" gs "), "{last}");
            assert_eq!(forms(&doc, index).len(), 1);
            assert_eq!(shown(&doc, index), ["DRAFT"]);
            let marks = page_marks(&doc, &all[index]).unwrap();
            assert_eq!(marks.len(), 1);
            assert_eq!(marks[0].id, id("draft"));
            assert!(!marks[0].hideable);
            // A mark is not an added object, and content ops don't find it.
            assert_eq!(added_objects(&doc, &all[index]).unwrap(), []);
        }
        // The page it wasn't named for is untouched.
        assert_eq!(page_marks(&doc, &all[1]).unwrap(), []);
        assert_eq!(
            doc.refused(DeleteContentObject::new(id("draft"), all[0].clone())),
            "unknown-content-object"
        );
        assert_eq!(
            doc.refused(UpdateContentObject::new(id("draft"), all[0].clone(), date())),
            "unknown-content-object"
        );
    }

    #[test]
    fn a_background_goes_behind_the_content_over_the_backgrounds_before_it() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        doc.applied(AddBackground::new(id("one"), all.clone(), text("ONE", 40.0), date()));
        doc.applied(AddBackground::new(id("two"), all.clone(), text("TWO", 40.0), date()));
        let page = streams(&doc, 0);
        // The page's own content is wrapped between the two: q, one, two, the content, Q.
        assert!(page[0].starts_with('q') && page[0].len() < 4, "{page:?}");
        let marks: Vec<_> = page_marks(&doc, &all[0]).unwrap().into_iter().map(|m| m.id).collect();
        assert_eq!(marks, [id("one"), id("two")]);
        assert!(page.iter().take(3).skip(1).all(|s| s.contains("Do")), "{page:?}");
        assert!(
            page.iter().skip(3).any(|s| s.contains("BT") || s.contains("Tj")),
            "{page:?}"
        );
    }

    #[test]
    fn an_id_in_use_by_a_mark_or_an_added_object_is_refused_and_nothing_is_written() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        let before = common::snapshot(&doc);
        doc.applied(watermark(&doc, "draft", &[0]));
        assert_eq!(doc.refused(watermark(&doc, "draft", &[0])), "content-id-in-use");
        let rect = laminapdf_model::RectPt::from_xywh(
            laminapdf_model::Pt::new(72.0),
            laminapdf_model::Pt::new(72.0),
            laminapdf_model::Pt::new(100.0),
            laminapdf_model::Pt::new(20.0),
        );
        let note = AddText::new(
            id("draft"),
            all[0].clone(),
            rect,
            vec![laminapdf_ops::TextSpan::plain("x")],
            date(),
        );
        assert_eq!(doc.refused(note), "content-id-in-use");
        doc.undo().unwrap();
        assert_eq!(common::snapshot(&doc), before);
        // A mark with an ID an added object has is refused too, even when only one page has it.
        let mut doc = self::doc("corpus/outline.pdf");
        let all = pages(&doc);
        let shape = AddContentObject::new(
            id("box"),
            all[2].clone(),
            ContentObjectSpec::Shape {
                path: laminapdf_ops::PathSpec::Rect { rect, radius: 0.0 },
                fill: Some(AnnotColor::Gray(0.5)),
                stroke: None,
            },
            laminapdf_model::Matrix::IDENTITY,
            date(),
        );
        doc.applied(shape);
        let snapshot = common::snapshot(&doc);
        assert_eq!(doc.refused(watermark(&doc, "box", &[0, 1, 2])), "content-id-in-use");
        assert_eq!(common::snapshot(&doc), snapshot);
    }

    #[test]
    fn bad_pages_text_sizes_and_angles_are_invalid() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        assert_eq!(doc.refused(watermark(&doc, "a", &[])), "invalid-op");
        assert_eq!(doc.refused(watermark(&doc, "a", &[0, 0])), "invalid-op");
        let blank = AddWatermark::new(id("a"), vec![all[0].clone()], text("  ", 12.0), date());
        assert_eq!(doc.refused(blank), "invalid-op");
        let big = AddWatermark::new(id("a"), vec![all[0].clone()], text("x", 900.0), date());
        assert_eq!(doc.refused(big), "invalid-op");
        assert_eq!(
            doc.refused(watermark(&doc, "a", &[0]).with_angle(f64::NAN)),
            "invalid-op"
        );
        assert_eq!(doc.refused(watermark(&doc, "a", &[0]).with_angle(720.0)), "invalid-op");
        let unknown = AddWatermark::new(id("a"), vec!["p99.0".parse().unwrap()], text("x", 12.0), date());
        assert_eq!(doc.refused(unknown), "unknown-page");
    }

    #[test]
    fn without_a_generator_there_is_no_mark() {
        let mut doc = open(&fixture("golden/examples/hello.pdf"));
        let all = pages(&doc);
        let op = AddWatermark::new(id("a"), vec![all[0].clone()], text("x", 12.0), date());
        assert_eq!(doc.refused(op), "appearance-unavailable");
    }

    #[test]
    fn a_character_no_font_has_is_missing_and_a_picture_not_an_image_is_invalid() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        let op = AddWatermark::new(id("a"), vec![all[0].clone()], text("日本語", 40.0), date());
        let code = doc.refused(op);
        assert!(
            matches!(code, "content-glyph-missing" | "appearance-needs-text"),
            "{code}"
        );
        let blob = doc.add_blob(b"not an image".to_vec());
        let op = AddWatermark::new(id("b"), vec![all[0].clone()], MarkContent::Image { blob }, date());
        assert_eq!(doc.refused(op), "content-object-invalid");
    }

    #[test]
    fn an_image_watermark_and_background_are_drawn_from_the_blob_store() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        let blob = doc.add_blob(fixture("content/add/logo-alpha.png"));
        doc.applied(AddWatermark::new(
            id("logo"),
            all.clone(),
            MarkContent::Image { blob },
            date(),
        ));
        doc.applied(AddBackground::new(
            id("paper"),
            all.clone(),
            MarkContent::Image { blob },
            date(),
        ));
        assert_eq!(page_marks(&doc, &all[0]).unwrap().len(), 2);
        // One image object serves both marks' forms.
        assert_eq!(forms(&doc, 0).len(), 2);
    }
}

mod fields {
    use super::*;

    fn slots(text: &str, at: usize) -> [Option<String>; 6] {
        let mut slots: [Option<String>; 6] = Default::default();
        slots[at] = Some(text.to_owned());
        slots
    }

    #[test]
    fn page_number_total_label_date_and_file_are_filled_per_page() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        let op = SetHeaderFooter::new(
            id("folio"),
            all,
            slots("Page {page} of {pages} · {label} · {date} · {file} {{x}}", 4),
            date(),
        )
        .with_fields("27 Nov 2026", "outline.pdf");
        doc.applied(op);
        for index in 0..3 {
            // The file's own page labels: Roman, then A-1 and A-2.
            let label = ["i", "A-1", "A-2"][index];
            let want = format!("Page {} of 3 · {label} · 27 Nov 2026 · outline.pdf {{x}}", index + 1);
            assert_eq!(shown(&doc, index).join(""), want, "page {index}");
        }
    }

    #[test]
    fn labels_come_from_the_documents_page_labels() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        doc.applied(laminapdf_ops::SetPageLabels::new(vec![
            laminapdf_ops::PageLabelRange::new(all[0].clone(), Some(laminapdf_model::LabelStyle::UpperLetters)),
        ]));
        doc.applied(SetHeaderFooter::new(id("folio"), all, slots("{label}", 3), date()));
        assert_eq!(
            (0..3).map(|i| shown(&doc, i).join("")).collect::<Vec<_>>(),
            ["A", "B", "C"]
        );
    }

    #[test]
    fn six_slots_are_six_text_boxes_in_the_right_columns() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        let six = ["a", "b", "c", "d", "e", "f"].map(|s| Some(s.to_owned()));
        doc.applied(SetHeaderFooter::new(id("all"), all, six, date()));
        assert_eq!(shown(&doc, 0).len(), 6);
        let (container, content) = forms(&doc, 0).pop().unwrap();
        // The container is the page's size, in page space; each box has its own matrix.
        assert!(container.get("BBox").is_some());
        assert_eq!(content.matches(" Do").count(), 6, "{content}");
    }

    #[test]
    fn an_empty_header_or_a_bad_margin_is_invalid() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        let none = SetHeaderFooter::new(id("a"), all.clone(), Default::default(), date());
        assert_eq!(doc.refused(none), "invalid-op");
        let blank = SetHeaderFooter::new(id("a"), all.clone(), slots("  ", 0), date());
        assert_eq!(doc.refused(blank), "invalid-op");
        let margin = SetHeaderFooter::new(id("a"), all, slots("x", 0), date()).with_margins(MarkMargins::all(-1.0));
        assert_eq!(doc.refused(margin), "invalid-op");
    }
}

mod bates {
    use super::*;

    #[test]
    fn numbers_count_up_in_the_order_the_op_lists_the_pages_padded_and_prefixed() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        // Page 3 first: numbering follows the op's order, not the document's.
        let order = vec![all[2].clone(), all[0].clone(), all[1].clone()];
        doc.applied(ApplyBates::new(id("bates"), order, 41, Slot::BottomRight, date()).with_text("ACME-", 6, " CONF"));
        assert_eq!(shown(&doc, 2).join(""), "ACME-000041 CONF");
        assert_eq!(shown(&doc, 0).join(""), "ACME-000042 CONF");
        assert_eq!(shown(&doc, 1).join(""), "ACME-000043 CONF");
        // A page keeps its own number in what it lists.
        let PageMark::Bates(b) = &page_marks(&doc, &all[1]).unwrap()[0].mark else {
            panic!("bates");
        };
        assert_eq!(b.start, 43);
    }

    #[test]
    fn a_number_wider_than_its_digits_is_not_cut_and_too_many_digits_are_invalid() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        doc.applied(ApplyBates::new(id("a"), all.clone(), 12_345, Slot::TopCenter, date()).with_text("", 2, ""));
        assert_eq!(shown(&doc, 0).join(""), "12345");
        let wide = ApplyBates::new(id("b"), all.clone(), 1, Slot::TopLeft, date()).with_text("", 21, "");
        assert_eq!(doc.refused(wide), "invalid-op");
        let overflow = ApplyBates::new(id("c"), all, u64::MAX, Slot::TopLeft, date());
        // One page: the number is u64::MAX, which fits; a second page would overflow.
        doc.applied(overflow);
    }

    #[test]
    fn numbers_that_overflow_are_invalid() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        let op = ApplyBates::new(id("c"), all, u64::MAX, Slot::TopLeft, date());
        assert_eq!(doc.refused(op), "invalid-op");
    }
}

mod hideable {
    use super::*;

    fn properties(doc: &Document) -> Option<Dict> {
        let catalog = doc.catalog().unwrap();
        let entry = catalog.get("OCProperties")?;
        Some(doc.deref(entry).unwrap().as_dict().unwrap().clone())
    }

    fn groups(doc: &Document) -> Vec<Dict> {
        let Some(props) = properties(doc) else {
            return Vec::new();
        };
        let Some(Object::Array(items)) = props.get("OCGs").map(|o| doc.deref(o).unwrap().into_owned()) else {
            return Vec::new();
        };
        items.iter().map(|g| dict(doc, g.as_ref().unwrap())).collect()
    }

    #[test]
    fn a_hideable_mark_names_one_group_on_every_page_with_its_usage() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        doc.applied(watermark(&doc, "draft", &[0, 1, 2]).hideable());
        let found = groups(&doc);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].get_name("Type"), Some(b"OCG".as_slice()));
        let usage = found[0].get("Usage").and_then(Object::as_dict).unwrap();
        let element = usage.get("PageElement").and_then(Object::as_dict).unwrap();
        assert_eq!(element.get_name("Subtype"), Some(b"FG".as_slice()));
        let oc: Vec<_> = (0..3).map(|i| forms(&doc, i)[0].0.get("OC").cloned()).collect();
        assert!(oc.iter().all(|o| o.is_some() && *o == oc[0]), "{oc:?}");
        assert!(page_marks(&doc, &all[1]).unwrap()[0].hideable);
        // It is listed in the default configuration's order, so a layers panel shows it.
        let props = properties(&doc).unwrap();
        let config = props.get("D").and_then(Object::as_dict).unwrap();
        assert_eq!(config.get("Order").and_then(Object::as_array).unwrap().len(), 1);
    }

    #[test]
    fn a_background_group_is_background_and_an_existing_layer_is_kept() {
        let mut doc = doc("content/add/layers.pdf");
        let before = groups(&doc).len();
        let all = pages(&doc);
        doc.applied(AddBackground::new(id("bg"), all.clone(), text("BG", 30.0), date()).hideable());
        let found = groups(&doc);
        assert_eq!(found.len(), before + 1);
        let last = found.last().unwrap();
        let usage = last.get("Usage").and_then(Object::as_dict).unwrap();
        assert_eq!(
            usage
                .get("PageElement")
                .and_then(Object::as_dict)
                .unwrap()
                .get_name("Subtype"),
            Some(b"BG".as_slice())
        );
        doc.applied(DeletePageMark::new(id("bg"), all));
        assert_eq!(groups(&doc).len(), before);
    }

    #[test]
    fn deleting_a_mark_from_some_pages_keeps_its_group_until_the_last_one_goes() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        doc.applied(watermark(&doc, "draft", &[0, 1, 2]).hideable());
        doc.applied(DeletePageMark::new(id("draft"), vec![all[0].clone()]));
        assert_eq!(groups(&doc).len(), 1);
        doc.applied(DeletePageMark::new(id("draft"), vec![all[1].clone(), all[2].clone()]));
        assert_eq!(groups(&doc), []);
        assert!(properties(&doc).is_none());
    }

    #[test]
    fn a_mark_made_unhideable_leaves_the_layers_and_one_made_hideable_joins_them() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        doc.applied(watermark(&doc, "draft", &[0]).hideable());
        let plain = PageMark::Watermark(laminapdf_ops::WatermarkSpec::new(text("DRAFT", 72.0)));
        doc.applied(UpdatePageMark::new(id("draft"), all.clone(), plain, date()));
        assert_eq!(groups(&doc), []);
        assert!(forms(&doc, 0)[0].0.get("OC").is_none());
        let mut spec = laminapdf_ops::WatermarkSpec::new(text("DRAFT", 72.0));
        spec.hideable = true;
        doc.applied(UpdatePageMark::new(id("draft"), all, PageMark::Watermark(spec), date()));
        assert_eq!(groups(&doc).len(), 1);
    }
}

mod changing {
    use super::*;

    #[test]
    fn an_update_draws_the_mark_again_in_its_place_under_the_same_id() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        doc.applied(watermark(&doc, "draft", &[0, 1]));
        doc.applied(AddWatermark::new(
            id("other"),
            vec![all[0].clone()],
            text("OTHER", 20.0),
            date(),
        ));
        let before = streams(&doc, 0);
        let new = PageMark::Watermark(laminapdf_ops::WatermarkSpec::new(text("FINAL", 60.0)));
        doc.applied(UpdatePageMark::new(id("draft"), all[..2].to_vec(), new, date()));
        let after = streams(&doc, 0);
        // The same number of streams, the draft's still before the other mark's.
        assert_eq!(after.len(), before.len());
        let marks: Vec<_> = page_marks(&doc, &all[0]).unwrap().into_iter().map(|m| m.id).collect();
        assert_eq!(marks, [id("draft"), id("other")]);
        assert!(shown(&doc, 0).contains(&"FINAL".to_owned()));
        assert!(!shown(&doc, 0).contains(&"DRAFT".to_owned()), "{:?}", shown(&doc, 0));
        assert_eq!(shown(&doc, 1), ["FINAL"]);
    }

    #[test]
    fn an_update_refills_fields_and_renumbers_bates_from_its_new_start() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        doc.applied(ApplyBates::new(id("bates"), all.clone(), 1, Slot::BottomLeft, date()).with_text("B", 3, ""));
        let PageMark::Bates(mut spec) = page_marks(&doc, &all[0]).unwrap()[0].mark.clone() else {
            panic!("bates");
        };
        spec.start = 100;
        let reversed: Vec<_> = all.iter().rev().cloned().collect();
        doc.applied(UpdatePageMark::new(
            id("bates"),
            reversed,
            PageMark::Bates(spec),
            date(),
        ));
        assert_eq!(shown(&doc, 2).join(""), "B100");
        assert_eq!(shown(&doc, 0).join(""), "B102");
    }

    #[test]
    fn updating_what_isnt_there_or_with_another_kind_is_refused() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        doc.applied(watermark(&doc, "draft", &[0]));
        let wm = || PageMark::Watermark(laminapdf_ops::WatermarkSpec::new(text("X", 20.0)));
        assert_eq!(
            doc.refused(UpdatePageMark::new(id("nope"), vec![all[0].clone()], wm(), date())),
            "unknown-page-mark"
        );
        // One page of two has no such mark: nothing changes.
        let snapshot = common::snapshot(&doc);
        assert_eq!(
            doc.refused(UpdatePageMark::new(id("draft"), all[..2].to_vec(), wm(), date())),
            "unknown-page-mark"
        );
        assert_eq!(common::snapshot(&doc), snapshot);
        let bg = PageMark::Background(laminapdf_ops::WatermarkSpec::new(text("X", 20.0)));
        assert_eq!(
            doc.refused(UpdatePageMark::new(id("draft"), vec![all[0].clone()], bg, date())),
            "invalid-op"
        );
        assert_eq!(
            doc.refused(DeletePageMark::new(id("nope"), vec![all[0].clone()])),
            "unknown-page-mark"
        );
    }

    #[test]
    fn a_delete_takes_the_form_the_stream_and_the_opacity_off_the_page() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        doc.applied(watermark(&doc, "draft", &[0]).with_opacity(laminapdf_ops::Opacity::new(0.5).unwrap()));
        // The page's content is wrapped in q … Q once, then the drawing follows it.
        let drawn = streams(&doc, 0).len();
        let gone = doc.applied(DeletePageMark::new(id("draft"), all.clone()));
        assert!(gone.still_in_earlier_revisions);
        assert_eq!(streams(&doc, 0).len(), drawn - 1);
        assert_eq!(forms(&doc, 0), []);
        assert_eq!(page_marks(&doc, &all[0]).unwrap(), []);
    }

    #[test]
    fn undo_and_redo_leave_the_overlay_as_it_was_and_as_it_was_made() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        let before = common::snapshot(&doc);
        doc.applied(watermark(&doc, "draft", &[0, 1]).hideable());
        let added = common::snapshot(&doc);
        let new = PageMark::Watermark(laminapdf_ops::WatermarkSpec::new(text("FINAL", 60.0)));
        doc.applied(UpdatePageMark::new(id("draft"), all[..2].to_vec(), new, date()));
        let changed = common::snapshot(&doc);
        doc.applied(DeletePageMark::new(id("draft"), all[..2].to_vec()));
        let deleted = common::snapshot(&doc);
        for expected in [&changed, &added, &before] {
            doc.undo().unwrap();
            assert_eq!(&common::snapshot(&doc), expected);
        }
        for expected in [&added, &changed, &deleted] {
            doc.redo().unwrap();
            assert_eq!(&common::snapshot(&doc), expected);
        }
    }
}

mod tagging {
    use super::*;

    fn last(doc: &Document) -> String {
        streams(doc, 0).pop().unwrap()
    }

    #[test]
    fn marks_are_artifacts_of_their_kind_in_a_tagged_file_and_own_no_structure() {
        for (op, artifact) in [
            (
                Op::from(AddWatermark::new(id("m"), vec![], text("X", 20.0), date())),
                "/Artifact <</Type /Pagination /Subtype /Watermark>> BDC",
            ),
            (
                Op::from(AddBackground::new(id("m"), vec![], text("X", 20.0), date())),
                "/Artifact <</Type /Background>> BDC",
            ),
        ] {
            let mut doc = doc("content/add/tagged.pdf");
            let page = pages(&doc);
            let op = match op {
                Op::AddWatermark(mut o) => {
                    o.pages = page.clone();
                    Op::from(o)
                }
                Op::AddBackground(mut o) => {
                    o.pages = page.clone();
                    Op::from(o)
                }
                other => other,
            };
            let before = struct_count(&doc);
            doc.applied(op);
            let streams = streams(&doc, 0);
            assert!(streams.iter().any(|s| s.contains(artifact)), "{artifact}: {streams:?}");
            assert_eq!(struct_count(&doc), before);
        }
    }

    fn struct_count(doc: &Document) -> usize {
        let catalog = doc.catalog().unwrap();
        let root = dict(doc, catalog.get("StructTreeRoot").unwrap().as_ref().unwrap());
        let document = dict(doc, root.get("K").unwrap().as_ref().unwrap());
        document.get("K").and_then(Object::as_array).map_or(0, <[Object]>::len)
    }

    #[test]
    fn a_header_a_footer_and_both_say_which() {
        let mut doc = doc("content/add/tagged.pdf");
        let page = pages(&doc);
        let mut top: [Option<String>; 6] = Default::default();
        top[1] = Some("Top".to_owned());
        let mut bottom: [Option<String>; 6] = Default::default();
        bottom[4] = Some("Bottom".to_owned());
        let mut both = top.clone();
        both[4] = Some("Bottom".to_owned());
        doc.applied(SetHeaderFooter::new(id("h"), page.clone(), top, date()));
        assert!(last(&doc).contains("/Subtype /Header"), "{}", last(&doc));
        doc.applied(SetHeaderFooter::new(id("f"), page.clone(), bottom, date()));
        assert!(last(&doc).contains("/Subtype /Footer"), "{}", last(&doc));
        doc.applied(SetHeaderFooter::new(id("b"), page.clone(), both, date()));
        assert!(last(&doc).contains("/Type /Pagination>>"), "{}", last(&doc));
        doc.applied(ApplyBates::new(id("n"), page, 1, Slot::TopRight, date()));
        assert!(last(&doc).contains("/Subtype /Bates"), "{}", last(&doc));
    }

    #[test]
    fn an_untagged_file_stays_unmarked_and_a_pdf_ua_file_needs_no_alt_text() {
        let mut plain = doc("golden/examples/hello.pdf");
        plain.applied(watermark(&plain, "draft", &[0]));
        assert!(!last(&plain).contains("BDC"));
        let mut ua = common::claiming(r#"pdfuaid:part="1""#).with_appearances(AnnotationAppearances);
        let all = pages(&ua);
        let blob = ua.add_blob(fixture("content/add/logo-alpha.png"));
        ua.applied(AddWatermark::new(id("logo"), all, MarkContent::Image { blob }, date()));
    }

    #[test]
    fn an_update_keeps_the_artifact() {
        let mut doc = doc("content/add/tagged.pdf");
        let all = pages(&doc);
        doc.applied(watermark(&doc, "draft", &[0]));
        let new = PageMark::Watermark(laminapdf_ops::WatermarkSpec::new(text("FINAL", 60.0)));
        doc.applied(UpdatePageMark::new(id("draft"), all, new, date()));
        assert!(last(&doc).contains("/Subtype /Watermark"), "{}", last(&doc));
    }
}

mod wire {
    use super::*;

    #[test]
    fn every_op_reads_and_writes_its_wire_form() {
        for json in [
            r#"{"op":"addWatermark","args":{"id":"draft","pages":["p3.0","p4.0"],"mark":{"text":{"text":"DRAFT","fonts":["helvetica"],"size":96.0,"color":[0.8,0.0,0.0]}},"opacity":0.3,"angle":45.0,"hideable":true,"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"addBackground","args":{"id":"paper","pages":["p3.0"],"mark":{"image":{"blob":"0000000000000000000000000000000000000000000000000000000000000000"}},"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"setHeaderFooter","args":{"id":"folio","pages":["p3.0"],"slots":[null,null,null,"{file}",null,"Page {page} of {pages}"],"fonts":["times"],"size":9.0,"margins":{"top":20.0,"right":36.0,"bottom":30.0,"left":36.0},"date":"27 Nov 2026","file":"a.pdf","modified":"D:20261118120000Z"}}"#,
            r#"{"op":"applyBates","args":{"id":"bates","pages":["p3.0","p4.0"],"prefix":"ACME-","start":1,"digits":4,"suffix":"-X","slot":"bottomRight","size":9.0,"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"updatePageMark","args":{"id":"draft","pages":["p3.0"],"mark":{"watermark":{"mark":{"text":{"text":"FINAL","size":96.0}},"angle":45.0}},"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"deletePageMark","args":{"id":"draft","pages":["p3.0"]}}"#,
        ] {
            let op: Op = serde_json::from_str(json).unwrap();
            assert_eq!(serde_json::to_string(&op).unwrap(), json);
        }
    }

    #[test]
    fn unknown_fields_and_bad_values_are_refused() {
        for json in [
            r#"{"op":"addWatermark","args":{"id":"d","pages":["p3.0"],"mark":{"text":{"text":"x","size":9.0}},"modified":"D:20261118120000Z","extra":1}}"#,
            r#"{"op":"addWatermark","args":{"id":"d","pages":["p3.0"],"mark":{"text":{"text":"x"}},"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"addWatermark","args":{"id":"d","pages":["p3.0"],"mark":{"stamp":{}},"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"addWatermark","args":{"id":"d e","pages":["p3.0"],"mark":{"text":{"text":"x","size":9.0}},"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"setHeaderFooter","args":{"id":"d","pages":["p3.0"],"slots":["a","b"],"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"applyBates","args":{"id":"d","pages":["p3.0"],"start":-1,"slot":"topLeft","modified":"D:20261118120000Z"}}"#,
            r#"{"op":"applyBates","args":{"id":"d","pages":["p3.0"],"start":1,"slot":"middle","modified":"D:20261118120000Z"}}"#,
            r#"{"op":"updatePageMark","args":{"id":"d","pages":["p3.0"],"mark":{"bates":{"start":1}},"modified":"D:20261118120000Z"}}"#,
        ] {
            assert!(serde_json::from_str::<Op>(json).is_err(), "{json}");
        }
    }
}

mod conformance {
    use laminapdf_ops::{ConformanceWarning, pending_conformance_warnings};

    use super::*;

    #[test]
    fn a_standard_font_mark_on_a_pdf_a_file_warns_with_its_page() {
        let mut doc =
            common::claiming(r#"pdfaid:part="2" pdfaid:conformance="B""#).with_appearances(AnnotationAppearances);
        let all = pages(&doc);
        doc.applied(AddWatermark::new(id("d"), all.clone(), text("DRAFT", 40.0), date()));
        let warnings: Vec<_> = pending_conformance_warnings(&doc, doc.pending_changes())
            .unwrap()
            .into_iter()
            .filter(|w| w.code() == "pdfa-font-not-embedded")
            .collect();
        assert_eq!(
            warnings,
            [ConformanceWarning::PdfaFontNotEmbedded {
                annotations: Vec::new(),
                pages: vec![all[0].to_string()]
            }]
        );
    }
}

mod review {
    use laminapdf_ops::{ApplyBates, Op};

    use super::*;

    /// A one-page file with an optional content group and the configurations `config` and
    /// `configs` (the text of `/D` and `/Configs`).
    fn layered(config: &str, configs: &str) -> Document {
        let catalog = format!(
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R] /D {config} /Configs {configs} >> >>"
        );
        open(&common::classic(
            &[
                catalog.as_bytes(),
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>",
                b"<< /Type /OCG /Name (Notes) >>",
            ],
            "<< /Size 5 /Root 1 0 R >>",
        ))
        .with_appearances(AnnotationAppearances)
    }

    fn properties(doc: &Document) -> Dict {
        let catalog = doc.catalog().unwrap();
        doc.deref(catalog.get("OCProperties").unwrap())
            .unwrap()
            .as_dict()
            .unwrap()
            .clone()
    }

    fn list(doc: &Document, dict: &Dict, key: &str) -> Vec<Object> {
        dict.get(key)
            .map(|o| doc.deref(o).unwrap().as_array().unwrap().to_vec())
            .unwrap_or_default()
    }

    fn default_config(doc: &Document) -> Dict {
        let props = properties(doc);
        doc.deref(props.get("D").unwrap()).unwrap().as_dict().unwrap().clone()
    }

    #[test]
    fn a_configuration_gets_a_name_and_a_hidden_by_default_base_state_lists_the_group_as_on() {
        let mut doc = layered("<< /BaseState /OFF /ON [4 0 R] /Order [4 0 R] >>", "[]");
        doc.applied(watermark(&doc, "draft", &[0]).hideable());
        let config = default_config(&doc);
        assert!(matches!(config.get("Name"), Some(Object::String(_))), "{config:?}");
        // The group of the mark is on, as is the file's own: an off base state hides the rest.
        assert_eq!(list(&doc, &config, "ON").len(), 2);
        assert_eq!(list(&doc, &config, "Order").len(), 2);
        // A file with no configuration at all gets one with a name, too.
        let mut bare = common::claiming("").with_appearances(AnnotationAppearances);
        let page = pages(&bare);
        bare.applied(AddWatermark::new(id("w"), page, text("X", 20.0), date()).hideable());
        let config = default_config(&bare);
        assert!(matches!(config.get("Name"), Some(Object::String(_))));
    }

    #[test]
    fn deleting_the_last_page_takes_the_group_out_of_every_configuration_list() {
        // The mark's group is the first object the op makes, number 5: the file's own
        // configurations name it already, as another tool's might.
        let mut doc = layered(
            "<< /Order [4 0 R 5 0 R] /Locked [4 0 R 5 0 R] /RBGroups [[4 0 R 5 0 R]] /AS [<< /Event /View /OCGs [4 0 R 5 0 R] /Category [/View] >>] >>",
            "[<< /Name (Alt) /Order [5 0 R] /OFF [5 0 R] /Locked [5 0 R] >>]",
        );
        let all = pages(&doc);
        doc.applied(watermark(&doc, "draft", &[0]).hideable());
        let groups = list(&doc, &properties(&doc), "OCGs");
        assert_eq!(groups.len(), 2);
        let mine = groups[1].as_ref().unwrap();
        assert_eq!(mine.num, 5);
        doc.applied(DeletePageMark::new(id("draft"), all));
        let props = properties(&doc);
        assert_eq!(list(&doc, &props, "OCGs").len(), 1);
        let names = |objects: Vec<Object>| objects.iter().filter_map(Object::as_ref).filter(|r| r.num == 5).count();
        let config = default_config(&doc);
        for key in ["Order", "Locked"] {
            assert_eq!(list(&doc, &config, key).len(), 1, "{key}");
        }
        let rb = list(&doc, &config, "RBGroups");
        assert_eq!(names(rb[0].as_array().unwrap().to_vec()), 0);
        let state = doc
            .deref(&list(&doc, &config, "AS")[0])
            .unwrap()
            .as_dict()
            .unwrap()
            .clone();
        assert_eq!(names(list(&doc, &state, "OCGs")), 0);
        let alt = doc
            .deref(&list(&doc, &props, "Configs")[0])
            .unwrap()
            .as_dict()
            .unwrap()
            .clone();
        for key in ["Order", "OFF", "Locked"] {
            assert_eq!(names(list(&doc, &alt, key)), 0, "{key}");
        }
    }

    #[test]
    fn an_update_that_keeps_the_group_renames_it_for_the_new_text() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        doc.applied(watermark(&doc, "draft", &[0]).hideable());
        let mut spec = laminapdf_ops::WatermarkSpec::new(text("FINAL", 72.0));
        spec.hideable = true;
        doc.applied(UpdatePageMark::new(id("draft"), all, PageMark::Watermark(spec), date()));
        let group = list(&doc, &properties(&doc), "OCGs").pop().unwrap();
        let name = dict(&doc, group.as_ref().unwrap());
        let Some(Object::String(name)) = name.get("Name") else {
            panic!("name");
        };
        assert!(String::from_utf8_lossy(&name.bytes).contains("FINAL"), "{name:?}");
    }

    #[test]
    fn a_hideable_mark_on_a_pdf_a_1_file_warns_and_one_on_a_pdf_a_2_file_does_not() {
        use laminapdf_ops::pending_conformance_warnings;
        for (claim, warns) in [
            (r#"pdfaid:part="1" pdfaid:conformance="B""#, true),
            (r#"pdfaid:part="2" pdfaid:conformance="B""#, false),
        ] {
            let mut doc = common::claiming(claim).with_appearances(AnnotationAppearances);
            let all = pages(&doc);
            doc.applied(AddWatermark::new(id("d"), all.clone(), text("X", 20.0), date()).hideable());
            let found: Vec<_> = pending_conformance_warnings(&doc, doc.pending_changes())
                .unwrap()
                .into_iter()
                .filter(|w| w.code() == "pdfa-optional-content")
                .collect();
            assert_eq!(found.len(), usize::from(warns), "{claim}");
            if warns {
                assert_eq!(
                    found,
                    [laminapdf_ops::ConformanceWarning::PdfaOptionalContent {
                        pages: vec![all[0].to_string()]
                    }]
                );
            }
        }
    }

    #[test]
    fn a_footer_whose_top_slot_is_blank_or_fills_empty_is_a_footer() {
        let mut doc = doc("content/add/tagged.pdf");
        let page = pages(&doc);
        let mut slots: [Option<String>; 6] = Default::default();
        slots[1] = Some("   ".to_owned());
        slots[0] = Some("{file}".to_owned());
        slots[4] = Some("Bottom".to_owned());
        doc.applied(SetHeaderFooter::new(id("f"), page, slots, date()));
        let last = streams(&doc, 0).pop().unwrap();
        assert!(last.contains("/Subtype /Footer"), "{last}");
    }

    #[test]
    fn a_mark_in_both_bands_tags_each_part_inside_its_form() {
        let mut doc = doc("content/add/tagged.pdf");
        let page = pages(&doc);
        let mut slots: [Option<String>; 6] = Default::default();
        slots[1] = Some("Top".to_owned());
        slots[4] = Some("Bottom".to_owned());
        doc.applied(SetHeaderFooter::new(id("both"), page, slots, date()));
        let last = streams(&doc, 0).pop().unwrap();
        assert!(last.contains("/Type /Pagination>>"), "{last}");
        let (_, content) = forms(&doc, 0).pop().unwrap();
        assert!(content.contains("/Subtype /Header>> BDC"), "{content}");
        assert!(content.contains("/Subtype /Footer>> BDC"), "{content}");
        assert_eq!(content.matches("EMC").count(), 2, "{content}");
        // An untagged file's mark has no marked content at all.
        let mut plain = self::doc("golden/examples/hello.pdf");
        let page = pages(&plain);
        let mut slots: [Option<String>; 6] = Default::default();
        slots[1] = Some("Top".to_owned());
        slots[4] = Some("Bottom".to_owned());
        plain.applied(SetHeaderFooter::new(id("both"), page, slots, date()));
        assert!(!forms(&plain, 0).pop().unwrap().1.contains("BDC"));
    }

    #[test]
    fn an_id_another_page_has_as_another_kind_is_refused_and_the_same_kind_is_added_to_more_pages() {
        let mut doc = doc("corpus/outline.pdf");
        let all = pages(&doc);
        doc.applied(watermark(&doc, "mark", &[0]));
        let snapshot = common::snapshot(&doc);
        let background = AddBackground::new(id("mark"), vec![all[1].clone()], text("B", 20.0), date());
        assert_eq!(doc.refused(background), "invalid-op");
        assert_eq!(common::snapshot(&doc), snapshot);
        // The same kind on other pages is how a mark grows to more pages.
        doc.applied(watermark(&doc, "mark", &[1, 2]));
        assert_eq!(page_marks(&doc, &all[2]).unwrap().len(), 1);
    }

    #[test]
    fn a_bates_number_is_six_digits_unless_told_otherwise_in_code_and_on_the_wire() {
        let mut doc = doc("golden/examples/hello.pdf");
        let all = pages(&doc);
        doc.applied(ApplyBates::new(id("a"), all.clone(), 7, Slot::TopLeft, date()));
        assert_eq!(shown(&doc, 0).join(""), "000007");
        let json = r#"{"op":"applyBates","args":{"id":"b","pages":["p3.0"],"start":8,"slot":"topRight","modified":"D:20261118120000Z"}}"#;
        let Op::ApplyBates(op) = serde_json::from_str::<Op>(json).unwrap() else {
            panic!("bates");
        };
        assert_eq!(op.digits, 6);
        // The default is left off the wire; no padding is written as 0.
        assert_eq!(serde_json::to_string(&Op::ApplyBates(op.clone())).unwrap(), json);
        let plain = ApplyBates::new(id("c"), all, 9, Slot::TopCenter, date()).with_text("", 0, "");
        assert!(
            serde_json::to_string(&Op::from(plain))
                .unwrap()
                .contains(r#""digits":0"#)
        );
    }

    #[test]
    fn an_image_marks_pixel_size_is_read_once_for_every_page() {
        // Three pages, one image: the same form serves each, and each page's container scales it.
        let mut doc = doc("corpus/outline.pdf");
        let blob = doc.add_blob(fixture("content/add/logo-alpha.png"));
        let all = pages(&doc);
        doc.applied(AddWatermark::new(id("logo"), all, MarkContent::Image { blob }, date()));
        assert_eq!(forms(&doc, 2).len(), 1);
    }
}

mod properties {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(24))]

        /// Any mark on any pages, added, then replaced or deleted, undone and redone, leaves the
        /// overlay as it was and as it was made, so the next save writes the same bytes.
        #[test]
        fn every_page_mark_op_undone_leaves_the_overlay_as_it_was(
            kind in 0_u8..4,
            text in "[A-Za-z][A-Za-z0-9 ]{0,11}",
            size in 4.0..120.0_f64,
            angle in -180.0..180.0_f64,
            hideable in any::<bool>(),
            start in 0_u64..100_000,
            digits in 0_u8..12,
            slot in 0_usize..6,
            first in 0_usize..3,
            delete in any::<bool>(),
        ) {
            let mut doc = doc("corpus/outline.pdf");
            let all = pages(&doc);
            let which: Vec<PageId> = all[first..].to_vec();
            let before = common::snapshot(&doc);
            let content = MarkContent::Text { text: text.clone(), fonts: Vec::new(), size, color: None };
            let mut watermark = laminapdf_ops::WatermarkSpec::new(content);
            watermark.angle = angle;
            watermark.hideable = hideable;
            let (op, replacement): (Op, PageMark) = match kind {
                0 => (
                    AddWatermark::new(id("m"), which.clone(), watermark.mark.clone(), date())
                        .with_angle(angle)
                        .into(),
                    PageMark::Watermark(watermark.clone()),
                ),
                1 => (
                    AddBackground::new(id("m"), which.clone(), watermark.mark.clone(), date()).into(),
                    PageMark::Background(watermark.clone()),
                ),
                2 => {
                    let mut slots: [Option<String>; 6] = Default::default();
                    slots[slot] = Some(format!("{text} {{page}}"));
                    let spec = laminapdf_ops::HeaderFooterSpec::new(slots.clone());
                    (
                        SetHeaderFooter::new(id("m"), which.clone(), slots, date()).into(),
                        PageMark::HeaderFooter(spec),
                    )
                }
                _ => {
                    let slot = Slot::ALL[slot];
                    let op = ApplyBates::new(id("m"), which.clone(), start, slot, date()).with_text(text, digits, "");
                    (op.into(), PageMark::Bates(laminapdf_ops::BatesSpec::new(start + 1, slot)))
                }
            };
            doc.applied(op);
            let added = common::snapshot(&doc);
            let second: Op = if delete {
                DeletePageMark::new(id("m"), which.clone()).into()
            } else {
                UpdatePageMark::new(id("m"), which, replacement, date()).into()
            };
            doc.applied(second);
            let changed = common::snapshot(&doc);
            doc.undo().unwrap();
            prop_assert_eq!(&common::snapshot(&doc), &added);
            doc.undo().unwrap();
            prop_assert_eq!(common::snapshot(&doc), before);
            doc.redo().unwrap();
            prop_assert_eq!(common::snapshot(&doc), added);
            doc.redo().unwrap();
            prop_assert_eq!(common::snapshot(&doc), changed);
        }
    }
}
