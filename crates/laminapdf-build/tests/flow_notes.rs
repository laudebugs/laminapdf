//! Footnotes, contents pages and the outline in flows (todo T4h): every footnote's marker is on
//! the page of its note, each `Note` has its own `/ID`, every contents entry prints the page its
//! heading starts on and links to its destination, a contents page of two pages settles, and
//! the outline nests the headings.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{reopen, save};
use laminapdf_build::flow::{Contents, Flow, FlowBlock, Margins, PageTemplate, Paragraph, ParagraphStyle, TextStyle};
use laminapdf_build::{BuildOptions, Builder, FontHandle, PageSize};
use laminapdf_model::{PageId, StructElem, StructKid};
use laminapdf_test_fixtures::fixture;
use laminapdf_text::{Font, TextExtractor};

const SIZE: f64 = 10.0;
const NAMES: [&str; 8] = [
    "Alder",
    "Birch",
    "Cedar",
    "Douglas fir",
    "Elm",
    "Fig",
    "Ginkgo",
    "Hazel",
];

fn noto(b: &mut Builder) -> FontHandle {
    b.font(Font::from_bytes(fixture("fonts/NotoSans-Regular.ttf")).unwrap())
        .unwrap()
}

/// A small page: 300 × 400 points, 40-point margins.
fn template() -> PageTemplate {
    PageTemplate::new(PageSize::new(
        laminapdf_model::Pt::new(300.0),
        laminapdf_model::Pt::new(400.0),
    ))
    .with_margins(Margins::all(40.0))
}

/// A section per name (a level-1 heading, a paragraph with two notes, a level-2 heading and a
/// paragraph), `sections` of them, after a contents page and a page break.
fn document(sections: usize) -> Vec<u8> {
    let mut b = Builder::new(BuildOptions::new().with_id(*b"lamina-flow-note").with_compress(false));
    let font = noto(&mut b);
    let body = TextStyle::new(&font, SIZE);
    let small = body.clone().with_size(8.0);
    let mut flow = Flow::new(template())
        .block(FlowBlock::Contents(Contents::new(&body)))
        .block(FlowBlock::PageBreak);
    for n in 0..sections {
        let name = NAMES[n % NAMES.len()];
        let title = if n < NAMES.len() {
            name.to_owned()
        } else {
            format!("More {name}")
        };
        let words = format!(
            "The {} grows in many places and is easy to tell apart. ",
            name.to_lowercase()
        );
        let paragraph = Paragraph::new(ParagraphStyle::new())
            .text(words.repeat(6), &body)
            .note(Paragraph::plain(
                format!("On the {}: a first note.", name.to_lowercase()),
                &small,
            ))
            .text(words.repeat(4), &body)
            .note(Paragraph::plain(
                "A second note, longer, which runs on to a second line at the foot of the page.",
                &small,
            ));
        flow = flow
            .block(FlowBlock::Heading(
                1,
                Paragraph::plain(title.clone(), &body.clone().with_size(14.0)),
            ))
            .block(FlowBlock::Paragraph(paragraph))
            .block(FlowBlock::Heading(
                2,
                Paragraph::plain(format!("Where {title} grows"), &body),
            ))
            .block(FlowBlock::Paragraph(Paragraph::plain(words.repeat(3), &body)));
    }
    b.flow(&flow).unwrap();
    save(b.finish().unwrap())
}

/// The 0-based page of the first marked content of `e` or the elements inside it.
fn first_page(e: &StructElem, pages: &[PageId]) -> Option<usize> {
    std::iter::once(e).chain(e.descendants()).find_map(|e| {
        e.kids.iter().find_map(|k| match k {
            StructKid::Mcid { page, .. } => pages.iter().position(|p| Some(p) == page.as_ref()),
            _ => None,
        })
    })
}

/// Each element of `kind`, in structure order, with the page it starts on.
fn starts(bytes: &[u8], kind: &[u8]) -> Vec<usize> {
    let doc = reopen(bytes);
    let pages: Vec<PageId> = doc.pages().unwrap().iter().map(|p| p.id().clone()).collect();
    let tree = doc.structure().unwrap().unwrap();
    tree.elements()
        .into_iter()
        .filter(|e| e.kind == kind)
        .map(|e| first_page(e, &pages).unwrap())
        .collect()
}

/// The contents entries as (title, printed page number), read from the contents pages' text.
fn entries(bytes: &[u8]) -> Vec<(String, usize)> {
    let doc = reopen(bytes);
    let mut extractor = TextExtractor::new();
    let pages = doc.pages().unwrap();
    let mut out = Vec::new();
    for page in pages.iter() {
        let lines = extractor.page(&doc, &page).unwrap().lines();
        if lines.iter().any(|l| l.starts_with("The ")) {
            break; // the body has started
        }
        for line in lines {
            let (title, number) = line.trim().rsplit_once(' ').unwrap();
            out.push((title.trim().to_owned(), number.parse().unwrap()));
        }
    }
    out
}

#[test]
fn every_note_is_on_the_page_of_its_marker_and_has_its_own_id() {
    let bytes = document(6);
    let (markers, notes) = (starts(&bytes, b"Reference"), starts(&bytes, b"Note"));
    assert_eq!(markers.len(), 12);
    assert_eq!(markers, notes);
    let text = String::from_utf8_lossy(&bytes);
    let mut ids: Vec<&str> = text
        .match_indices("/ID (Note")
        .map(|(at, _)| &text[at..at + 14])
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 12, "{ids:?}");
}

#[test]
fn every_contents_entry_prints_its_heading_s_page_and_links_to_it() {
    let bytes = document(6);
    let headings: Vec<usize> = {
        let (h1, h2) = (starts(&bytes, b"H1"), starts(&bytes, b"H2"));
        // Structure order alternates H1 and H2, one of each per section.
        h1.iter().zip(&h2).flat_map(|(a, b)| [*a, *b]).collect()
    };
    let listed = entries(&bytes);
    assert_eq!(listed.len(), 12);
    let printed: Vec<usize> = listed.iter().map(|(_, n)| *n).collect();
    let actual: Vec<usize> = headings.iter().map(|p| p + 1).collect();
    assert_eq!(printed, actual);
    assert_eq!(listed[0].0, "Alder");
    assert_eq!(listed[1].0, "Where Alder grows");
    let text = String::from_utf8_lossy(&bytes);
    for n in 0..12 {
        assert!(text.contains(&format!("(flow0-h{n})")), "no destination flow0-h{n}");
    }
    // Two links per entry, its text and its number.
    assert_eq!(starts(&bytes, b"TOCI").len(), 12);
    assert_eq!(starts(&bytes, b"TOC").len(), 1);
}

#[test]
fn a_contents_page_of_two_pages_settles_with_the_right_numbers() {
    let bytes = document(16);
    let listed = entries(&bytes);
    assert_eq!(listed.len(), 32);
    assert!(
        starts(&bytes, b"TOCI").contains(&1),
        "the contents runs onto a second page"
    );
    let h1 = starts(&bytes, b"H1");
    let h2 = starts(&bytes, b"H2");
    let actual: Vec<usize> = h1.iter().zip(&h2).flat_map(|(a, b)| [a + 1, b + 1]).collect();
    assert_eq!(listed.iter().map(|(_, n)| *n).collect::<Vec<_>>(), actual);
}

#[test]
fn the_outline_nests_the_headings_and_goes_to_their_pages() {
    let bytes = document(3);
    let doc = reopen(&bytes);
    let outline = doc.outline().unwrap();
    let items: Vec<(String, usize, Option<usize>)> = outline
        .iter()
        .map(|i| {
            (
                i.title().unwrap(),
                i.depth(),
                i.target().unwrap().and_then(|d| d.page_index),
            )
        })
        .collect();
    let h1 = starts(&bytes, b"H1");
    let h2 = starts(&bytes, b"H2");
    let s = |t: &str| t.to_owned();
    assert_eq!(
        items,
        [
            (s("Alder"), 1, Some(h1[0])),
            (s("Where Alder grows"), 2, Some(h2[0])),
            (s("Birch"), 1, Some(h1[1])),
            (s("Where Birch grows"), 2, Some(h2[1])),
            (s("Cedar"), 1, Some(h1[2])),
            (s("Where Cedar grows"), 2, Some(h2[2])),
        ]
    );
}

#[test]
fn a_flow_without_an_outline_adds_none() {
    let mut b = Builder::new(BuildOptions::new());
    let font = noto(&mut b);
    let style = TextStyle::new(&font, SIZE);
    let flow = Flow::new(template())
        .with_outline(false)
        .block(FlowBlock::Heading(1, Paragraph::plain("Alone", &style)));
    b.flow(&flow).unwrap();
    let doc = reopen(&save(b.finish().unwrap()));
    assert!(doc.outline().unwrap().is_empty());
}
