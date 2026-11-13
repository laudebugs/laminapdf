//! `SetInitialView` (ISO 32000-2 §12.2, §7.7.2): the frozen sample, the patch semantics, the
//! open action that is never replaced, the version raise, and undo/redo (ADR 0006 §3).
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

mod common;

use common::{classic_with_root, corpus, fixture, open, snapshot};
use laminapdf_model::{ActionKind, OpenAction, PageLayout, PageMode, View, WindowPreferences};
use laminapdf_ops::{
    Document, Error, ModificationClass, Op, OpenAt, OpenZoom, Origin, SetInitialView, WindowPreferencesPatch,
};

/// Three pages (3, 4, 5) under 2; the catalog holds `catalog_extra`, the header says 1.`minor`.
fn doc(catalog_extra: &str, extra: &[&str]) -> Document {
    let catalog = format!("<< /Type /Catalog /Pages 2 0 R {catalog_extra} >>");
    let mut objects = vec![
        catalog.as_str(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 /MediaBox [0 0 600 800] >>",
        "<< /Type /Page /Parent 2 0 R >>",
        "<< /Type /Page /Parent 2 0 R >>",
        "<< /Type /Page /Parent 2 0 R >>",
    ];
    objects.extend_from_slice(extra);
    open(&classic_with_root(&objects, ""))
}

fn apply(doc: &mut Document, op: SetInitialView) -> Result<(), Error> {
    doc.apply_all(Origin::Local, [Op::from(op)]).map(|_| ())
}

fn opens_at(doc: &Document) -> Option<(usize, View)> {
    match doc.initial_view().unwrap().open {
        OpenAction::Destination(d) => Some((d.page_index.unwrap(), d.view)),
        OpenAction::None => None,
        other => panic!("not a destination: {other:?}"),
    }
}

#[test]
fn the_sample_deserialises_and_applies() {
    let op: Op = serde_json::from_slice(&fixture("ops/v1/set-initial-view.json")).unwrap();
    let mut doc = corpus("hello.pdf");
    doc.apply_all(Origin::Local, [op]).unwrap();
    let view = doc.initial_view().unwrap();
    assert_eq!(opens_at(&doc), Some((0, View::FitH { top: None })));
    assert_eq!(
        (view.page_mode, view.page_layout),
        (Some(PageMode::Outlines), Some(PageLayout::OneColumn))
    );
    assert_eq!(
        view.preferences,
        WindowPreferences {
            display_doc_title: true,
            fit_window: true,
            ..WindowPreferences::default()
        }
    );
}

#[test]
fn the_sample_round_trips_through_json() {
    let op: Op = serde_json::from_slice(&fixture("ops/v1/set-initial-view.json")).unwrap();
    let again: Op = serde_json::from_str(&serde_json::to_string(&op).unwrap()).unwrap();
    assert_eq!(again, op);
}

#[test]
fn it_is_a_document_change_that_saves_incrementally() {
    let op = Op::from(SetInitialView::default().with_page_mode(Some(PageMode::Outlines)));
    assert_eq!(
        (op.class(), op.requires_rewrite()),
        (ModificationClass::Document, false)
    );
}

#[test]
fn each_zoom_writes_its_destination() {
    let cases = [
        (
            OpenZoom::ReaderDefault,
            View::Xyz {
                left: None,
                top: None,
                zoom: None,
            },
        ),
        (OpenZoom::FitPage, View::Fit),
        (OpenZoom::FitWidth, View::FitH { top: None }),
        (
            OpenZoom::Percent(125),
            View::Xyz {
                left: None,
                top: None,
                zoom: Some(1.25),
            },
        ),
    ];
    for (zoom, view) in cases {
        let mut d = doc("", &[]);
        apply(
            &mut d,
            SetInitialView::default().with_open(Some(OpenAt::new("p4.0".parse().unwrap(), zoom))),
        )
        .unwrap();
        assert_eq!(opens_at(&d), Some((1, view)), "{zoom:?}");
    }
}

#[test]
fn a_field_left_out_keeps_what_the_file_has_and_null_removes_it() {
    let mut d = doc(
        "/OpenAction [5 0 R /XYZ 72 720 1.5] /PageMode /UseThumbs /PageLayout /TwoColumnLeft",
        &[],
    );
    apply(&mut d, SetInitialView::default().with_page_layout(None)).unwrap();
    let view = d.initial_view().unwrap();
    assert_eq!((view.page_mode, view.page_layout), (Some(PageMode::Thumbnails), None));
    // The open destination, which the op's zooms can't express, is kept to the byte.
    assert_eq!(
        opens_at(&d),
        Some((
            2,
            View::Xyz {
                left: Some(72.0),
                top: Some(720.0),
                zoom: Some(1.5)
            }
        ))
    );
    apply(&mut d, SetInitialView::default().with_open(None).with_page_mode(None)).unwrap();
    let view = d.initial_view().unwrap();
    assert_eq!((view.open, view.page_mode), (OpenAction::None, None));
}

#[test]
fn preferences_set_true_remove_false_and_keep_other_entries() {
    let mut d = doc("/ViewerPreferences << /HideToolbar true /Direction /R2L >>", &[]);
    let patch = WindowPreferencesPatch::default()
        .with_hide_reader_controls(false)
        .with_display_doc_title(true);
    apply(&mut d, SetInitialView::default().with_preferences(patch)).unwrap();
    assert_eq!(
        d.initial_view().unwrap().preferences,
        WindowPreferences {
            display_doc_title: true,
            ..WindowPreferences::default()
        }
    );
    let catalog = d.catalog().unwrap();
    let prefs = catalog.get("ViewerPreferences").unwrap().as_dict().unwrap();
    assert_eq!(
        prefs.iter().map(|(k, _)| k.as_bytes()).collect::<Vec<_>>(),
        [b"Direction".as_slice(), b"DisplayDocTitle"]
    );
}

#[test]
fn an_indirect_preferences_dictionary_is_edited_in_place() {
    let mut d = doc("/ViewerPreferences 6 0 R", &["<< /FitWindow true >>"]);
    apply(
        &mut d,
        SetInitialView::default().with_preferences(WindowPreferencesPatch::default().with_center_window(true)),
    )
    .unwrap();
    let prefs = d.initial_view().unwrap().preferences;
    assert_eq!((prefs.fit_window, prefs.center_window), (true, true));
    assert!(
        d.catalog()
            .unwrap()
            .get("ViewerPreferences")
            .unwrap()
            .as_ref()
            .is_some()
    );
}

#[test]
fn an_open_action_that_is_not_a_destination_is_never_replaced() {
    let mut d = doc("/OpenAction << /S /JavaScript /JS (app.alert(1)) >>", &[]);
    let before = snapshot(&d);
    for op in [
        SetInitialView::default().with_open(Some(OpenAt::new("p3.0".parse().unwrap(), OpenZoom::FitPage))),
        SetInitialView::default()
            .with_open(None)
            .with_page_mode(Some(PageMode::Outlines)),
    ] {
        assert_eq!(
            apply(&mut d, op).map_err(|e| e.code()),
            Err("open-action-not-destination")
        );
        assert_eq!(snapshot(&d), before);
    }
    // Leaving `open` out changes the rest and keeps the action.
    apply(
        &mut d,
        SetInitialView::default().with_page_mode(Some(PageMode::Outlines)),
    )
    .unwrap();
    let view = d.initial_view().unwrap();
    assert_eq!(view.open, OpenAction::Action(ActionKind::JavaScript));
    assert_eq!(view.page_mode, Some(PageMode::Outlines));
}

#[test]
fn a_goto_that_runs_more_actions_or_an_unreadable_action_is_never_replaced() {
    for action in [
        "/OpenAction << /S /GoTo /D [3 0 R /Fit] /Next << /S /JavaScript /JS (app.alert(1)) >> >>",
        "/OpenAction << /JS (app.alert(1)) /S (JavaScript) >>",
    ] {
        let mut d = doc(action, &[]);
        let before = snapshot(&d);
        let op = SetInitialView::default().with_open(Some(OpenAt::new("p3.0".parse().unwrap(), OpenZoom::FitPage)));
        assert_eq!(
            apply(&mut d, op).map_err(|e| e.code()),
            Err("open-action-not-destination"),
            "{action}"
        );
        assert_eq!(snapshot(&d), before);
        assert!(
            matches!(d.initial_view().unwrap().open, OpenAction::Action(_)),
            "{action}"
        );
    }
}

#[test]
fn a_goto_open_action_is_a_destination_and_can_be_replaced() {
    let mut d = doc("/OpenAction << /S /GoTo /D [3 0 R /Fit] >>", &[]);
    apply(
        &mut d,
        SetInitialView::default().with_open(Some(OpenAt::new("p5.0".parse().unwrap(), OpenZoom::FitPage))),
    )
    .unwrap();
    assert_eq!(opens_at(&d), Some((2, View::Fit)));
}

#[test]
fn an_unknown_page_is_refused() {
    let mut d = doc("", &[]);
    let op = SetInitialView::default().with_open(Some(OpenAt::new("p9.0".parse().unwrap(), OpenZoom::FitPage)));
    assert_eq!(apply(&mut d, op).map_err(|e| e.code()), Err("unknown-page"));
}

#[test]
fn a_value_newer_than_the_file_raises_its_version() {
    // `classic_with_root` writes a 1.7 header; 1.3 has the same length, so the offsets hold.
    let bytes = classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
        ],
        "",
    );
    let old = || open(&[b"%PDF-1.3".as_slice(), &bytes[8..]].concat());
    let mut d = old();
    assert_eq!(d.version().unwrap().as_deref(), Some("1.3"));
    apply(
        &mut d,
        SetInitialView::default().with_page_layout(Some(PageLayout::TwoPageLeft)),
    )
    .unwrap();
    assert_eq!(d.version().unwrap().as_deref(), Some("1.5"));
    let mut d = old();
    apply(
        &mut d,
        SetInitialView::default().with_page_layout(Some(PageLayout::OneColumn)),
    )
    .unwrap();
    assert_eq!(d.version().unwrap().as_deref(), Some("1.3"));
    // A 1.7 file already allows every value.
    let mut d = doc("", &[]);
    apply(
        &mut d,
        SetInitialView::default().with_page_mode(Some(PageMode::Attachments)),
    )
    .unwrap();
    assert_eq!(d.catalog().unwrap().get("Version"), None);
}

#[test]
fn undo_leaves_the_overlay_empty_and_redo_writes_the_same_changes() {
    let mut d = doc("/ViewerPreferences << /HideToolbar true >>", &[]);
    let base = snapshot(&d);
    apply(
        &mut d,
        SetInitialView::default()
            .with_open(Some(OpenAt::new("p4.0".parse().unwrap(), OpenZoom::Percent(200))))
            .with_page_mode(Some(PageMode::Thumbnails))
            .with_page_layout(Some(PageLayout::TwoPageRight))
            .with_preferences(WindowPreferencesPatch::default().with_hide_reader_controls(false)),
    )
    .unwrap();
    let applied = snapshot(&d);
    d.undo().unwrap();
    assert_eq!(snapshot(&d), base);
    assert!(d.overlay().is_empty());
    d.redo().unwrap();
    assert_eq!(snapshot(&d), applied);
}

#[test]
fn setting_what_the_file_already_has_writes_nothing() {
    let mut d = doc(
        "/PageMode /UseOutlines /ViewerPreferences << /DisplayDocTitle true >>",
        &[],
    );
    apply(
        &mut d,
        SetInitialView::default()
            .with_page_mode(Some(PageMode::Outlines))
            .with_preferences(WindowPreferencesPatch::default().with_display_doc_title(true)),
    )
    .unwrap();
    assert!(d.overlay().is_empty());
}

mod rejects {
    use super::*;

    fn parse(args: &str) -> serde_json::Result<Op> {
        serde_json::from_str(&format!(r#"{{ "op": "setInitialView", "args": {args} }}"#))
    }

    #[test]
    fn an_unknown_field_mode_layout_or_preference() {
        for args in [
            r#"{ "colour": "red" }"#,
            r#"{ "pageMode": "useOutlines" }"#,
            r#"{ "pageLayout": "spread" }"#,
            r#"{ "preferences": { "hideEverything": true } }"#,
            r#"{ "open": { "page": "p3.0", "zoom": "fitBox" } }"#,
            r#"{ "open": { "page": "p3.0", "zoom": 0 } }"#,
            r#"{ "open": { "page": "p3.0", "zoom": 6401 } }"#,
            r#"{ "open": { "zoom": "fitPage" } }"#,
            r#"{ "open": { "page": 3 } }"#,
            r#"{ "open": "p3.0" }"#,
            r#"{ "preferences": { "fitWindow": "yes" } }"#,
            r#"{ "pageMode": 1 }"#,
            "[]",
        ] {
            assert!(parse(args).is_err(), "{args}");
        }
    }

    #[test]
    fn an_unknown_field_by_name() {
        let error = parse(r#"{ "colour": "red" }"#).unwrap_err().to_string();
        assert!(error.contains("unknown field `colour`"), "{error}");
    }

    #[test]
    fn the_same_op_whichever_key_comes_first() {
        let args = r#"{ "open": { "page": "p3.0", "zoom": 125 }, "preferences": { "centerWindow": true } }"#;
        let before: Op = serde_json::from_str(&format!(r#"{{ "args": {args}, "op": "setInitialView" }}"#)).unwrap();
        assert_eq!(before, parse(args).unwrap());
    }

    #[test]
    fn nothing_when_every_field_is_left_out() {
        assert_eq!(parse("{}").unwrap(), Op::from(SetInitialView::default()));
    }
}
