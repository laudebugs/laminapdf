//! `AddText` (E6 plan §2.2, todo E6c): what it writes on a page, fonts and their fallbacks, the
//! refusals, tagging, editing and deleting the text through the content ops, and the wire form.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{claiming, fixture, open};
use laminapdf_build::AnnotationAppearances;
use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{Matrix, PageId, PdfDate, Pt, RectPt};
use laminapdf_ops::{
    AddText, AnnotColor, AnnotFont, Applied, ContentId, ContentObjectSpec, DeleteContentObject, Document, Op, Origin,
    StandardFamily, TagChoice, TextAlign, TextBlock, TextSpan, UpdateContentObject, added_objects,
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

fn page(doc: &Document) -> PageId {
    doc.pages().unwrap().get(0).unwrap().id().clone()
}

fn id(s: &str) -> ContentId {
    ContentId::new(s).unwrap()
}

fn date() -> PdfDate {
    "D:20261118120000Z".parse().unwrap()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

const HELVETICA: AnnotFont = AnnotFont::Standard(StandardFamily::Helvetica);

fn text(doc: &Document, name: &str, runs: &str) -> AddText {
    AddText::new(
        id(name),
        page(doc),
        rect(72.0, 100.0, 288.0, 148.0),
        vec![TextSpan::plain(runs)],
        date(),
    )
}

fn font(doc: &mut Document, name: &str) -> AnnotFont {
    AnnotFont::Font(doc.add_blob(fixture(&format!("fonts/{name}"))))
}

fn dict(doc: &Document, r: ObjRef) -> Dict {
    match &*doc.resolve(r).unwrap() {
        Object::Dict(d) => d.clone(),
        Object::Stream(s) => s.dict.clone(),
        other => panic!("{other:?}"),
    }
}

/// The page's `/Contents`, each stream decoded.
fn streams(doc: &Document) -> Vec<String> {
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let Some(Object::Array(items)) = dict(doc, page.cos().obj_ref.unwrap()).get("Contents").cloned() else {
        panic!("contents");
    };
    items
        .iter()
        .map(|i| match &*doc.resolve(i.as_ref().unwrap()).unwrap() {
            Object::Stream(s) => String::from_utf8_lossy(&doc.cos().decode_stream(s).unwrap()).into_owned(),
            other => panic!("{other:?}"),
        })
        .collect()
}

/// The fonts the first added object's form uses: each font dictionary.
fn form_fonts(doc: &Document) -> Vec<Dict> {
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let page_dict = dict(doc, page.cos().obj_ref.unwrap());
    let Some(resources) = page_dict.get("Resources") else {
        return Vec::new();
    };
    let resources = doc.deref(resources).unwrap().as_dict().unwrap().clone();
    let Some(xobjects) = resources.get("XObject") else {
        return Vec::new();
    };
    let xobjects = doc.deref(xobjects).unwrap().as_dict().unwrap().clone();
    let mut fonts = Vec::new();
    for (_, x) in xobjects.iter() {
        let Object::Stream(form) = doc.deref(x).unwrap().into_owned() else {
            continue;
        };
        let Some(form_resources) = form.dict.get("Resources") else {
            continue;
        };
        let form_resources = doc.deref(form_resources).unwrap().as_dict().unwrap().clone();
        let Some(set) = form_resources.get("Font") else {
            continue;
        };
        for (_, f) in doc.deref(set).unwrap().as_dict().unwrap().iter() {
            fonts.push(doc.deref(f).unwrap().as_dict().unwrap().clone());
        }
    }
    fonts
}

/// `bytes` with `OS/2.fsType` set to `fs_type`.
fn with_fs_type(bytes: &[u8], fs_type: u16) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    for i in 0..tables {
        let record = 12 + 16 * i;
        if &bytes[record..record + 4] == b"OS/2" {
            let offset = u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize;
            out[offset + 8..offset + 10].copy_from_slice(&fs_type.to_be_bytes());
        }
    }
    out
}

mod adding {
    use super::*;

    #[test]
    fn text_in_a_standard_font_is_its_own_form_listed_with_its_box() {
        let mut doc = doc("golden/examples/hello.pdf");
        let applied = doc.applied(text(&doc, "note", "Reviewed twice"));
        assert_eq!(applied.touched_pages.len(), 1);
        let objects = added_objects(&doc, &page(&doc)).unwrap();
        assert_eq!(objects.len(), 1);
        let note = &objects[0];
        assert_eq!(note.id, id("note"));
        let Some(ContentObjectSpec::Text(block)) = &note.object else {
            panic!("{:?}", note.object);
        };
        assert_eq!(
            (block.width.to_bits(), block.height.to_bits()),
            (216.0_f64.to_bits(), 48.0_f64.to_bits())
        );
        // The page's top-left corner is the origin of page space: the box sits where the rect is.
        assert_eq!(note.bounds, rect(72.0, 100.0, 288.0, 148.0));
        let last = streams(&doc).pop().unwrap();
        assert!(last.starts_with("q 1 0 0 1 72 "), "{last}");
        assert!(last.trim_end().ends_with("Do Q"), "{last}");
        let fonts = form_fonts(&doc);
        assert_eq!(fonts.len(), 1);
        assert_eq!(fonts[0].get_name("BaseFont"), Some(b"Helvetica".as_slice()));
    }

    #[test]
    fn a_font_given_by_its_bytes_is_embedded_as_a_subset_with_a_to_unicode_map() {
        let mut doc = doc("golden/examples/hello.pdf");
        let noto = font(&mut doc, "NotoSans-Regular.ttf");
        let op = text(&doc, "note", "Grüße, Zoë").with_fonts(vec![noto], 14.0);
        doc.applied(op);
        let fonts = form_fonts(&doc);
        assert_eq!(fonts.len(), 1);
        assert_eq!(fonts[0].get_name("Subtype"), Some(b"Type0".as_slice()));
        assert!(fonts[0].get("ToUnicode").is_some(), "{:?}", fonts[0]);
    }

    #[test]
    fn a_character_the_font_lacks_comes_from_the_next_font_of_the_chain() {
        let mut doc = doc("golden/examples/hello.pdf");
        let hebrew = font(&mut doc, "NotoSansHebrew-Regular.ttf");
        let bare = text(&doc, "sum", "a שלום b").with_fonts(vec![HELVETICA], 12.0);
        // Helvetica has no Hebrew letters.
        assert_eq!(doc.refused(bare.clone()), "content-glyph-missing");
        doc.applied(bare.with_fonts(vec![HELVETICA, hebrew], 12.0));
        // The fallback's font and Helvetica both draw.
        assert_eq!(form_fonts(&doc).len(), 2);
    }

    #[test]
    fn bold_italic_alignment_colour_and_paragraphs_are_kept_in_the_object() {
        let mut doc = doc("golden/examples/hello.pdf");
        let runs = vec![
            TextSpan::plain("One "),
            TextSpan::plain("two").with_bold(true),
            TextSpan::plain("\nthree").with_italic(true),
        ];
        let op = AddText::new(id("note"), page(&doc), rect(72.0, 100.0, 288.0, 148.0), runs, date())
            .with_align(TextAlign::Center)
            .with_color(AnnotColor::Rgb(0.1, 0.1, 0.6));
        doc.applied(op);
        let block = match added_objects(&doc, &page(&doc)).unwrap().remove(0).object {
            Some(ContentObjectSpec::Text(b)) => b,
            other => panic!("{other:?}"),
        };
        assert_eq!(block.align, TextAlign::Center);
        assert_eq!(block.color, Some(AnnotColor::Rgb(0.1, 0.1, 0.6)));
        assert_eq!(block.paragraphs().len(), 2);
        // Bold and italic Helvetica faces for the styled spans.
        let names: Vec<_> = form_fonts(&doc)
            .iter()
            .filter_map(|f| f.get_name("BaseFont").map(<[u8]>::to_vec))
            .collect();
        assert!(names.contains(&b"Helvetica-Bold".to_vec()), "{names:?}");
        assert!(names.contains(&b"Helvetica-Oblique".to_vec()), "{names:?}");
    }
}

mod refusals {
    use super::*;

    #[test]
    fn a_font_that_forbids_embedding_is_refused_with_its_own_code() {
        let mut doc = doc("golden/examples/hello.pdf");
        let restricted = AnnotFont::Font(doc.add_blob(with_fs_type(&fixture("fonts/NotoSans-Regular.ttf"), 0x0002)));
        let op = text(&doc, "note", "Hello").with_fonts(vec![restricted], 12.0);
        assert_eq!(doc.refused(op), "content-font-not-embeddable");
        assert_eq!(added_objects(&doc, &page(&doc)).unwrap(), []);
    }

    #[test]
    fn bytes_that_are_not_a_font_are_invalid_and_unknown_bytes_are_unknown() {
        let mut doc = doc("golden/examples/hello.pdf");
        let junk = AnnotFont::Font(doc.add_blob(b"not a font".to_vec()));
        let op = text(&doc, "note", "Hello");
        assert_eq!(
            doc.refused(op.clone().with_fonts(vec![junk], 12.0)),
            "content-font-invalid"
        );
        let ghost = AnnotFont::Font(laminapdf_ops::BlobId::of(b"never added"));
        assert_eq!(doc.refused(op.with_fonts(vec![ghost], 12.0)), "unknown-blob");
    }

    #[test]
    fn a_box_with_no_area_no_text_or_a_silly_size_is_invalid() {
        let mut doc = doc("golden/examples/hello.pdf");
        let op = text(&doc, "note", "Hello");
        let mut flat = op.clone();
        flat.rect = rect(72.0, 100.0, 72.0, 148.0);
        let mut empty = op.clone();
        empty.runs = vec![TextSpan::plain("  ")];
        for bad in [
            flat,
            empty,
            op.clone().with_fonts(Vec::new(), 0.5),
            op.with_fonts(Vec::new(), 900.0),
        ] {
            assert_eq!(doc.refused(bad), "invalid-op");
        }
        // A fallback is always a font's bytes.
        let two_standard =
            text(&doc, "note", "Hello").with_fonts(vec![HELVETICA, AnnotFont::Standard(StandardFamily::Times)], 12.0);
        assert_eq!(doc.refused(two_standard), "invalid-op");
    }

    #[test]
    fn an_id_in_use_is_refused_and_text_is_not_added_with_add_content_object() {
        let mut doc = doc("golden/examples/hello.pdf");
        doc.applied(text(&doc, "note", "One"));
        assert_eq!(doc.refused(text(&doc, "note", "Two")), "content-id-in-use");
        let block = TextBlock::new(100.0, 20.0, vec![TextSpan::plain("Three")]);
        let wrong = laminapdf_ops::AddContentObject::new(
            id("three"),
            page(&doc),
            ContentObjectSpec::Text(block),
            Matrix::IDENTITY,
            date(),
        );
        assert_eq!(doc.refused(wrong), "invalid-op");
    }

    #[test]
    fn without_a_generator_there_is_no_text() {
        let mut doc = open(&fixture("golden/examples/hello.pdf"));
        let op = text(&doc, "note", "Hello");
        assert_eq!(doc.refused(op), "appearance-unavailable");
    }
}

mod changing {
    use super::*;

    #[test]
    fn a_move_and_new_text_redraw_the_object_and_keep_its_id() {
        let mut doc = doc("golden/examples/hello.pdf");
        doc.applied(text(&doc, "note", "Before"));
        let moved = UpdateContentObject::new(id("note"), page(&doc), date())
            .with_matrix(Matrix::new([1.0, 0.0, 0.0, -1.0, 100.0, 300.0]));
        doc.applied(moved);
        let before = added_objects(&doc, &page(&doc)).unwrap().remove(0);
        assert_eq!(before.bounds, rect(100.0, 252.0, 316.0, 300.0));
        let block = TextBlock::new(216.0, 48.0, vec![TextSpan::plain("After, longer")]);
        doc.applied(UpdateContentObject::new(id("note"), page(&doc), date()).with_text(block.clone()));
        let after = added_objects(&doc, &page(&doc)).unwrap().remove(0);
        assert_eq!(after.object, Some(ContentObjectSpec::Text(block)));
        assert_eq!(after.bounds, before.bounds);
    }

    #[test]
    fn a_patch_for_another_kind_and_alt_text_for_text_are_invalid() {
        let mut doc = doc("golden/examples/hello.pdf");
        doc.applied(text(&doc, "note", "Hello"));
        let base = UpdateContentObject::new(id("note"), page(&doc), date());
        assert_eq!(doc.refused(base.clone().with_crop(None)), "invalid-op");
        assert_eq!(
            doc.refused(base.with_alt(laminapdf_ops::Alt::Text("x".into()))),
            "invalid-op"
        );
        let shape = ContentObjectSpec::Shape {
            path: laminapdf_ops::PathSpec::Rect {
                rect: rect(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
            },
            fill: Some(AnnotColor::Gray(0.5)),
            stroke: None,
        };
        doc.applied(laminapdf_ops::AddContentObject::new(
            id("box"),
            page(&doc),
            shape,
            Matrix::IDENTITY,
            date(),
        ));
        let text = TextBlock::new(10.0, 10.0, vec![TextSpan::plain("x")]);
        assert_eq!(
            doc.refused(UpdateContentObject::new(id("box"), page(&doc), date()).with_text(text)),
            "invalid-op"
        );
    }

    #[test]
    fn a_delete_takes_the_text_and_its_form_off_the_page() {
        let mut doc = doc("golden/examples/hello.pdf");
        doc.applied(text(&doc, "note", "Hello"));
        doc.applied(DeleteContentObject::new(id("note"), page(&doc)));
        assert_eq!(added_objects(&doc, &page(&doc)).unwrap(), []);
        assert_eq!(form_fonts(&doc), []);
    }

    #[test]
    fn undo_and_redo_leave_the_overlay_as_it_was_and_as_it_was_made() {
        let mut doc = doc("golden/examples/hello.pdf");
        let before = common::snapshot(&doc);
        let noto = font(&mut doc, "NotoSans-Regular.ttf");
        doc.applied(text(&doc, "note", "Hello, world").with_fonts(vec![noto], 12.0));
        let added = common::snapshot(&doc);
        let block = TextBlock::new(216.0, 48.0, vec![TextSpan::plain("Changed")]);
        doc.applied(UpdateContentObject::new(id("note"), page(&doc), date()).with_text(block));
        let changed = common::snapshot(&doc);
        doc.undo().unwrap();
        assert_eq!(common::snapshot(&doc), added);
        doc.undo().unwrap();
        assert_eq!(common::snapshot(&doc), before);
        doc.redo().unwrap();
        assert_eq!(common::snapshot(&doc), added);
        doc.redo().unwrap();
        assert_eq!(common::snapshot(&doc), changed);
    }
}

mod tagging {
    use super::*;

    /// The structure elements of the document element that are `role`, as (MCID, page).
    fn elements(doc: &Document, role: &[u8]) -> Vec<i64> {
        let catalog = doc.catalog().unwrap();
        let root = dict(doc, catalog.get("StructTreeRoot").unwrap().as_ref().unwrap());
        let document = dict(doc, root.get("K").unwrap().as_ref().unwrap());
        let kids = document.get("K").and_then(Object::as_array).unwrap().to_vec();
        kids.iter()
            .filter_map(Object::as_ref)
            .map(|k| dict(doc, k))
            .filter(|d| d.get_name("S") == Some(role))
            .map(|d| d.get_int("K").unwrap())
            .collect()
    }

    #[test]
    fn text_in_a_tagged_file_is_a_p_element_that_owns_the_next_mcid() {
        let mut doc = doc("content/add/tagged.pdf");
        doc.applied(text(&doc, "note", "Tagged"));
        // The file's own paragraph owns MCID 0.
        assert_eq!(elements(&doc, b"P"), [0, 1]);
        let last = streams(&doc).pop().unwrap();
        assert!(last.contains("/P <</MCID 1>> BDC"), "{last}");
        // An edit keeps the marked content, and a delete takes the element out.
        let block = TextBlock::new(216.0, 48.0, vec![TextSpan::plain("Still tagged")]);
        doc.applied(UpdateContentObject::new(id("note"), page(&doc), date()).with_text(block));
        assert!(streams(&doc).pop().unwrap().contains("/P <</MCID 1>> BDC"));
        doc.applied(DeleteContentObject::new(id("note"), page(&doc)));
        assert_eq!(elements(&doc, b"P"), [0]);
    }

    #[test]
    fn an_artifact_choice_marks_decoration_and_an_untagged_file_is_left_untagged() {
        let mut doc = doc("content/add/tagged.pdf");
        doc.applied(text(&doc, "note", "Decoration").with_tag(TagChoice::Artifact));
        assert_eq!(elements(&doc, b"P"), [0]);
        assert!(streams(&doc).pop().unwrap().contains("/Artifact <</Type /Layout>> BDC"));
        let mut plain = self::doc("golden/examples/hello.pdf");
        plain.applied(text(&plain, "note", "Untagged"));
        assert!(!streams(&plain).pop().unwrap().contains("BDC"));
    }

    #[test]
    fn a_pdf_ua_file_needs_no_alt_text_for_text() {
        let mut doc = claiming(r#"pdfuaid:part="1""#).with_appearances(AnnotationAppearances);
        doc.applied(text(&doc, "note", "Plain text"));
    }
}

mod wire {
    use super::*;

    #[test]
    fn the_op_reads_and_writes_its_wire_form() {
        let json = r#"{"op":"addText","args":{"id":"note","page":"p3.0","rect":[72.0,72.0,288.0,120.0],"runs":[{"text":"Reviewed "},{"text":"twice","bold":true}],"align":"center","fonts":["helvetica"],"size":14.0,"color":[1.0,0.0,0.0],"tag":"artifact","modified":"D:20261118120000Z"}}"#;
        let op: Op = serde_json::from_str(json).unwrap();
        assert_eq!(serde_json::to_string(&op).unwrap(), json);
    }

    #[test]
    fn unknown_fields_and_bad_values_are_refused() {
        for json in [
            r#"{"op":"addText","args":{"id":"n","page":"p3.0","rect":[0,0,1,1],"runs":[],"extra":1,"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"addText","args":{"id":"n","page":"p3.0","rect":[0,0,1,1],"runs":[{"text":"a","size":9}],"modified":"D:20261118120000Z"}}"#,
            r#"{"op":"addText","args":{"id":"n","page":"p3.0","rect":[0,0,1,1],"runs":[],"tag":"heading","modified":"D:20261118120000Z"}}"#,
            r#"{"op":"addText","args":{"id":"n","page":"p3.0","rect":[0,0,1,1],"runs":[],"fonts":["comic-sans"],"modified":"D:20261118120000Z"}}"#,
        ] {
            assert!(serde_json::from_str::<Op>(json).is_err(), "{json}");
        }
    }
}

mod warnings {
    use laminapdf_ops::{ConformanceWarning, pending_conformance_warnings};

    use super::*;

    fn unembedded(doc: &Document) -> Vec<ConformanceWarning> {
        pending_conformance_warnings(doc, doc.pending_changes())
            .unwrap()
            .into_iter()
            .filter(|w| matches!(w.code(), "pdfa-font-not-embedded"))
            .collect()
    }

    #[test]
    fn a_standard_font_on_a_pdf_a_file_warns_with_its_page_and_an_embedded_one_does_not() {
        let mut doc = claiming(r#"pdfaid:part="2" pdfaid:conformance="B""#).with_appearances(AnnotationAppearances);
        let page = page(&doc).to_string();
        doc.applied(text(&doc, "plain", "Helvetica is never embedded"));
        assert_eq!(
            unembedded(&doc),
            [ConformanceWarning::PdfaFontNotEmbedded {
                annotations: Vec::new(),
                pages: vec![page]
            }]
        );
        doc.undo().unwrap();
        let noto = font(&mut doc, "NotoSans-Regular.ttf");
        doc.applied(text(&doc, "embedded", "A subset is embedded").with_fonts(vec![noto], 12.0));
        assert_eq!(unembedded(&doc), []);
    }
}
