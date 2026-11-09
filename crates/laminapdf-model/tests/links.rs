//! `Pages::links` on malformed link annotations (ADR 0003): every value the
//! reader can't use leaves the link without an action, or leaves the link out, and never fails.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_model::{ActionKind, Document, LinkAction, OpenOptions};
use laminapdf_test_fixtures::classic_with_root;

/// The actions of a one-page file's links, each annotation given after `/Subtype /Link`.
fn actions(annots: &[&str]) -> Vec<LinkAction> {
    let list = (0..annots.len())
        .map(|i| format!("{} 0 R", i + 4))
        .collect::<Vec<_>>()
        .join(" ");
    let page = format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [{list}] >>");
    let bodies: Vec<String> = annots
        .iter()
        .map(|a| format!("<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] {a} >>"))
        .collect();
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page.as_str(),
    ];
    objects.extend(bodies.iter().map(String::as_str));
    let doc = Document::open(classic_with_root(&objects, ""), OpenOptions::default()).unwrap();
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    pages.links(&page).unwrap().into_iter().map(|l| l.action).collect()
}

#[test]
fn an_action_that_cant_be_read_leaves_the_link_without_one() {
    assert_eq!(
        actions(&[
            "/A (not a dictionary)",
            "/A << /URI (https://example.com/) >>",
            "/A << /S (URI) /URI (https://example.com/) >>",
            "/A << /S /URI /URI 42 >>",
            "/A << /S /GoTo >>",
            "/A << /S /Named /N (NextPage) >>",
        ]),
        vec![LinkAction::None; 6]
    );
}

#[test]
fn a_destination_that_cant_be_read_falls_back_to_the_action() {
    let found = actions(&["/Dest 42 /A << /S /URI /URI (https://example.com/) >>"]);
    assert_eq!(found, [LinkAction::Uri(b"https://example.com/".to_vec())]);
}

#[test]
fn a_uri_keeps_its_bytes_whatever_their_encoding() {
    let found = actions(&[
        "/A << /S /URI /URI <68747470733a2f2fe4> >>",
        "/A << /S /URI /URI /mailto:x@y.z >>",
    ]);
    assert_eq!(
        found,
        [
            LinkAction::Uri(b"https://\xe4".to_vec()),
            LinkAction::Uri(b"mailto:x@y.z".to_vec())
        ]
    );
}

#[test]
fn another_actions_type_is_reported_never_run() {
    let found = actions(&["/A << /S /Launch /F (calc.exe) >>", "/A << /S /Rendition >>"]);
    assert_eq!(
        found,
        [
            LinkAction::Other(ActionKind::Launch),
            LinkAction::Other(ActionKind::Other(b"Rendition".to_vec()))
        ]
    );
}

#[test]
fn a_link_without_a_rectangle_or_that_hides_is_left_out() {
    // A /Rect that isn't four numbers, NoView (/F 32), and an annotation that isn't a link.
    let doc = Document::open(
        classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R 5 0 R 6 0 R] >>",
                "<< /Type /Annot /Subtype /Link /Rect [0 0 x 10] /Dest [3 0 R /Fit] >>",
                "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /F 32 /Dest [3 0 R /Fit] >>",
                "<< /Type /Annot /Subtype /Text /Rect [0 0 10 10] >>",
            ],
            "",
        ),
        OpenOptions::default(),
    )
    .unwrap();
    let pages = doc.pages().unwrap();
    assert_eq!(pages.links(&pages.get(0).unwrap()).unwrap(), []);
}

#[test]
fn another_files_name_is_read_from_a_string_or_a_name() {
    let found = actions(&[
        "/A << /S /GoToR /F << /Type /Filespec /UF (budget.pdf) >> >>",
        "/A << /S /GoToR /F << /Type /Filespec /F /budget.pdf >> >>",
    ]);
    assert_eq!(
        found,
        [
            LinkAction::OtherFile(Some("budget.pdf".to_owned())),
            LinkAction::OtherFile(Some("budget.pdf".to_owned()))
        ]
    );
}
