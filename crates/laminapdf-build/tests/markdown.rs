//! Markdown in, flowing pages out (todo T4d): each construct of ADR 0032 §6's dialect builds the
//! block and structure type its mapping row names, front matter becomes the metadata, `#slug`
//! links reach their heading, raw HTML and missing images are dropped with a coded warning
//! that carries an offset and no document text, and the standard 14 defaults set Latin text
//! but fail under PDF/UA-1.
#![cfg(feature = "markdown")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{reopen, save};
use laminapdf_build::markdown::{MarkdownBuildOptions, MarkdownFonts};
use laminapdf_build::{BuildOptions, Builder, Error, Image, Profile, Warning};
use laminapdf_model::{StructElem, StructKid};
use laminapdf_test_fixtures::fixture;
use laminapdf_text::{Font, TextExtractor};

/// A Markdown document built with Noto Sans in every role, uncompressed, saved.
fn build(markdown: &str, options: MarkdownBuildOptions) -> (Vec<u8>, Vec<Warning>) {
    let mut b = Builder::new(BuildOptions::new().with_id(*b"lamina-markdown!").with_compress(false));
    let noto = b
        .font(Font::from_bytes(fixture("fonts/NotoSans-Regular.ttf")).unwrap())
        .unwrap();
    let report = b
        .markdown(markdown, &options.with_fonts(MarkdownFonts::new(&noto)))
        .unwrap();
    (save(b.finish().unwrap()), report.warnings)
}

/// The tree as nested kinds, e.g. `Document(H1 P(Span))`; marked content is left out.
fn outline(e: &StructElem) -> String {
    let kind = String::from_utf8_lossy(&e.kind).into_owned();
    let kids: Vec<String> = e.children().map(outline).collect();
    if kids.is_empty() {
        kind
    } else {
        format!("{kind}({})", kids.join(" "))
    }
}

/// The structure tree of `markdown`, under its `Document` element.
fn tree(markdown: &str) -> String {
    tree_with(markdown, MarkdownBuildOptions::new())
}

fn tree_with(markdown: &str, options: MarkdownBuildOptions) -> String {
    let (bytes, _) = build(markdown, options);
    let doc = reopen(&bytes);
    let tree = doc.structure().unwrap().unwrap();
    let root = &tree.roots[0];
    assert_eq!(root.kind, b"Document");
    root.children().map(outline).collect::<Vec<_>>().join(" ")
}

/// Every element of `kind` in `markdown`'s tree.
fn elements(bytes: &[u8], kind: &[u8]) -> Vec<StructElem> {
    let doc = reopen(bytes);
    let tree = doc.structure().unwrap().unwrap();
    tree.elements()
        .into_iter()
        .filter(|e| e.kind == kind)
        .cloned()
        .collect()
}

/// Whether a page draws a `Layout` artifact (a rule or a strike line).
fn layout_artifact(bytes: &[u8]) -> bool {
    bytes.windows(13).any(|w| w == b"/Type /Layout")
}

fn text(bytes: &[u8]) -> String {
    let doc = reopen(bytes);
    let mut extractor = TextExtractor::new();
    let pages = doc.pages().unwrap();
    pages
        .iter()
        .flat_map(|p| extractor.page(&doc, &p).unwrap().lines())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn headings_are_h1_to_h6() {
    let md = "# One\n\n## Two\n\n### Three\n\n#### Four\n\n##### Five\n\n###### Six\n";
    assert_eq!(tree(md), "H1 H2 H3 H4 H5 H6");
}

#[test]
fn a_paragraph_is_p_and_a_hard_break_keeps_it_one_paragraph() {
    assert_eq!(tree("First line  \nsecond line.\n\nAnother.\n"), "P P");
    let (bytes, _) = build("First line  \nsecond line.\n", MarkdownBuildOptions::new());
    assert!(text(&bytes).contains("First line\nsecond line."));
}

#[test]
fn emphasis_strong_and_strikethrough_are_spans_and_code_is_code() {
    let md = "Some *soft* and **loud** and ~~gone~~ and `code` words.\n";
    let t = tree(md);
    assert!(t.starts_with("P("), "{t}");
    assert!(t.contains("Code"), "{t}");
    let (bytes, _) = build(md, MarkdownBuildOptions::new());
    assert!(text(&bytes).contains("Some soft and loud and gone and code words."));
    // The strike line is a layout artifact.
    assert!(layout_artifact(&bytes));
}

#[test]
fn a_link_is_a_link_element_and_a_slug_link_goes_to_its_heading() {
    let md = "# Getting started\n\nSee [the site](https://example.com) and [above](#getting-started).\n";
    assert_eq!(tree(md), "H1 P(Link Link)");
    let (bytes, _) = build(md, MarkdownBuildOptions::new());
    assert!(bytes.windows(19).any(|w| w == b"https://example.com"));
    assert!(
        bytes.windows(16).any(|w| w == b"/Dest (flow0-h0)"),
        "the slug link names the heading's destination"
    );
}

#[test]
fn a_heading_in_a_list_item_or_note_doesn_t_shift_the_slug_links() {
    let md = "- # In an item\n\nA note.[^n]\n\n# Target\n\n[go](#target)\n\n[^n]: # In a note\n";
    let (bytes, _) = build(md, MarkdownBuildOptions::new());
    assert!(bytes.windows(16).any(|w| w == b"/Dest (flow0-h0)"));
    assert!(!bytes.windows(15).any(|w| w == b"(flow0-h1)"));
}

#[test]
fn a_slug_link_in_a_footnote_goes_to_its_heading() {
    let md = "# Target\n\nA note.[^n]\n\n[^n]: See [the target](#target).\n";
    let (bytes, _) = build(md, MarkdownBuildOptions::new());
    let links = bytes.windows(16).filter(|w| *w == b"/Dest (flow0-h0)").count();
    assert_eq!(links, 2, "the outline item and the note's link");
}

#[test]
fn front_matter_keeps_the_metadata_the_caller_set() {
    let mut b = Builder::new(BuildOptions::new().with_compress(false));
    b.metadata(
        laminapdf_build::Metadata::default()
            .with_author("Caller")
            .with_title("Old"),
    );
    b.markdown("---\ntitle: New\n---\n\nBody.\n", &MarkdownBuildOptions::new())
        .unwrap();
    let bytes = save(b.finish().unwrap());
    let doc = reopen(&bytes);
    let info = doc.info().unwrap().unwrap();
    assert_eq!(info.title().unwrap().as_deref(), Some("New"));
    assert_eq!(info.author().unwrap().as_deref(), Some("Caller"));
}

#[test]
fn a_supplied_image_is_a_figure_with_its_text_as_alt() {
    let png = common::png((2, 2), png::ColorType::Rgb, png::BitDepth::Eight, &[0; 12], None, None);
    let options = MarkdownBuildOptions::new().with_image("chart.png", Image::png(&png).unwrap());
    let (bytes, warnings) = build("Before.\n\n![A small chart](chart.png)\n\nAfter.\n", options);
    assert!(warnings.is_empty(), "{warnings:?}");
    let figures = elements(&bytes, b"Figure");
    assert_eq!(figures.len(), 1);
    assert_eq!(figures[0].alt.as_deref(), Some("A small chart"));
}

#[test]
fn an_image_in_a_table_cell_or_list_item_keeps_its_alt_text_there() {
    let png = common::png((2, 2), png::ColorType::Rgb, png::BitDepth::Eight, &[0; 12], None, None);
    let options = MarkdownBuildOptions::new().with_image("dot.png", Image::png(&png).unwrap());
    let md = "| Before ![a dot](dot.png) after |\n| --- |\n| x |\n\n- Item ![a dot](dot.png) end\n";
    let (bytes, _) = build(md, options);
    assert_eq!(elements(&bytes, b"Figure").len(), 0);
    let t = text(&bytes);
    assert!(t.contains("Before a dot after") && t.contains("Item a dot end"), "{t}");
}

#[test]
fn lists_are_l_li_lbl_lbody_with_their_start_number_and_task_markers() {
    let item = "LI(Lbl LBody)";
    assert_eq!(tree("- one\n- two\n"), format!("L({item} {item})"));
    let (bytes, _) = build("3. three\n4. four\n", MarkdownBuildOptions::new());
    let t = text(&bytes);
    assert!(t.contains("3.") && t.contains("4."), "{t}");
    let (bytes, _) = build("- [ ] open\n- [x] done\n", MarkdownBuildOptions::new());
    let t = text(&bytes);
    // Noto Sans has no ☐ or ☑, so the markers fall back to brackets.
    assert!(t.contains("[ ] open") && t.contains("[x] done"), "{t}");
}

#[test]
fn a_block_quote_is_a_blockquote() {
    assert_eq!(tree("> Quoted words.\n>\n> More.\n"), "BlockQuote(P P)");
}

#[test]
fn a_code_block_is_a_p_holding_code() {
    let md = "```\nfn main() {}\nlet x = 1;\n```\n";
    assert_eq!(tree(md), "P(Code)");
    let (bytes, _) = build(md, MarkdownBuildOptions::new());
    assert!(text(&bytes).contains("fn main() {}\nlet x = 1;"));
}

#[test]
fn a_thematic_break_is_a_layout_artifact_and_no_element() {
    assert_eq!(tree("Above.\n\n---\n\nBelow.\n"), "P P");
    let (bytes, _) = build("Above.\n\n---\n\nBelow.\n", MarkdownBuildOptions::new());
    assert!(layout_artifact(&bytes));
}

#[test]
fn a_table_has_a_header_row_and_a_body() {
    let md = "| Name | Count |\n| :--- | ---: |\n| Ash | 3 |\n| Oak | 12 |\n";
    assert_eq!(tree(md), "Table(THead(TR(TH TH)) TBody(TR(TD TD) TR(TD TD)))");
}

#[test]
fn a_footnote_is_a_reference_to_a_note() {
    let md = "A claim.[^1] More.\n\n[^1]: The source.\n";
    let t = tree(md);
    assert!(t.contains("Reference"), "{t}");
    assert!(t.contains("Note"), "{t}");
    let (bytes, _) = build(md, MarkdownBuildOptions::new());
    assert!(text(&bytes).contains("The source."));
}

#[test]
fn front_matter_sets_the_title_language_author_and_subject() {
    let md = "---\ntitle: Field notes\nlang: en-GB\nauthor: A. Writer\nsubject: Trees\n---\n\nBody.\n";
    let (bytes, _) = build(md, MarkdownBuildOptions::new());
    let doc = reopen(&bytes);
    let info = doc.info().unwrap().unwrap();
    assert_eq!(info.title().unwrap().as_deref(), Some("Field notes"));
    assert_eq!(info.author().unwrap().as_deref(), Some("A. Writer"));
    assert_eq!(info.subject().unwrap().as_deref(), Some("Trees"));
    assert!(
        bytes.windows(13).any(|w| w == b"/Lang (en-GB)"),
        "the catalog's language"
    );
    assert!(!text(&bytes).contains("title:"), "front matter isn't body text");
}

#[test]
fn raw_html_and_a_missing_image_are_dropped_with_their_offsets() {
    let md = "<div>MARKER-HTML</div>\n\nText <b>bold</b>.\n\n![alt](MARKER-IMAGE.png)\n";
    let (bytes, warnings) = build(md, MarkdownBuildOptions::new());
    assert_eq!(
        warnings,
        [
            Warning::MarkdownHtmlIgnored { offset: 0 },
            Warning::MarkdownHtmlIgnored { offset: 29 },
            Warning::MarkdownHtmlIgnored { offset: 36 },
            Warning::MarkdownImageMissing { offset: 43 },
        ]
    );
    assert!(!text(&bytes).contains("MARKER"));
    for w in &warnings {
        assert!(!w.to_string().contains("MARKER"), "{w}");
    }
}

#[test]
fn without_fonts_latin_text_builds_in_the_standard_14_but_not_under_pdf_ua() {
    let md = "---\ntitle: Plain\nlang: en\n---\n\n# Plain\n\nSome *Latin* text and `code`.\n";
    let mut b = Builder::new(BuildOptions::new().with_compress(false));
    b.markdown(md, &MarkdownBuildOptions::new()).unwrap();
    let bytes = save(b.finish().unwrap());
    assert!(text(&bytes).contains("Some Latin text and code."));
    assert!(bytes.windows(15).any(|w| w == b"/BaseFont /Helv"));
    assert!(bytes.windows(17).any(|w| w == b"/BaseFont /Courie"));

    let mut b = Builder::new(BuildOptions::new().with_profile(Profile::PdfUa1));
    b.markdown(md, &MarkdownBuildOptions::new()).unwrap();
    let err = b.finish().unwrap_err();
    assert!(matches!(err, Error::FontNotEmbedded { .. }), "{err:?}");
}

#[test]
fn a_contents_page_lists_the_headings() {
    let md = "# Alpha\n\nText.\n\n## Beta\n\nText.\n";
    let t = tree_with(md, MarkdownBuildOptions::new().with_contents(2));
    assert!(t.starts_with("TOC("), "{t}");
}

#[test]
fn every_heading_s_marked_content_is_on_a_page() {
    let (bytes, _) = build("# One\n\nText.\n", MarkdownBuildOptions::new());
    let h1 = &elements(&bytes, b"H1")[0];
    assert!(h1.kids.iter().any(|k| matches!(k, StructKid::Mcid { .. })));
}
