//! The typed views over `fixtures/corpus/outline.pdf` and small hand-built files: pages and
//! their boxes, rotation and references; the outline; Info; name trees; page labels; and
//! the per-field tolerance of ADR 0003 (a malformed field is absent plus a warning).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{classic, fixture, open};
use lamina_cos::ObjRef;
use lamina_model::{
    ActionKind, Document, Error, LabelStyle, OpenOptions, PageId, PointPt, Pt, Rotation, UserSpacePoint, UserSpaceRect,
    View, Warning,
};

fn outline_pdf() -> Document {
    open(&fixture("corpus/outline.pdf")).unwrap()
}

fn codes(doc: &Document) -> Vec<&'static str> {
    doc.warnings().iter().map(Warning::code).collect()
}

/// A one-page file whose page dictionary has `page_extra` (pages: object 2, page: 3).
fn one_page(page_extra: &str) -> Document {
    open(&classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            &format!("<< /Type /Page /Parent 2 0 R {page_extra} >>"),
        ],
        "",
    ))
    .unwrap()
}

mod frame {
    use super::*;

    /// A one-page file without any box (pages: object 2, page: 3).
    fn boxless(page_extra: &str) -> Document {
        open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                &format!("<< /Type /Page /Parent 2 0 R {page_extra} >>"),
            ],
            "",
        ))
        .unwrap()
    }

    #[test]
    fn uses_the_crop_box_rotation_and_user_unit() {
        let doc = one_page("/CropBox [10 20 310 420] /Rotate 90 /UserUnit 2");
        let frame = doc.pages().unwrap().get(0).unwrap().frame().unwrap();
        assert_eq!(
            (frame.width(), frame.height(), frame.is_letter_fallback()),
            (Pt::new(800.0), Pt::new(600.0), false)
        );
        assert_eq!(
            frame.to_page(UserSpacePoint::new(10.0, 20.0)),
            PointPt::new(Pt::ZERO, Pt::ZERO)
        );
    }

    #[test]
    fn falls_back_to_us_letter_with_one_invalid_box_warning_when_no_box_exists() {
        let doc = boxless("");
        let pages = doc.pages().unwrap();
        let page = pages.get(0).unwrap();
        let frame = page.frame().unwrap();
        let again = page.frame().unwrap();
        assert_eq!(
            (frame.crop_box(), frame.is_letter_fallback(), again, codes(&doc)),
            (lamina_model::LETTER, true, frame, vec!["invalid-box"])
        );
    }

    #[test]
    fn falls_back_to_us_letter_when_the_media_box_has_no_area() {
        let doc = boxless("/MediaBox [0 0 0 792]");
        let frame = doc.pages().unwrap().get(0).unwrap().frame().unwrap();
        assert_eq!((frame.width(), frame.height()), (Pt::new(612.0), Pt::new(792.0)));
    }

    #[test]
    fn the_letter_fallback_is_an_error_in_strict_mode() {
        let bytes = classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
            ],
            "",
        );
        let doc = Document::open(bytes, OpenOptions::default().with_strict(true)).unwrap();
        let err = doc.pages().unwrap().get(0).unwrap().frame().unwrap_err();
        assert_eq!(err.code(), "strict");
    }

    #[test]
    fn an_invalid_rotate_is_read_once_per_page() {
        let doc = one_page("/Rotate 45");
        let pages = doc.pages().unwrap();
        let page = pages.get(0).unwrap();
        let rotations = (
            page.rotate().unwrap(),
            page.frame().unwrap().rotate(),
            page.rotate().unwrap(),
        );
        assert_eq!(
            (rotations, codes(&doc)),
            ((Rotation::None, Rotation::None, Rotation::None), vec!["invalid-rotate"])
        );
    }
}

mod pages {
    use super::*;

    /// capability: C.04 core read
    #[test]
    fn ids_are_the_page_objects() {
        let doc = outline_pdf();
        let ids: Vec<String> = doc.pages().unwrap().iter().map(|p| p.id().to_string()).collect();
        assert_eq!(ids, ["p4.0", "p5.0", "p6.0"]);
    }

    #[test]
    fn by_id_finds_the_page() {
        let doc = outline_pdf();
        let pages = doc.pages().unwrap();
        assert_eq!(pages.by_id(&"p6.0".parse().unwrap()).map(|p| p.index()), Some(2));
    }

    #[test]
    fn by_id_of_a_foreign_page_is_none() {
        let doc = outline_pdf();
        assert!(
            doc.pages()
                .unwrap()
                .by_id(&PageId::object(ObjRef::new(99, 0)))
                .is_none()
        );
    }

    #[test]
    fn media_box_is_inherited_and_normalised() {
        let doc = one_page("");
        let page = doc.pages().unwrap();
        assert_eq!(
            page.get(0).unwrap().media_box().unwrap(),
            Some(UserSpaceRect::from_corners(0.0, 0.0, 612.0, 792.0))
        );
    }

    #[test]
    fn rect_corners_are_normalised() {
        let doc = one_page("/MediaBox [612 792 0 0]");
        let media = doc.pages().unwrap().get(0).unwrap().media_box().unwrap().unwrap();
        assert_eq!(media.to_array(), [0.0, 0.0, 612.0, 792.0]);
    }

    #[test]
    fn a_malformed_media_box_is_none_with_a_warning() {
        let doc = one_page("/MediaBox [0 0 (wide) 792]");
        assert_eq!(doc.pages().unwrap().get(0).unwrap().media_box().unwrap(), None);
        assert_eq!(codes(&doc), ["invalid-box"]);
    }

    #[test]
    fn a_malformed_box_is_an_error_in_strict_mode() {
        let bytes = classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1] >>",
            ],
            "",
        );
        let doc = Document::open(bytes, OpenOptions::default().with_strict(true)).unwrap();
        let err = doc.pages().unwrap().get(0).unwrap().media_box().unwrap_err();
        assert_eq!(
            err,
            Error::Strict {
                warning: Warning::InvalidBox {
                    object: Some(3),
                    page: 0,
                    key: "MediaBox"
                }
            }
        );
    }

    #[test]
    fn crop_box_defaults_to_the_media_box() {
        let doc = one_page("");
        let page = doc.pages().unwrap();
        let page = page.get(0).unwrap();
        assert_eq!(page.crop_box().unwrap(), page.media_box().unwrap());
    }

    /// capability: C.09 core read
    #[test]
    fn crop_box_is_clipped_to_the_media_box() {
        let doc = one_page("/CropBox [-10 -10 300 300]");
        let crop = doc.pages().unwrap().get(0).unwrap().crop_box().unwrap();
        assert_eq!(crop, Some(UserSpaceRect::from_corners(0.0, 0.0, 300.0, 300.0)));
    }

    #[test]
    fn trim_box_defaults_to_the_crop_box() {
        let doc = one_page("/CropBox [10 10 300 300]");
        let pages = doc.pages().unwrap();
        let page = pages.get(0).unwrap();
        assert_eq!(page.trim_box().unwrap(), page.crop_box().unwrap());
    }

    /// capability: C.11 core read
    /// capability: K.10 core read
    #[test]
    fn art_and_bleed_boxes_are_read_from_the_page() {
        let doc = one_page("/ArtBox [1 2 3 4] /BleedBox [0 0 600 780]");
        let pages = doc.pages().unwrap();
        let page = pages.get(0).unwrap();
        assert_eq!(
            (page.art_box().unwrap(), page.bleed_box().unwrap()),
            (
                Some(UserSpaceRect::from_corners(1.0, 2.0, 3.0, 4.0)),
                Some(UserSpaceRect::from_corners(0.0, 0.0, 600.0, 780.0))
            )
        );
    }

    /// capability: C.08 core read
    #[test]
    fn rotate_normalises_negative_multiples_of_90() {
        let doc = one_page("/Rotate -90");
        assert_eq!(
            doc.pages().unwrap().get(0).unwrap().rotate().unwrap(),
            Rotation::Clockwise270
        );
    }

    #[test]
    fn rotate_accepts_an_integral_real() {
        let doc = one_page("/Rotate 180.0");
        assert_eq!(doc.pages().unwrap().get(0).unwrap().rotate().unwrap(), Rotation::Half);
    }

    #[test]
    fn rotate_that_is_not_a_multiple_of_90_is_none_with_a_warning() {
        let doc = one_page("/Rotate 45");
        assert_eq!(doc.pages().unwrap().get(0).unwrap().rotate().unwrap(), Rotation::None);
        assert_eq!(codes(&doc), ["invalid-rotate"]);
    }

    #[test]
    fn an_invalid_user_unit_is_one_with_a_warning() {
        let doc = one_page("/UserUnit -3");
        let unit = doc.pages().unwrap().get(0).unwrap().user_unit().unwrap();
        assert_eq!(
            (unit.to_bits(), codes(&doc)),
            (1.0_f64.to_bits(), vec!["invalid-entry"])
        );
    }

    #[test]
    fn two_malformed_boxes_on_one_page_are_both_reported() {
        let doc = one_page("/MediaBox [0 0] /CropBox (x)");
        doc.pages().unwrap().get(0).unwrap().crop_box().unwrap();
        assert_eq!(codes(&doc), ["invalid-box", "invalid-box"]);
    }

    #[test]
    fn malformed_boxes_on_two_direct_pages_are_both_reported() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [<< /Type /Page /MediaBox [1] >> << /Type /Page /MediaBox [2] >>] /Count 2 >>",
            ],
            "",
        ))
        .unwrap();
        for page in doc.pages().unwrap().iter() {
            page.media_box().unwrap();
        }
        assert_eq!(codes(&doc), ["invalid-box", "invalid-box"]);
    }

    #[test]
    fn strict_trim_box_reports_the_trim_box_itself() {
        let bytes = classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1] /TrimBox (x) >>",
            ],
            "",
        );
        let doc = Document::open(bytes, OpenOptions::default().with_strict(true)).unwrap();
        let err = doc.pages().unwrap().get(0).unwrap().trim_box().unwrap_err();
        assert!(
            matches!(
                err,
                Error::Strict {
                    warning: Warning::InvalidBox { key: "TrimBox", .. }
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn user_unit_is_read() {
        let doc = one_page("/UserUnit 2.5");
        assert!((doc.pages().unwrap().get(0).unwrap().user_unit().unwrap() - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn contents_lists_the_stream_references() {
        let doc = one_page("/Contents [7 0 R 8 0 R]");
        let refs = doc.pages().unwrap().get(0).unwrap().contents().unwrap();
        assert_eq!(refs, [ObjRef::new(7, 0), ObjRef::new(8, 0)]);
    }

    #[test]
    fn a_single_contents_reference_is_listed() {
        let doc = outline_pdf();
        assert_eq!(
            doc.pages().unwrap().get(0).unwrap().contents().unwrap(),
            [ObjRef::new(7, 0)]
        );
    }

    #[test]
    fn annots_lists_references_only() {
        let doc = one_page("/Annots [9 0 R << /Subtype /Link >>]");
        assert_eq!(
            doc.pages().unwrap().get(0).unwrap().annots().unwrap(),
            [ObjRef::new(9, 0)]
        );
    }

    #[test]
    fn resources_are_inherited() {
        let doc = outline_pdf();
        let pages = doc.pages().unwrap();
        assert!(pages.get(1).unwrap().resources().unwrap().contains_key("Font"));
    }
}

mod labels {
    use super::*;

    /// capability: C.14 core read
    /// capability: I.10 core read
    #[test]
    fn labels_follow_the_ranges() {
        let doc = outline_pdf();
        let labels: Vec<_> = doc
            .pages()
            .unwrap()
            .iter()
            .map(|p| p.label().unwrap().unwrap())
            .collect();
        assert_eq!(labels, ["i", "A-1", "A-2"]);
    }

    #[test]
    fn ranges_are_exposed_in_order() {
        let doc = outline_pdf();
        let styles: Vec<_> = doc.page_labels().unwrap().ranges().iter().map(|r| r.style).collect();
        assert_eq!(styles, [Some(LabelStyle::LowerRoman), Some(LabelStyle::Decimal)]);
    }

    #[test]
    fn no_page_labels_means_no_label() {
        assert_eq!(one_page("").pages().unwrap().get(0).unwrap().label().unwrap(), None);
    }

    fn labelled(nums: &str) -> Document {
        open(&classic(
            &[
                &format!("<< /Type /Catalog /Pages 2 0 R /PageLabels << /Nums [{nums}] >> >>"),
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
            ],
            "",
        ))
        .unwrap()
    }

    #[test]
    fn a_label_that_is_not_a_dictionary_falls_back_to_decimal_with_a_warning() {
        let doc = labelled("0 (junk)");
        assert_eq!(
            doc.pages().unwrap().get(0).unwrap().label().unwrap().as_deref(),
            Some("1")
        );
        assert_eq!(codes(&doc), ["invalid-page-label"]);
    }

    #[test]
    fn an_unknown_style_keeps_the_prefix_and_start() {
        let doc = labelled("0 << /S /Q /P (x-) /St 3 >>");
        assert_eq!(
            doc.pages().unwrap().get(0).unwrap().label().unwrap().as_deref(),
            Some("x-")
        );
        assert_eq!(codes(&doc), ["invalid-page-label"]);
    }

    #[test]
    fn a_negative_start_is_reported_as_stored() {
        let doc = labelled("-1 << /S /D >> 0 << /S /r >>");
        assert_eq!(
            doc.pages().unwrap().get(0).unwrap().label().unwrap().as_deref(),
            Some("i")
        );
        assert_eq!(doc.warnings(), [Warning::InvalidPageLabel { start: -1 }]);
    }

    #[test]
    fn st_and_letters_combine() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R /PageLabels << /Nums [0 << /S /A /St 26 >>] >> >>",
                "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
                "<< /Type /Page /Parent 2 0 R >>",
                "<< /Type /Page /Parent 2 0 R >>",
            ],
            "",
        ))
        .unwrap();
        let labels: Vec<_> = doc
            .pages()
            .unwrap()
            .iter()
            .map(|p| p.label().unwrap().unwrap())
            .collect();
        assert_eq!(labels, ["Z", "AA"]);
    }
}

mod outline {
    use super::*;

    /// capability: I.01 core read
    #[test]
    fn items_nest_as_in_the_file() {
        let doc = outline_pdf();
        let outline = doc.outline().unwrap();
        let tree: Vec<(String, Vec<String>)> = outline
            .roots()
            .map(|r| (r.title().unwrap(), r.children().map(|c| c.title().unwrap()).collect()))
            .collect();
        assert_eq!(
            tree,
            [
                ("Introduction".into(), vec![]),
                (
                    "Chapter Ł1 — Café".into(),
                    vec!["Section 2.1".into(), "Section 2.2".into()]
                ),
                ("Appendix 𝄞 Ω".into(), vec!["Notes".into()]),
            ]
        );
    }

    #[test]
    fn depth_counts_from_one() {
        let doc = outline_pdf();
        let depths: Vec<usize> = doc.outline().unwrap().iter().map(|i| i.depth()).collect();
        assert_eq!(depths, [1, 1, 2, 2, 1, 2]);
    }

    #[test]
    fn ids_are_the_item_objects() {
        let doc = outline_pdf();
        let ids: Vec<String> = doc
            .outline()
            .unwrap()
            .iter()
            .map(|i| i.id().unwrap().to_string())
            .collect();
        assert_eq!(ids, ["o11.0", "o12.0", "o14.0", "o15.0", "o13.0", "o16.0"]);
    }

    #[test]
    fn open_state_follows_the_sign_of_count() {
        let doc = outline_pdf();
        let outline = doc.outline().unwrap();
        let open: Vec<bool> = outline.roots().map(|i| i.is_open().unwrap()).collect();
        assert_eq!(open, [false, true, false]);
    }

    #[test]
    fn an_explicit_destination_resolves_to_its_page_and_view() {
        let doc = outline_pdf();
        let outline = doc.outline().unwrap();
        let dest = outline.iter().next().unwrap().target().unwrap().unwrap();
        assert_eq!(
            (dest.page.map(|p| p.to_string()), dest.page_index, dest.view),
            (
                Some("p4.0".into()),
                Some(0),
                View::Xyz {
                    left: Some(0.0),
                    top: Some(792.0),
                    zoom: Some(0.0)
                }
            )
        );
    }

    /// capability: I.04 core read
    #[test]
    fn a_named_destination_resolves_through_the_catalog_dests() {
        let doc = outline_pdf();
        let outline = doc.outline().unwrap();
        let dest = outline.iter().nth(1).unwrap().target().unwrap().unwrap();
        assert_eq!(
            (dest.name.as_deref(), dest.page_index, dest.view),
            (Some(&b"chapter2"[..]), Some(1), View::Fit)
        );
    }

    /// capability: I.04 core read
    #[test]
    fn a_goto_action_resolves_through_the_names_tree() {
        let doc = outline_pdf();
        let outline = doc.outline().unwrap();
        let item = outline.iter().nth(4).unwrap();
        let dest = item.target().unwrap().unwrap();
        assert_eq!(
            (item.action().unwrap(), dest.page_index, dest.view),
            (
                Some(ActionKind::GoTo),
                Some(2),
                View::Xyz {
                    left: None,
                    top: None,
                    zoom: None
                }
            )
        );
    }

    #[test]
    fn a_uri_action_has_no_target() {
        let doc = outline_pdf();
        let outline = doc.outline().unwrap();
        let item = outline.iter().nth(3).unwrap();
        assert_eq!(
            (item.action().unwrap(), item.target().unwrap()),
            (Some(ActionKind::Uri), None)
        );
    }

    #[test]
    fn fith_carries_its_top() {
        let doc = outline_pdf();
        let outline = doc.outline().unwrap();
        let dest = outline.iter().nth(2).unwrap().target().unwrap().unwrap();
        assert_eq!(dest.view, View::FitH { top: Some(700.0) });
    }

    #[test]
    fn an_unknown_name_keeps_the_name_unresolved() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
                "<< /First 5 0 R >>",
                "<< /Title (x) /Dest (nowhere) >>",
            ],
            "",
        ))
        .unwrap();
        let outline = doc.outline().unwrap();
        let dest = outline.iter().next().unwrap().target().unwrap().unwrap();
        assert_eq!((dest.name.as_deref(), dest.page), (Some(&b"nowhere"[..]), None));
    }

    #[test]
    fn a_remote_page_number_keeps_the_raw_array() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
                "<< /First 5 0 R >>",
                "<< /Title (x) /Dest [7 /Fit] >>",
            ],
            "",
        ))
        .unwrap();
        let outline = doc.outline().unwrap();
        let dest = outline.iter().next().unwrap().target().unwrap().unwrap();
        assert_eq!((dest.page, dest.raw.as_array().map(<[_]>::len)), (None, Some(2)));
    }

    fn with_outline(item: &str, catalog_extra: &str) -> Document {
        open(&classic(
            &[
                &format!("<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R {catalog_extra} >>"),
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
                "<< /First 5 0 R >>",
                item,
            ],
            "",
        ))
        .unwrap()
    }

    #[test]
    fn a_named_destination_with_a_malformed_value_keeps_its_name() {
        let doc = with_outline("<< /Title (x) /Dest /bad >>", "/Dests << /bad (junk) >>");
        let outline = doc.outline().unwrap();
        let dest = outline.iter().next().unwrap().target().unwrap().unwrap();
        assert_eq!(
            (dest.name.as_deref(), dest.page, dest.raw.clone()),
            (
                Some(&b"bad"[..]),
                None,
                lamina_cos::Object::String(lamina_cos::PdfString::literal("junk"))
            )
        );
    }

    #[test]
    fn a_null_dest_falls_back_to_the_goto_action() {
        let doc = with_outline("<< /Title (x) /Dest null /A << /S /GoTo /D [3 0 R /Fit] >> >>", "");
        let outline = doc.outline().unwrap();
        let dest = outline.iter().next().unwrap().target().unwrap().unwrap();
        assert_eq!(dest.page_index, Some(0));
    }

    #[test]
    fn a_malformed_outlines_entry_is_reported() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Outlines (junk) >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
            ],
            "",
        ))
        .unwrap();
        assert_eq!((doc.outline().unwrap().len(), codes(&doc)), (0, vec!["invalid-entry"]));
    }

    #[test]
    fn outline_from_pages_resolves_like_the_document_outline() {
        let doc = outline_pdf();
        let pages = doc.pages().unwrap();
        let shared: Vec<_> = pages.outline().unwrap().iter().map(|i| i.target().unwrap()).collect();
        let own: Vec<_> = doc.outline().unwrap().iter().map(|i| i.target().unwrap()).collect();
        assert_eq!(shared, own);
    }

    #[test]
    fn by_id_finds_an_item() {
        let doc = outline_pdf();
        let outline = doc.outline().unwrap();
        let item = outline.by_id(&"o14.0".parse().unwrap()).unwrap();
        assert_eq!(item.title().unwrap(), "Section 2.1");
    }

    #[test]
    fn a_cycle_stops_each_looping_branch_with_a_warning() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
                "<< /First 5 0 R >>",
                "<< /Title (a) /Next 6 0 R >>",
                "<< /Title (b) /Next 5 0 R /First 6 0 R >>",
            ],
            "",
        ))
        .unwrap();
        let titles = common::outline_titles(&doc).unwrap();
        assert_eq!(
            (titles, codes(&doc)),
            (vec!["a".to_owned(), "b".to_owned()], vec!["outline-cycle"; 2])
        );
    }

    #[test]
    fn a_non_dictionary_item_is_skipped_with_a_warning() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
                "<< /First 5 0 R >>",
                "(not an item)",
            ],
            "",
        ))
        .unwrap();
        assert_eq!(
            (doc.outline().unwrap().len(), codes(&doc)),
            (0, vec!["outline-item-skipped"])
        );
    }

    #[test]
    fn direct_items_get_path_ids() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
                "<< /First << /Title (a) /Next << /Title (b) >> >> >>",
            ],
            "",
        ))
        .unwrap();
        let ids: Vec<String> = doc
            .outline()
            .unwrap()
            .iter()
            .map(|i| i.id().unwrap().to_string())
            .collect();
        assert_eq!(ids, ["o4.0/First", "o4.0/First/Next"]);
    }

    #[test]
    fn no_outlines_means_an_empty_outline() {
        assert!(one_page("").outline().unwrap().is_empty());
    }
}

mod info {
    use super::*;

    /// capability: J.01 core read
    #[test]
    fn text_fields_are_decoded() {
        let doc = outline_pdf();
        let info = doc.info().unwrap().unwrap();
        assert_eq!(
            (info.title().unwrap(), info.author().unwrap(), info.subject().unwrap()),
            (Some("Outline fixture".into()), Some("lamina".into()), None)
        );
    }

    /// capability: J.02 core read
    #[test]
    fn a_valid_date_is_parsed() {
        let doc = outline_pdf();
        let date = doc.info().unwrap().unwrap().creation_date().unwrap().unwrap();
        assert_eq!(date.parsed.map(|d| d.to_string()).as_deref(), Some("D:20261028120000Z"));
    }

    #[test]
    fn an_invalid_date_keeps_the_raw_string_with_a_warning() {
        let doc = outline_pdf();
        let date = doc.info().unwrap().unwrap().mod_date().unwrap().unwrap();
        assert_eq!(
            (date.raw.as_str(), date.parsed, codes(&doc)),
            ("yesterday", None, vec!["invalid-date"])
        );
    }

    #[test]
    fn two_invalid_dates_are_both_reported() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [] /Count 0 >>",
                "<< /CreationDate (soon) /ModDate (later) >>",
            ],
            "/Info 3 0 R",
        ))
        .unwrap();
        let info = doc.info().unwrap().unwrap();
        info.creation_date().unwrap();
        info.mod_date().unwrap();
        assert_eq!(codes(&doc), ["invalid-date", "invalid-date"]);
    }

    #[test]
    fn an_info_that_is_not_a_dictionary_is_none_with_a_warning() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [] /Count 0 >>",
                "(junk)",
            ],
            "/Info 3 0 R",
        ))
        .unwrap();
        assert_eq!(
            (doc.info().unwrap().is_none(), codes(&doc)),
            (true, vec!["invalid-entry"])
        );
    }

    #[test]
    fn no_info_is_none() {
        assert!(one_page("").info().unwrap().is_none());
    }
}

mod trees {
    use super::*;

    fn names_doc(tree: &str) -> Document {
        open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Names << /Dests 4 0 R >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
                tree,
            ],
            "",
        ))
        .unwrap()
    }

    #[test]
    fn entries_come_in_tree_order() {
        let doc = names_doc("<< /Kids [<< /Limits [(a) (b)] /Names [(a) 1 (b) 2] >> << /Names [(c) 3] >>] >>");
        let keys: Vec<Vec<u8>> = doc
            .name_tree("Dests")
            .unwrap()
            .unwrap()
            .entries()
            .unwrap()
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(keys, [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn get_finds_a_key_in_a_kid() {
        let doc = names_doc(
            "<< /Kids [<< /Limits [(a) (b)] /Names [(a) 1 (b) 2] >> << /Limits [(c) (d)] /Names [(c) 3] >>] >>",
        );
        assert_eq!(
            doc.name_tree("Dests")
                .unwrap()
                .unwrap()
                .get(b"c")
                .unwrap()
                .and_then(|v| v.as_int()),
            Some(3)
        );
    }

    #[test]
    fn malformed_limits_do_not_hide_entries() {
        let doc = names_doc("<< /Kids [<< /Limits (junk) /Names [(z) 26] >>] >>");
        assert_eq!(
            doc.name_tree("Dests")
                .unwrap()
                .unwrap()
                .get(b"z")
                .unwrap()
                .and_then(|v| v.as_int()),
            Some(26)
        );
    }

    #[test]
    fn a_bad_kid_is_skipped_with_a_warning() {
        let doc = names_doc("<< /Kids [(junk) << /Names [(a) 1] >>] >>");
        let entries = doc.name_tree("Dests").unwrap().unwrap().entries().unwrap();
        assert_eq!((entries.len(), codes(&doc)), (1, vec!["name-tree-node-skipped"]));
    }

    #[test]
    fn a_self_referencing_kid_is_skipped_with_a_warning() {
        let doc = names_doc("<< /Kids [4 0 R] /Names [(a) 1] >>");
        let entries = doc.name_tree("Dests").unwrap().unwrap().entries().unwrap();
        assert_eq!((entries.len(), codes(&doc)), (1, vec!["name-tree-node-skipped"]));
    }

    #[test]
    fn get_stops_at_the_first_match() {
        let doc = names_doc("<< /Kids [<< /Names [(a) 1] >> (junk)] >>");
        let found = doc.name_tree("Dests").unwrap().unwrap().get(b"a").unwrap();
        assert_eq!((found.and_then(|v| v.as_int()), codes(&doc)), (Some(1), vec![]));
    }

    #[test]
    fn a_malformed_names_dictionary_is_reported() {
        let doc = open(&classic(
            &[
                "<< /Type /Catalog /Pages 2 0 R /Names (junk) >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R >>",
            ],
            "",
        ))
        .unwrap();
        assert_eq!(
            (doc.name_tree("Dests").unwrap().is_none(), codes(&doc)),
            (true, vec!["invalid-entry"])
        );
    }

    #[test]
    fn number_trees_look_up_integer_keys() {
        let doc = one_page("");
        let tree = doc.number_tree(lamina_cos::Object::Dict(
            [(
                "Nums",
                lamina_cos::Object::Array(vec![1.into(), 10.into(), 5.into(), 50.into()]),
            )]
            .into_iter()
            .collect(),
        ));
        assert_eq!(tree.get(5).unwrap().and_then(|v| v.as_int()), Some(50));
    }
}

#[test]
fn view_warnings_are_raised_once_per_object() {
    let doc = one_page("/MediaBox [0 0]");
    let pages = doc.pages().unwrap();
    for _ in 0..3 {
        pages.get(0).unwrap().media_box().unwrap();
    }
    assert_eq!(codes(&doc), ["invalid-box"]);
}
