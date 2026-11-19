//! `AddContentObject`, `UpdateContentObject` and `DeleteContentObject` (E6 plan §2.1, todo E6b):
//! what each writes on a page, how `added_objects` finds the objects again, the stacking order,
//! tagging, the refusals, and the PDF/A warnings.
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
use laminapdf_model::{Matrix, PageId, PdfDate, PointPt, Pt, RectPt};
use laminapdf_ops::{
    AddContentObject, AddedObject, Alt, AnnotColor, Applied, ContentId, ContentLayer, ContentObjectSpec,
    DeleteContentObject, Document, Op, Opacity, Origin, PathSpec, Stroke, StrokeDash, UpdateContentObject,
    added_objects,
};

fn doc(name: &str) -> Document {
    open(&fixture(name)).with_appearances(AnnotationAppearances)
}

/// Applying one op, as methods so the op can be built from the document it applies to.
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

/// `w` by `h` points with the top-left corner at `(x, y)`, for an image or a page.
fn at(x: f64, y: f64, w: f64, h: f64) -> Matrix {
    Matrix::new([w, 0.0, 0.0, h, x, y])
}

const IDENTITY: Matrix = Matrix::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

fn image(doc: &mut Document, name: &str) -> ContentObjectSpec {
    ContentObjectSpec::Image {
        blob: doc.add_blob(fixture(name)),
        crop: None,
    }
}

fn square(fill: Option<AnnotColor>) -> ContentObjectSpec {
    ContentObjectSpec::Shape {
        path: PathSpec::Rect {
            rect: rect(72.0, 72.0, 172.0, 172.0),
            radius: 0.0,
        },
        fill,
        stroke: Some(Stroke::new(AnnotColor::Gray(0.0), 2.0)),
    }
}

fn add(doc: &Document, name: &str, object: ContentObjectSpec, matrix: Matrix) -> AddContentObject {
    AddContentObject::new(id(name), page(doc), object, matrix, date())
}

fn listed(doc: &Document) -> Vec<AddedObject> {
    added_objects(doc, &page(doc)).unwrap()
}

fn ids(doc: &Document) -> Vec<String> {
    listed(doc).iter().map(|o| o.id.to_string()).collect()
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

mod adding {
    use super::*;

    #[test]
    fn an_image_is_its_own_form_drawn_by_one_stream_after_the_wrapped_content() {
        let mut doc = doc("golden/examples/hello.pdf");
        let object = image(&mut doc, "images/photo-rgb.jpg");
        let applied = doc.applied(add(&doc, "photo", object.clone(), at(72.0, 72.0, 144.0, 96.0)));
        assert_eq!(applied.touched_pages.len(), 1);
        let all = streams(&doc);
        assert_eq!(all[0].trim(), "q");
        let last = all.last().unwrap();
        // The page is A4: y runs up from its bottom in user space.
        assert!(last.starts_with("q 144 0 0 -96 72 769.889764 cm /Fm"), "{last}");
        assert!(last.trim_end().ends_with("Do Q"), "{last}");
        let objects = listed(&doc);
        assert_eq!(objects.len(), 1);
        let photo = &objects[0];
        assert_eq!(
            (&photo.id, photo.object.as_ref(), photo.layer, photo.bounds),
            (
                &id("photo"),
                Some(&object),
                ContentLayer::Front,
                rect(72.0, 72.0, 216.0, 168.0)
            )
        );
        assert_eq!(photo.matrix, at(72.0, 72.0, 144.0, 96.0));
    }

    #[test]
    fn the_form_names_the_object_in_its_page_piece_dictionary() {
        let mut doc = doc("golden/examples/hello.pdf");
        let object = square(None);
        doc.applied(add(&doc, "box", object, IDENTITY));
        let pages = doc.pages().unwrap();
        let resources = pages.get(0).unwrap().resources().cloned().unwrap();
        let xobjects = doc
            .deref(resources.get("XObject").unwrap())
            .unwrap()
            .as_dict()
            .cloned()
            .unwrap();
        let (_, form) = xobjects.iter().next().unwrap();
        let form = dict(&doc, form.as_ref().unwrap());
        let lamina = form
            .get("PieceInfo")
            .and_then(Object::as_dict)
            .unwrap()
            .get("LaminaPDF");
        let private = lamina
            .and_then(Object::as_dict)
            .unwrap()
            .get("Private")
            .and_then(Object::as_dict)
            .unwrap();
        assert_eq!(private.get_name("Kind"), Some(b"Shape".as_slice()));
        assert!(form.get("LastModified").is_some());
        // The stroke reaches a width past the square: the box holds it.
        assert_eq!(
            form.get("BBox").cloned(),
            Some(Object::Array([70, 70, 174, 174].map(Object::from).into()))
        );
    }

    #[test]
    fn a_png_with_transparency_gets_a_soft_mask_and_a_page_of_a_pdf_its_content() {
        let mut doc = doc("golden/examples/hello.pdf");
        let png = image(&mut doc, "content/add/logo-alpha.png");
        let logo = ContentObjectSpec::Form {
            blob: doc.add_blob(fixture("content/add/logo.pdf")),
            page: 0,
        };
        doc.applied(add(&doc, "fade", png, at(72.0, 300.0, 128.0, 64.0)));
        doc.applied(add(&doc, "logo", logo, at(300.0, 72.0, 120.0, 60.0)));
        assert_eq!(ids(&doc), ["fade", "logo"]);
        let common::Drawn { images, forms } = common::drawn_xobjects(&doc);
        assert!(images.iter().any(|d| d.get("SMask").is_some()), "{images:?}");
        assert!(
            forms
                .iter()
                .any(|d| d.get("Group").is_none() && d.get("BBox").is_some())
        );
    }

    #[test]
    fn behind_objects_go_under_the_content_each_over_the_last() {
        let mut doc = doc("golden/examples/hello.pdf");
        for name in ["back", "middle"] {
            let op = add(&doc, name, square(Some(AnnotColor::Gray(0.9))), IDENTITY).with_layer(ContentLayer::Behind);
            doc.applied(op);
        }
        doc.applied(add(&doc, "front", square(None), IDENTITY));
        let objects = listed(&doc);
        assert_eq!(
            objects.iter().map(|o| (o.id.to_string(), o.layer)).collect::<Vec<_>>(),
            [
                ("back".to_owned(), ContentLayer::Behind),
                ("middle".to_owned(), ContentLayer::Behind),
                ("front".to_owned(), ContentLayer::Front)
            ]
        );
        // The wrap's `q`, then the two behind, then the page's own content.
        let all = streams(&doc);
        assert!(
            all[1].contains("Do") && all[2].contains("Do") && !all[3].contains(" Do Q"),
            "{all:?}"
        );
    }

    #[test]
    fn a_page_turned_and_off_the_origin_takes_page_space_as_shown() {
        let mut doc = doc("content/add/rotated.pdf");
        doc.applied(add(&doc, "box", square(None), at(10.0, 20.0, 1.0, 1.0)));
        assert_eq!(listed(&doc)[0].matrix, at(10.0, 20.0, 1.0, 1.0));
        assert_eq!(listed(&doc)[0].bounds, rect(80.0, 90.0, 184.0, 194.0));
    }

    #[test]
    fn content_that_leaves_a_q_open_is_wrapped_first() {
        let mut doc = doc("content/add/unbalanced.pdf");
        doc.applied(add(&doc, "box", square(None), IDENTITY));
        let all = streams(&doc);
        assert_eq!(all[all.len() - 2].trim(), "Q\nQ\nQ");
        assert_eq!(ids(&doc), ["box"]);
    }
}

mod refusals {
    use super::*;

    #[test]
    fn an_id_the_page_has_is_in_use() {
        let mut doc = doc("golden/examples/hello.pdf");
        doc.applied(add(&doc, "box", square(None), IDENTITY));
        assert_eq!(
            doc.refused(add(&doc, "box", square(None), IDENTITY)),
            "content-id-in-use"
        );
    }

    #[test]
    fn bytes_that_arent_an_image_or_a_pdf_with_the_page_are_invalid() {
        let mut doc = doc("golden/examples/hello.pdf");
        let text = ContentObjectSpec::Image {
            blob: doc.add_blob(b"not an image".to_vec()),
            crop: None,
        };
        let missing = ContentObjectSpec::Form {
            blob: doc.add_blob(fixture("content/add/logo.pdf")),
            page: 1,
        };
        let unknown = ContentObjectSpec::Image {
            blob: laminapdf_ops::BlobId::of(b"never added"),
            crop: None,
        };
        assert_eq!(
            [
                doc.refused(add(&doc, "a", text, at(0.0, 0.0, 10.0, 10.0))),
                doc.refused(add(&doc, "b", missing, at(0.0, 0.0, 10.0, 10.0))),
                doc.refused(add(&doc, "c", unknown, at(0.0, 0.0, 10.0, 10.0))),
                doc.refused(add(&doc, "d", square(None), Matrix::new([0.0; 6]))),
            ],
            [
                "content-object-invalid",
                "content-object-invalid",
                "unknown-blob",
                "invalid-op"
            ]
        );
        assert_eq!(listed(&doc), []);
    }

    #[test]
    fn an_id_the_page_doesnt_draw_is_unknown() {
        let mut doc = doc("golden/examples/hello.pdf");
        let page = page(&doc);
        assert_eq!(
            [
                doc.refused(DeleteContentObject::new(id("ghost"), page.clone())),
                doc.refused(UpdateContentObject::new(id("ghost"), page, date())),
            ],
            ["unknown-content-object"; 2]
        );
    }

    #[test]
    fn a_patch_for_another_kind_is_invalid() {
        let mut doc = doc("golden/examples/hello.pdf");
        doc.applied(add(&doc, "box", square(None), IDENTITY));
        let crop = UpdateContentObject::new(id("box"), page(&doc), date()).with_crop(Some(rect(0.0, 0.0, 0.5, 0.5)));
        assert_eq!(doc.refused(crop), "invalid-op");
    }

    #[test]
    fn a_stream_another_app_changed_is_the_pages_own_content() {
        let piece = |id: &str| {
            format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> /LastModified (D:20261118120000Z) \
                 /PieceInfo << /LaminaPDF << /LastModified (D:20261118120000Z) /Private << /Id ({id}) /Kind /Shape >> >> >>"
            )
        };
        let drawn = common::stream_body("", b"q 1 0 0 1 0 0 cm /Fm1 Do Q");
        let changed = common::stream_body("", b"0 0 1 rg q 1 0 0 1 0 0 cm /Fm2 Do Q");
        let kept = common::stream_body(&piece("kept"), b"0 0 10 10 re f");
        let other = common::stream_body(&piece("changed"), b"0 0 10 10 re f");
        let bytes = common::classic(
            &[
                b"<< /Type /Catalog /Pages 2 0 R >>",
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents [4 0 R 5 0 R] /Resources << /XObject << /Fm1 6 0 R /Fm2 7 0 R >> >> >>",
                &drawn,
                &changed,
                &kept,
                &other,
            ],
            "<< /Size 8 /Root 1 0 R >>",
        );
        let doc = open(&bytes);
        assert_eq!(ids(&doc), ["kept"]);
    }
}

mod changing {
    use super::*;

    #[test]
    fn a_move_and_a_fade_rewrite_only_the_drawing_stream() {
        let mut doc = doc("golden/examples/hello.pdf");
        let object = image(&mut doc, "images/photo-rgb.jpg");
        doc.applied(add(&doc, "photo", object, at(72.0, 72.0, 144.0, 96.0)));
        let before = listed(&doc)[0].clone();
        let op = UpdateContentObject::new(id("photo"), page(&doc), date())
            .with_matrix(at(100.0, 100.0, 72.0, 48.0))
            .with_opacity(Opacity::new(0.5).unwrap());
        doc.applied(op);
        let after = listed(&doc)[0].clone();
        assert_eq!(
            (after.matrix, after.opacity, after.object),
            (at(100.0, 100.0, 72.0, 48.0), Opacity::new(0.5).unwrap(), before.object)
        );
        assert!(streams(&doc).last().unwrap().contains(" gs "));
    }

    #[test]
    fn a_crop_changes_the_clip_and_keeps_the_image() {
        let mut doc = doc("golden/examples/hello.pdf");
        let object = image(&mut doc, "images/photo-rgb.jpg");
        doc.applied(add(&doc, "photo", object, at(72.0, 72.0, 200.0, 100.0)));
        let crop = rect(0.25, 0.0, 0.75, 1.0);
        doc.applied(UpdateContentObject::new(id("photo"), page(&doc), date()).with_crop(Some(crop)));
        let photo = &listed(&doc)[0];
        assert_eq!(photo.bounds, rect(122.0, 72.0, 222.0, 172.0));
        let Some(ContentObjectSpec::Image { crop: kept, .. }) = &photo.object else {
            panic!("image");
        };
        assert_eq!(*kept, Some(crop));
        doc.applied(UpdateContentObject::new(id("photo"), page(&doc), date()).with_crop(None));
        assert_eq!(listed(&doc)[0].bounds, rect(72.0, 72.0, 272.0, 172.0));
    }

    #[test]
    fn a_new_fill_and_stroke_draw_the_form_again() {
        let mut doc = doc("golden/examples/hello.pdf");
        doc.applied(add(&doc, "box", square(None), IDENTITY));
        let dotted = Stroke::new(AnnotColor::Rgb(0.0, 0.0, 1.0), 4.0).with_dash(StrokeDash::Dotted);
        let op = UpdateContentObject::new(id("box"), page(&doc), date())
            .with_fill(Some(AnnotColor::Rgb(1.0, 0.0, 0.0)))
            .with_stroke(Some(dotted.clone()));
        doc.applied(op);
        let Some(ContentObjectSpec::Shape { fill, stroke, .. }) = listed(&doc)[0].object.clone() else {
            panic!("shape");
        };
        assert_eq!((fill, stroke), (Some(AnnotColor::Rgb(1.0, 0.0, 0.0)), Some(dotted)));
        assert_eq!(listed(&doc)[0].bounds, rect(68.0, 68.0, 176.0, 176.0));
    }

    #[test]
    fn order_moves_an_object_among_those_on_its_layer() {
        let mut doc = doc("golden/examples/hello.pdf");
        for name in ["a", "b", "c"] {
            doc.applied(add(&doc, name, square(None), IDENTITY));
        }
        let order =
            |doc: &Document, name: &str, order| UpdateContentObject::new(id(name), page(doc), date()).with_order(order);
        doc.applied(order(&doc, "a", -1));
        assert_eq!(ids(&doc), ["b", "c", "a"]);
        doc.applied(order(&doc, "a", 0));
        assert_eq!(ids(&doc), ["a", "b", "c"]);
        doc.applied(order(&doc, "c", 1));
        assert_eq!(ids(&doc), ["a", "c", "b"]);
    }

    #[test]
    fn a_delete_takes_the_drawing_and_the_form_off_the_page() {
        let mut doc = doc("golden/examples/hello.pdf");
        doc.applied(add(&doc, "box", square(None), IDENTITY));
        let count = streams(&doc).len();
        let applied = doc.applied(DeleteContentObject::new(id("box"), page(&doc)));
        assert!(applied.still_in_earlier_revisions);
        assert_eq!((listed(&doc).len(), streams(&doc).len()), (0, count - 1));
        let pages = doc.pages().unwrap();
        let resources = pages.get(0).unwrap().resources().cloned().unwrap_or_default();
        assert!(resources.get("XObject").is_none(), "{resources:?}");
    }

    #[test]
    fn undo_leaves_the_overlay_empty() {
        let mut doc = doc("golden/examples/hello.pdf");
        let before = common::snapshot(&doc);
        let object = image(&mut doc, "images/photo-rgb.jpg");
        doc.applied(add(&doc, "photo", object, at(72.0, 72.0, 144.0, 96.0)));
        doc.undo().unwrap();
        assert_eq!(common::snapshot(&doc), before);
    }
}

mod tagging {
    use super::*;

    fn figures(doc: &Document) -> Vec<(String, i64)> {
        let catalog = doc.catalog().unwrap();
        let root = dict(doc, catalog.get("StructTreeRoot").unwrap().as_ref().unwrap());
        let document = dict(doc, root.get("K").unwrap().as_ref().unwrap());
        let kids = document.get("K").and_then(Object::as_array).unwrap().to_vec();
        kids.iter()
            .filter_map(Object::as_ref)
            .map(|k| dict(doc, k))
            .filter(|d| d.get_name("S") == Some(b"Figure".as_slice()))
            .map(|d| {
                let Some(Object::String(alt)) = d.get("Alt") else {
                    panic!("alt");
                };
                (
                    String::from_utf8_lossy(&alt.bytes).into_owned(),
                    d.get_int("K").unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn alt_text_makes_a_figure_that_owns_the_next_mcid_and_decoration_an_artifact() {
        let mut doc = doc("content/add/tagged.pdf");
        let object = image(&mut doc, "images/photo-rgb.jpg");
        let figure = add(&doc, "photo", object, at(72.0, 72.0, 144.0, 96.0)).with_alt(Alt::Text("A photo".into()));
        doc.applied(figure);
        doc.applied(add(&doc, "rule", square(None), IDENTITY).with_alt(Alt::Decorative));
        assert_eq!(figures(&doc), [("A photo".to_owned(), 1)]);
        let all = streams(&doc);
        assert!(all[all.len() - 2].contains("/Figure <</MCID 1>> BDC"), "{all:?}");
        assert!(
            all[all.len() - 1].contains("/Artifact <</Type /Layout>> BDC"),
            "{all:?}"
        );
        let objects = listed(&doc);
        assert_eq!(objects[0].alt, Some(Alt::Text("A photo".into())));
        assert_eq!(objects[1].alt, Some(Alt::Decorative));
    }

    #[test]
    fn new_alt_text_changes_the_figure_and_a_delete_takes_it_out() {
        let mut doc = doc("content/add/tagged.pdf");
        let object = image(&mut doc, "images/photo-rgb.jpg");
        let figure = add(&doc, "photo", object, at(72.0, 72.0, 144.0, 96.0)).with_alt(Alt::Text("A photo".into()));
        doc.applied(figure);
        let alt = UpdateContentObject::new(id("photo"), page(&doc), date()).with_alt(Alt::Text("The office".into()));
        doc.applied(alt);
        assert_eq!(figures(&doc), [("The office".to_owned(), 1)]);
        doc.applied(DeleteContentObject::new(id("photo"), page(&doc)));
        assert_eq!(figures(&doc), []);
    }

    #[test]
    fn a_pdf_ua_file_needs_alt_text_for_a_picture() {
        let mut doc = common::claiming(r#"pdfuaid:part="1""#).with_appearances(AnnotationAppearances);
        let object = image(&mut doc, "images/photo-rgb.jpg");
        let bare = add(&doc, "photo", object, at(72.0, 72.0, 144.0, 96.0));
        assert_eq!(doc.refused(bare.clone()), "content-needs-alt-text");
        doc.applied(bare.with_alt(Alt::Decorative));
        // A shape is decoration unless given text.
        doc.applied(add(&doc, "box", square(None), IDENTITY));
    }
}

mod warnings {
    use laminapdf_ops::{ConformanceWarning, pending_conformance_warnings};

    use super::*;

    fn warnings(doc: &Document) -> Vec<ConformanceWarning> {
        pending_conformance_warnings(doc, doc.pending_changes())
            .unwrap()
            .into_iter()
            .filter(|w| matches!(w.code(), "pdfa-transparency" | "pdfa-font-not-embedded"))
            .collect()
    }

    #[test]
    fn a_see_through_object_on_pdf_a_1_warns_with_its_page() {
        let mut doc =
            common::claiming(r#"pdfaid:part="1" pdfaid:conformance="B""#).with_appearances(AnnotationAppearances);
        let page = page(&doc).to_string();
        let op = add(&doc, "box", square(None), IDENTITY).with_opacity(Opacity::new(0.5).unwrap());
        doc.applied(op);
        assert_eq!(
            warnings(&doc),
            [ConformanceWarning::PdfaTransparency {
                annotations: Vec::new(),
                pages: vec![page]
            }]
        );
        doc.undo().unwrap();
        assert_eq!(warnings(&doc), []);
    }

    #[test]
    fn a_png_with_alpha_on_pdf_a_1_warns_and_on_pdf_a_2_doesnt() {
        for (part, warns) in [("1", true), ("2", false)] {
            let claim = format!(r#"pdfaid:part="{part}" pdfaid:conformance="B""#);
            let mut doc = common::claiming(&claim).with_appearances(AnnotationAppearances);
            let png = image(&mut doc, "content/add/logo-alpha.png");
            doc.applied(add(&doc, "fade", png, at(72.0, 72.0, 128.0, 64.0)));
            assert_eq!(!warnings(&doc).is_empty(), warns, "PDF/A-{part}");
        }
    }

    #[test]
    fn a_page_whose_font_isnt_embedded_warns_on_pdf_a() {
        let mut doc =
            common::claiming(r#"pdfaid:part="2" pdfaid:conformance="B""#).with_appearances(AnnotationAppearances);
        let page = page(&doc).to_string();
        let logo = ContentObjectSpec::Form {
            blob: doc.add_blob(fixture("content/add/logo.pdf")),
            page: 0,
        };
        doc.applied(add(&doc, "logo", logo, at(72.0, 72.0, 120.0, 60.0)).with_alt(Alt::Decorative));
        assert_eq!(
            warnings(&doc),
            [ConformanceWarning::PdfaFontNotEmbedded {
                annotations: Vec::new(),
                pages: vec![page]
            }]
        );
    }
}

#[test]
fn a_line_an_ellipse_and_a_polygon_each_draw() {
    let mut doc = doc("golden/examples/hello.pdf");
    let stroke = Some(Stroke::new(AnnotColor::Gray(0.0), 1.0).with_dash(StrokeDash::Dashed));
    let p = PointPt::from_points;
    let shapes = [
        PathSpec::Line {
            from: p(72.0, 400.0),
            to: p(300.0, 420.0),
        },
        PathSpec::Ellipse {
            rect: rect(72.0, 450.0, 200.0, 520.0),
        },
        PathSpec::Polygon {
            points: vec![p(300.0, 450.0), p(380.0, 450.0), p(340.0, 520.0)],
            closed: true,
        },
    ];
    for (i, path) in shapes.into_iter().enumerate() {
        let fill = (!matches!(path, PathSpec::Line { .. })).then_some(AnnotColor::Rgb(0.2, 0.6, 0.2));
        let object = ContentObjectSpec::Shape {
            path,
            fill,
            stroke: stroke.clone(),
        };
        doc.applied(add(&doc, &format!("s{i}"), object, IDENTITY));
    }
    assert_eq!(ids(&doc), ["s0", "s1", "s2"]);
}

mod properties {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        /// Any shape, placed anywhere, faded or not, in front or behind, then undone, leaves
        /// the overlay as it was; and a change or a delete of it undone leaves the overlay as
        /// the add left it.
        #[test]
        fn every_op_undone_leaves_the_overlay_as_it_was(
            x in 0.0..500.0_f64,
            y in 0.0..700.0_f64,
            w in 1.0..200.0_f64,
            h in 1.0..200.0_f64,
            turn in 0.0..360.0_f64,
            opacity in 0.0..=1.0_f64,
            behind in any::<bool>(),
            delete in any::<bool>(),
        ) {
            let mut doc = doc("golden/examples/hello.pdf");
            let before = common::snapshot(&doc);
            let object = ContentObjectSpec::Shape {
                path: PathSpec::Ellipse { rect: rect(0.0, 0.0, w, h) },
                fill: Some(AnnotColor::Gray(0.5)),
                stroke: None,
            };
            let matrix = Matrix::rotate(turn).then(&Matrix::translate(x, y));
            let layer = if behind { ContentLayer::Behind } else { ContentLayer::Front };
            let add = add(&doc, "shape", object, matrix)
                .with_opacity(Opacity::new(opacity).unwrap())
                .with_layer(layer);
            doc.applied(add);
            let added = common::snapshot(&doc);
            let page = page(&doc);
            let second: Op = if delete {
                DeleteContentObject::new(id("shape"), page).into()
            } else {
                UpdateContentObject::new(id("shape"), page, date())
                    .with_matrix(Matrix::translate(y, x))
                    .with_opacity(Opacity::new(1.0 - opacity).unwrap())
                    .into()
            };
            doc.applied(second);
            doc.undo().unwrap();
            prop_assert_eq!(common::snapshot(&doc), added);
            doc.undo().unwrap();
            prop_assert_eq!(common::snapshot(&doc), before);
        }
    }
}
