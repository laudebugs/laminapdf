//! A contents page made from the bookmarks (E3l): its entries, the labels they print, a link per
//! entry to its bookmark's page, one undo step, the bookmark it can add, the refusals, and the
//! page saved and read back.
#![expect(clippy::unwrap_used, reason = "test code: a panic is a test failure")]

use laminapdf::extract::TextOptions;
use laminapdf::{
    ContentsFonts, ContentsListing, ContentsOptions, Document, Font, LinkTo, OpenOptions, PagePosition, SaveOptions,
    UndoOutcome,
};
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
    ContentsOptions::new()
}

fn fonts() -> ContentsFonts {
    ContentsFonts::new().with_font(noto())
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
    doc.create_contents_page(&options(), &fonts()).unwrap();
    assert_eq!(doc.page_count().unwrap(), 5);
}

#[test]
fn create_contents_page_should_list_the_bookmarks_that_go_to_pages_with_their_labels() {
    let mut doc = outline();
    doc.create_contents_page(&options(), &fonts()).unwrap();
    // The heading, then each bookmark with a page, two levels deep, and the label its page
    // shows once the contents page is in: the file labels its pages i, ii, 1, 2, and the
    // contents page joins the first range as "i", so the title page becomes "ii".
    assert_eq!(
        text(&doc, 0),
        "Contents\nTitle page ii\nContents iii\nChapter 1 1\nChapter 2 2"
    );
}

#[test]
fn create_contents_page_should_print_the_labels_the_linked_pages_show() {
    let mut doc = outline();
    doc.create_contents_page(&options(), &fonts()).unwrap();
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
    doc.create_contents_page(&options(), &fonts()).unwrap();
    // Title page, Contents, Chapter 1 and Chapter 2, on pages 1 to 4 once the contents page
    // is page 0; Chapters (no page) and Website (a web address) are left out.
    assert_eq!(link_pages(&doc, 0), vec![1, 2, 3, 4]);
}

#[test]
fn create_contents_page_should_list_one_level_when_asked() {
    let mut doc = outline();
    doc.create_contents_page(&options().with_levels(1), &fonts()).unwrap();
    assert_eq!(link_pages(&doc, 0), vec![1, 2]);
}

#[test]
fn create_contents_page_should_go_where_it_is_asked() {
    let mut doc = outline();
    doc.create_contents_page(&options().with_at(PagePosition::End), &fonts())
        .unwrap();
    assert_eq!(link_pages(&doc, 4), vec![0, 1, 2, 3]);
}

#[test]
fn create_contents_page_should_refuse_a_title_with_a_character_no_font_has() {
    // `fixtures/corpus/outline.pdf`'s "Appendix 𝄞 Ω": Noto Sans has no 𝄞, and the page never
    // draws a box in its place.
    let mut doc = Document::open_bytes(fixture("corpus/outline.pdf"), OpenOptions::default()).unwrap();
    let err = doc.create_contents_page(&options(), &fonts()).unwrap_err();
    assert_eq!(err.code(), "missing-glyph");
}

#[test]
fn undo_should_remove_the_contents_page_and_its_links_in_one_step() {
    let mut doc = outline();
    doc.create_contents_page(&options().with_bookmark(true), &fonts())
        .unwrap();
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
    doc.create_contents_page(&options().with_bookmark(true), &fonts())
        .unwrap();
    let first = doc.outline().unwrap().into_iter().next().unwrap();
    assert_eq!(
        (first.title.as_str(), first.target.map(|t| t.page)),
        ("Contents", Some(0))
    );
}

#[test]
fn create_contents_page_should_refuse_a_document_without_bookmarks() {
    let mut doc = Document::open_bytes(fixture("golden/examples/hello.pdf"), OpenOptions::default()).unwrap();
    let err = doc
        .create_contents_page(&ContentsOptions::new(), &ContentsFonts::new())
        .unwrap_err();
    assert_eq!(err.code(), "no-bookmarks");
}

#[test]
fn create_contents_page_should_refuse_the_standard_font_on_a_pdf_ua_file() {
    let mut doc = Document::open_bytes(fixture("golden/flow-table.pdf"), OpenOptions::default()).unwrap();
    let err = doc
        .create_contents_page(&ContentsOptions::new(), &ContentsFonts::new())
        .unwrap_err();
    assert_eq!(err.code(), "font-not-embedded");
}

#[test]
fn a_saved_contents_page_should_keep_its_links_when_read_back() {
    let mut doc = outline();
    doc.create_contents_page(&options(), &fonts()).unwrap();
    let saved = doc.save(&SaveOptions::default()).unwrap();
    let back = Document::open_bytes(saved.bytes, OpenOptions::default()).unwrap();
    assert_eq!(link_pages(&back, 0), vec![1, 2, 3, 4]);
}

#[test]
fn creating_a_contents_page_twice_on_fresh_copies_should_save_the_same_bytes() {
    // What a replay of the op log needs: the same source bytes and ops from the same file.
    let save = || {
        let mut doc = outline();
        doc.create_contents_page(&options(), &fonts()).unwrap();
        doc.save(&SaveOptions::default()).unwrap().bytes
    };
    assert_eq!(save(), save());
}

/// The kinds of the structure tree's elements, depth first.
fn kinds(nodes: &[laminapdf::StructureNode], out: &mut Vec<String>) {
    for node in nodes {
        out.push(node.kind.clone());
        let kids: Vec<laminapdf::StructureNode> = node
            .kids
            .iter()
            .filter_map(|k| match k {
                laminapdf::StructureKid::Node(n) => Some(n.clone()),
                _ => None,
            })
            .collect();
        kinds(&kids, out);
    }
}

/// A tagged two-page document whose two headings are its bookmarks.
fn tagged() -> Document {
    use laminapdf::flow::{Flow, FlowBlock, PageTemplate, Paragraph, TextStyle};
    use laminapdf::{BuildOptions, Builder, PageSize};
    let mut b = Builder::new(BuildOptions::new().with_tagged(true));
    let font = b.font(noto()).unwrap();
    let body = TextStyle::new(&font, 11.0);
    let flow = Flow::new(PageTemplate::new(PageSize::A4))
        .block(FlowBlock::Heading(1, Paragraph::plain("Chapter A", &body)))
        .block(FlowBlock::PageBreak)
        .block(FlowBlock::Heading(1, Paragraph::plain("Chapter B", &body)));
    b.flow(&flow).unwrap();
    let bytes = b.finish().unwrap().save(&SaveOptions::default()).unwrap().bytes;
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

#[test]
fn a_contents_page_in_a_tagged_document_should_join_its_structure_tree() {
    let mut doc = tagged();
    doc.create_contents_page(&options(), &fonts()).unwrap();
    let mut out = Vec::new();
    kinds(&doc.structure_tree().unwrap().unwrap(), &mut out);
    let toc = out.iter().filter(|k| *k == "TOC").count();
    let entries = out.iter().filter(|k| *k == "TOCI").count();
    assert_eq!((toc, entries), (1, 2), "{out:?}");
}

#[test]
fn a_listing_should_read_back_from_json_as_it_was() {
    let listing = outline().contents_listing(&options()).unwrap();
    let json = serde_json::to_string(&listing).unwrap();
    assert_eq!(serde_json::from_str::<ContentsListing>(&json).unwrap(), listing);
}

#[test]
fn inserting_a_page_laid_out_before_an_edit_should_be_refused() {
    let mut doc = outline();
    let listing = doc.contents_listing(&options()).unwrap();
    let page = listing.lay_out(&fonts()).unwrap();
    // Deleting the last page sends Chapter 2's bookmark to the page before, whose label differs.
    let last = doc.page_info(3).unwrap().id;
    doc.apply(vec![laminapdf::Op::from(laminapdf::DeletePages::new(vec![last]))])
        .unwrap();
    let err = doc.insert_contents_page(&options(), page).unwrap_err();
    assert_eq!(err.code(), "document-changed");
}

#[test]
fn a_page_laid_out_from_a_listing_should_insert_as_create_does() {
    let mut split = outline();
    let listing = split.contents_listing(&options()).unwrap();
    split
        .insert_contents_page(&options(), listing.lay_out(&fonts()).unwrap())
        .unwrap();
    let mut whole = outline();
    whole.create_contents_page(&options(), &fonts()).unwrap();
    let save = |doc: &mut Document| doc.save(&SaveOptions::default()).unwrap().bytes;
    assert_eq!(save(&mut split), save(&mut whole));
}

#[test]
fn create_contents_page_should_refuse_a_standard_font_given_on_a_pdf_ua_file() {
    let mut doc = Document::open_bytes(fixture("golden/flow-table.pdf"), OpenOptions::default()).unwrap();
    let helvetica = ContentsFonts::new().with_font(Font::standard(laminapdf::Standard14::Helvetica));
    let err = doc.create_contents_page(&options(), &helvetica).unwrap_err();
    assert_eq!(err.code(), "font-not-embedded");
}

/// An untagged document of `count` pages, each headed "Chapter n" (a bookmark per page).
fn chapters(count: usize) -> Document {
    use laminapdf::flow::{Flow, FlowBlock, PageTemplate, Paragraph, TextStyle};
    use laminapdf::{BuildOptions, Builder, PageSize};
    let mut b = Builder::new(BuildOptions::new());
    let font = b.font(noto()).unwrap();
    let body = TextStyle::new(&font, 11.0);
    let mut flow = Flow::new(PageTemplate::new(PageSize::A4)).with_tagged(false);
    for n in 1..=count {
        if n > 1 {
            flow = flow.block(FlowBlock::PageBreak);
        }
        flow = flow.block(FlowBlock::Heading(1, Paragraph::plain(format!("Chapter {n}"), &body)));
    }
    b.flow(&flow).unwrap();
    let bytes = b.finish().unwrap().save(&SaveOptions::default()).unwrap().bytes;
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

#[test]
fn a_contents_page_over_several_pages_should_print_the_pages_its_links_go_to() {
    let mut doc = chapters(120);
    doc.create_contents_page(&options(), &fonts()).unwrap();
    let added = doc.page_count().unwrap() - 120;
    let mut printed = Vec::new();
    let mut linked = Vec::new();
    for page in 0..added {
        let lines = text(&doc, page);
        // Each entry's line ends with the number it prints; the heading's has none.
        printed.extend(
            lines
                .lines()
                .filter_map(|l| l.split_whitespace().last()?.parse::<u32>().ok()),
        );
        linked.extend(link_pages(&doc, page).into_iter().map(|p| p + 1));
    }
    assert_eq!((added > 1, printed.len(), printed), (true, 120, linked));
}

#[test]
fn contents_listing_should_refuse_an_entry_size_that_is_not_a_size() {
    let doc = outline();
    let codes: Vec<String> = [0.0, -11.0, f64::NAN, 500.0]
        .into_iter()
        .map(|size| {
            let error = doc.contents_listing(&options().with_size(size)).unwrap_err();
            error.code().to_owned()
        })
        .collect();
    assert_eq!(codes, ["invalid-op"; 4]);
}
