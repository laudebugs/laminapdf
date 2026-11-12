//! `Document::initial_view` and `Document::threads` in page space (ADR 0027), and
//! `SetInitialView` through `apply`, `undo` and `save`.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use laminapdf::{
    Document, Op, OpenAt, OpenOptions, OpenTarget, OpenZoom, PageLayout, PageMode, PageTargetView, SaveOptions,
    SetInitialView, WindowPreferencesPatch,
};
use laminapdf_test_fixtures::classic_with_root;

/// Two 600 × 800 pages (3, 4); page 4 turned a quarter. The catalog holds `catalog_extra`.
fn doc(catalog_extra: &str, extra: &[&str]) -> Document {
    let catalog = format!("<< /Type /Catalog /Pages 2 0 R {catalog_extra} >>");
    let mut objects = vec![
        catalog.as_str(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 600 800] >>",
        "<< /Type /Page /Parent 2 0 R >>",
        "<< /Type /Page /Parent 2 0 R /Rotate 90 >>",
    ];
    objects.extend_from_slice(extra);
    Document::open_bytes(classic_with_root(&objects, ""), OpenOptions::new()).unwrap()
}

/// The page and view a document opens at, when it opens at a page.
fn opens_at(open: &OpenTarget) -> Option<(u32, PageTargetView)> {
    match open {
        OpenTarget::Page(target) => Some((target.page, target.view)),
        _ => None,
    }
}

#[test]
fn the_open_destination_is_a_page_target_in_page_space() {
    let view = doc("/OpenAction [3 0 R /XYZ 72 700 2]", &[]).initial_view().unwrap();
    assert_eq!(
        opens_at(&view.open),
        Some((
            0,
            PageTargetView::At {
                x: Some(72.0),
                y: Some(100.0),
                zoom: Some(2.0)
            }
        ))
    );
}

#[test]
fn an_action_is_named_by_its_type() {
    let view = doc("/OpenAction << /S /Named /N /LastPage >>", &[])
        .initial_view()
        .unwrap();
    assert_eq!(view.open, OpenTarget::Action("Named".to_owned()));
}

#[test]
fn a_destination_to_a_missing_page_opens_at_the_reader_default() {
    let view = doc("/OpenAction [9 0 R /Fit]", &[]).initial_view().unwrap();
    assert_eq!(view.open, OpenTarget::ReaderDefault);
}

/// capability: I.09 core read
#[test]
fn thread_parts_are_rectangles_in_page_space() {
    let d = doc(
        "/Threads [5 0 R]",
        &[
            "<< /Type /Thread /F 6 0 R /I << /Title (Backlog) >> >>",
            "<< /Type /Bead /T 5 0 R /N 7 0 R /V 7 0 R /P 3 0 R /R [50 600 300 700] >>",
            "<< /Type /Bead /N 6 0 R /V 6 0 R /P 4 0 R /R [0 0 600 100] >>",
        ],
    );
    let threads = d.threads().unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].title.as_deref(), Some("Backlog"));
    let parts = &threads[0].parts;
    assert_eq!(parts.iter().map(|p| p.page).collect::<Vec<_>>(), [Some(0), Some(1)]);
    // Top-left origin, y down: 700 in user space is 100 from the top of an 800-point page.
    let first = parts[0].rect.unwrap();
    assert_eq!(
        (first.x0.get(), first.y0.get(), first.x1.get(), first.y1.get()),
        (50.0, 100.0, 300.0, 200.0)
    );
    // The turned page is 800 wide and 600 tall as displayed.
    let second = parts[1].rect.unwrap();
    assert_eq!((second.width().get(), second.height().get()), (100.0, 600.0));
}

#[test]
fn an_edit_applies_undoes_and_saves() {
    let mut d = doc("", &[]);
    let page = d.page_info(1).unwrap().id;
    let op = SetInitialView::default()
        .with_open(Some(OpenAt::new(page, OpenZoom::FitPage)))
        .with_page_mode(Some(PageMode::Thumbnails))
        .with_page_layout(Some(PageLayout::TwoPageRight))
        .with_preferences(WindowPreferencesPatch::default().with_fit_window(true));
    let edited = d.apply(vec![Op::from(op)]).unwrap();
    assert!(edited.changed);
    assert!(edited.touched_pages.is_empty());
    let view = d.initial_view().unwrap();
    assert_eq!(opens_at(&view.open), Some((1, PageTargetView::Fit)));
    assert_eq!(
        (view.page_mode, view.page_layout, view.preferences.fit_window),
        (Some(PageMode::Thumbnails), Some(PageLayout::TwoPageRight), true)
    );
    let saved = d.save(&SaveOptions::new()).unwrap().bytes;
    let reopened = Document::open_bytes(saved, OpenOptions::new()).unwrap();
    assert_eq!(reopened.initial_view().unwrap(), view);
    d.undo().unwrap();
    assert_eq!(d.initial_view().unwrap().open, OpenTarget::ReaderDefault);
}
