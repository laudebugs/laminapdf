//! A contents page made from the bookmarks (E3l): its entries, the labels they print, a link per
//! entry to its bookmark's page, one undo step, the bookmark it can add, the refusals, and the
//! page saved and read back.
#![expect(clippy::unwrap_used, reason = "test code: a panic is a test failure")]

use laminapdf::extract::TextOptions;
use laminapdf::{ContentsOptions, Document, Font, LinkTo, OpenOptions, PagePosition, SaveOptions, UndoOutcome};
use laminapdf_test_fixtures::fixture;

/// `fixtures/golden/links-and-outlines.pdf`: four unlabelled pages; bookmarks Title page
/// (page 1), Contents (page 2, named `contents`), Chapters (no page) holding Chapter 1
/// (page 3) and Chapter 2 (page 4, named `chapter-2`) holding Appendix (page 4), and Website
/// (a web address).
fn outline() -> Document {
    Document::open_bytes(fixture("golden/links-and-outlines.pdf"), OpenOptions::default()).unwrap()
}

fn noto() -> Font {
    Font::from_bytes(fixture("fonts/NotoSans-Regular.ttf")).unwrap()
}

fn options() -> ContentsOptions {
    ContentsOptions::new().with_font(noto())
}

/// The page text of page `index`, lines joined by newlines.
fn text(doc: &Document, index: u32) -> String {
    doc.page_text(index, &TextOptions::new()).unwrap().lines().join("\n")
}

/// Each link of page `index` as the 0-based index of the page it goes to.
fn link_pages(doc: &Document, index: u32) -> Vec<u32> {
    doc.page_links(index)
        .unwrap()
        .into_iter()
        .filter_map(|l| match l.to {
            LinkTo::Destination { target, .. } => target.map(|t| t.page),
            _ => None,
        })
        .collect()
}

#[test]
fn create_contents_page_should_insert_one_page_before_the_first() {
    let mut doc = outline();
    doc.create_contents_page(&options()).unwrap();
    assert_eq!(doc.page_count().unwrap(), 5);
}

#[test]
fn create_contents_page_should_list_the_bookmarks_that_go_to_pages_with_their_labels() {
    let mut doc = outline();
    doc.create_contents_page(&options()).unwrap();
    // The heading, then each bookmark with a page, two levels deep, and the label its page
    // shows once the contents page is in (the file labels its pages i, ii, 1, 2).
    assert_eq!(
        text(&doc, 0),
        "Contents\nTitle page i\nContents ii\nChapter 1 1\nChapter 2 2"
    );
}

#[test]
fn create_contents_page_should_print_the_labels_the_linked_pages_show() {
    let mut doc = outline();
    doc.create_contents_page(&options()).unwrap();
    let shown: Vec<String> = link_pages(&doc, 0)
        .into_iter()
        .map(|page| doc.page_info(page).unwrap().label.unwrap_or_default())
        .collect();
    let printed: Vec<String> = text(&doc, 0)
        .lines()
        .skip(1)
        .map(|line| line.rsplit(' ').next().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(printed, shown);
}

#[test]
fn create_contents_page_should_link_each_entry_to_its_bookmarks_page() {
    let mut doc = outline();
    doc.create_contents_page(&options()).unwrap();
    // Title page, Contents, Chapter 1 and Chapter 2, on pages 1 to 4 once the contents page
    // is page 0; Chapters (no page) and Website (a web address) are left out.
    assert_eq!(link_pages(&doc, 0), vec![1, 2, 3, 4]);
}

#[test]
fn create_contents_page_should_list_one_level_when_asked() {
    let mut doc = outline();
    doc.create_contents_page(&options().with_levels(1)).unwrap();
    assert_eq!(link_pages(&doc, 0), vec![1, 2]);
}

#[test]
fn create_contents_page_should_go_where_it_is_asked() {
    let mut doc = outline();
    doc.create_contents_page(&options().with_at(PagePosition::End)).unwrap();
    assert_eq!(link_pages(&doc, 4), vec![0, 1, 2, 3]);
}

#[test]
fn create_contents_page_should_refuse_a_title_with_a_character_no_font_has() {
    // `fixtures/corpus/outline.pdf`'s "Appendix 𝄞 Ω": Noto Sans has no 𝄞, and the page never
    // draws a box in its place.
    let mut doc = Document::open_bytes(fixture("corpus/outline.pdf"), OpenOptions::default()).unwrap();
    let err = doc.create_contents_page(&options()).unwrap_err();
    assert_eq!(err.code(), "missing-glyph");
}

#[test]
fn undo_should_remove_the_contents_page_and_its_links_in_one_step() {
    let mut doc = outline();
    doc.create_contents_page(&options().with_bookmark(true)).unwrap();
    let reverted = doc.undo().unwrap().unwrap();
    assert_eq!(
        (
            reverted.outcome,
            doc.page_count().unwrap(),
            doc.outline().unwrap().len()
        ),
        (UndoOutcome::Applied, 4, 4)
    );
}

#[test]
fn create_contents_page_should_add_its_bookmark_first_when_asked() {
    let mut doc = outline();
    doc.create_contents_page(&options().with_bookmark(true)).unwrap();
    let first = doc.outline().unwrap().into_iter().next().unwrap();
    assert_eq!(
        (first.title.as_str(), first.target.map(|t| t.page)),
        ("Contents", Some(0))
    );
}

#[test]
fn create_contents_page_should_refuse_a_document_without_bookmarks() {
    let mut doc = Document::open_bytes(fixture("golden/examples/hello.pdf"), OpenOptions::default()).unwrap();
    let err = doc.create_contents_page(&ContentsOptions::new()).unwrap_err();
    assert_eq!(err.code(), "no-bookmarks");
}

#[test]
fn create_contents_page_should_refuse_the_standard_font_on_a_pdf_ua_file() {
    let mut doc = Document::open_bytes(fixture("golden/examples/pdf_ua_report.pdf"), OpenOptions::default()).unwrap();
    let err = doc.create_contents_page(&ContentsOptions::new()).unwrap_err();
    assert_eq!(err.code(), "font-not-embedded");
}

#[test]
fn a_saved_contents_page_should_keep_its_links_when_read_back() {
    let mut doc = outline();
    doc.create_contents_page(&options()).unwrap();
    let saved = doc.save(&SaveOptions::default()).unwrap();
    let back = Document::open_bytes(saved.bytes, OpenOptions::default()).unwrap();
    assert_eq!(link_pages(&back, 0), vec![1, 2, 3, 4]);
}
