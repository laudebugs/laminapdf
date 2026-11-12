//! `Document::initial_view` and `Document::threads`: the catalog's open action, page mode,
//! page layout and window preferences, and article threads with their beads (ISO 32000-2
//! §12.2, §7.7.2, §12.4.3).
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use laminapdf_model::{ActionKind, Document, OpenAction, OpenOptions, PageLayout, PageMode, View, WindowPreferences};
use laminapdf_test_fixtures::classic_with_root;

/// A three-page document whose catalog holds `catalog_extra`; `extra` objects follow as 6, 7, ….
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
    Document::open(classic_with_root(&objects, ""), OpenOptions::default()).unwrap()
}

#[test]
fn a_catalog_without_entries_opens_with_the_defaults() {
    let view = doc("", &[]).initial_view().unwrap();
    assert_eq!(view.open, OpenAction::None);
    assert_eq!((view.page_mode, view.page_layout), (None, None));
    assert_eq!(view.preferences, WindowPreferences::default());
}

#[test]
fn an_explicit_destination_opens_at_its_page_and_view() {
    let view = doc("/OpenAction [4 0 R /FitH 700]", &[]).initial_view().unwrap();
    let OpenAction::Destination(dest) = view.open else {
        panic!("no destination: {:?}", view.open);
    };
    assert_eq!(dest.page_index, Some(1));
    assert_eq!(dest.view, View::FitH { top: Some(700.0) });
}

#[test]
fn a_goto_action_reads_as_its_destination() {
    let view = doc("/OpenAction << /S /GoTo /D [5 0 R /XYZ null null 1.5] >>", &[])
        .initial_view()
        .unwrap();
    let OpenAction::Destination(dest) = view.open else {
        panic!("no destination: {:?}", view.open);
    };
    assert_eq!(dest.page_index, Some(2));
    assert_eq!(
        dest.view,
        View::Xyz {
            left: None,
            top: None,
            zoom: Some(1.5)
        }
    );
}

#[test]
fn another_action_is_reported_by_its_type_and_never_resolved() {
    let view = doc("/OpenAction << /S /JavaScript /JS (app.alert(1)) >>", &[])
        .initial_view()
        .unwrap();
    assert_eq!(view.open, OpenAction::Action(ActionKind::JavaScript));
}

#[test]
fn a_named_destination_resolves_through_the_dests_dictionary() {
    let view = doc(
        "/OpenAction << /S /GoTo /D /intro >> /Dests << /intro [3 0 R /Fit] >>",
        &[],
    )
    .initial_view()
    .unwrap();
    let OpenAction::Destination(dest) = view.open else {
        panic!("no destination: {:?}", view.open);
    };
    assert_eq!((dest.page_index, dest.view), (Some(0), View::Fit));
    assert_eq!(dest.name.as_deref(), Some(b"intro".as_slice()));
}

#[test]
fn page_mode_layout_and_window_preferences_are_read() {
    let view = doc(
        "/PageMode /UseThumbs /PageLayout /TwoPageRight /ViewerPreferences 6 0 R",
        &["<< /DisplayDocTitle true /FitWindow true /HideToolbar false /Direction /R2L >>"],
    )
    .initial_view()
    .unwrap();
    assert_eq!(view.page_mode, Some(PageMode::Thumbnails));
    assert_eq!(view.page_layout, Some(PageLayout::TwoPageRight));
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
fn an_unknown_mode_or_a_value_of_the_wrong_type_counts_as_absent() {
    let d = doc("/PageMode /UseMagic /PageLayout 3 /ViewerPreferences (no)", &[]);
    let view = d.initial_view().unwrap();
    assert_eq!((view.page_mode, view.page_layout), (None, None));
    assert_eq!(view.preferences, WindowPreferences::default());
    let codes: Vec<&str> = d.warnings().iter().map(laminapdf_model::Warning::code).collect();
    assert_eq!(codes, ["invalid-entry", "invalid-entry"]);
}

#[test]
fn every_page_mode_and_layout_round_trips_through_its_pdf_name() {
    for mode in PageMode::ALL {
        assert_eq!(PageMode::from_pdf_name(mode.pdf_name().as_bytes()), Some(mode));
    }
    for layout in PageLayout::ALL {
        assert_eq!(PageLayout::from_pdf_name(layout.pdf_name().as_bytes()), Some(layout));
    }
    assert_eq!(PageMode::from_pdf_name(b"UseMagic"), None);
}

/// Three beads in a circle over pages 1, 2 and 3: 6 is the thread, 7–9 its beads.
const THREAD: [&str; 4] = [
    "<< /Type /Thread /F 7 0 R /I << /Title (Repairs backlog) >> >>",
    "<< /Type /Bead /T 6 0 R /N 8 0 R /V 9 0 R /P 3 0 R /R [50 400 300 700] >>",
    "<< /Type /Bead /N 9 0 R /V 7 0 R /P 4 0 R /R [50 100 300 700] >>",
    "<< /Type /Bead /N 7 0 R /V 8 0 R /P 5 0 R /R [300 100 550 400] >>",
];

/// capability: I.09 core read
#[test]
fn a_thread_lists_its_beads_in_order_and_stops_where_the_circle_closes() {
    let threads = doc("/Threads [6 0 R]", &THREAD).threads().unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].title.as_deref(), Some("Repairs backlog"));
    let pages: Vec<Option<usize>> = threads[0].beads.iter().map(|b| b.page_index).collect();
    assert_eq!(pages, [Some(0), Some(1), Some(2)]);
    let first = threads[0].beads[0].rect.unwrap();
    assert_eq!((first.x0, first.y0, first.x1, first.y1), (50.0, 400.0, 300.0, 700.0));
}

#[test]
fn a_bead_on_a_page_the_document_lacks_has_no_page() {
    let threads = doc(
        "/Threads [6 0 R]",
        &[
            "<< /Type /Thread /F 7 0 R >>",
            "<< /Type /Bead /T 6 0 R /N 7 0 R /V 7 0 R /P 99 0 R /R [0 0 10 10] >>",
        ],
    )
    .threads()
    .unwrap();
    assert_eq!(threads[0].title, None);
    assert_eq!(threads[0].beads.len(), 1);
    assert_eq!(
        (threads[0].beads[0].page.as_ref(), threads[0].beads[0].page_index),
        (None, None)
    );
}

#[test]
fn a_document_without_threads_has_none() {
    assert_eq!(doc("", &[]).threads().unwrap(), []);
}
