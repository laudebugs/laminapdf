//! A report that flows across pages: a cover, then sections whose paragraphs break between
//! pages, with the section's title in a running header and "Page X of Y" in the footer.
//!
//! Lines split with at least two at the foot and the head of a page, headings stay with what
//! follows them, and the photo moves whole. The output is tagged and checked against PDF/UA-1.
//! Run it with `cargo run -p laminapdf --example flowing_report -- regular.ttf bold.ttc arabic.ttf
//! devanagari.ttf photo.jpg`.

use std::error::Error;

use laminapdf::flow::{
    Figure, Flow, FlowBlock, List, ListItem, ListKind, PageRegion, PageTemplate, Paragraph, ParagraphStyle, TextStyle,
};
use laminapdf::{BuildOptions, Builder, Font, Image, Metadata, PageSize, Profile, SaveOptions};

const TITLE: &str = "Reading files offline: a year of the reader";
pub(crate) const ARABIC: &str =
    "يعمل القارئ دون اتصال بالشبكة، فتُفتح الملفات وتُقرأ وتُبحث على الجهاز نفسه، ولا يغادر أي منها الحاسوب.";
pub(crate) const HINDI: &str = "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण।";
const TOPICS: [&str; 5] = ["Opening files", "Search", "Printing", "Signing", "What comes next"];

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let [regular, bold, arabic, hindi, photo] = paths.as_slice() else {
        return Err("arguments: <regular.ttf> <bold.ttc> <arabic.ttf> <devanagari.ttf> <photo.jpg>".into());
    };
    let fonts = [regular, bold, arabic, hindi]
        .map(std::fs::read)
        .map(Result::unwrap_or_default);
    let pdf = build(BuildOptions::new(), fonts, std::fs::read(photo)?)?;
    std::fs::create_dir_all("target/examples-out")?;
    Ok(std::fs::write("target/examples-out/flowing_report.pdf", pdf)?)
}

/// The report's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(options: BuildOptions, fonts: [Vec<u8>; 4], photo: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let [regular, bold, arabic, hindi] = fonts;
    let mut b = Builder::new(options.with_profile(Profile::PdfUa1));
    b.metadata(Metadata::default().with_title(TITLE));
    b.lang("en");
    let regular = b.font(Font::from_bytes(regular)?)?;
    let body = TextStyle::new(&regular, 11.0)
        .with_fallback(&b.font(Font::from_bytes(arabic)?)?)
        .with_fallback(&b.font(Font::from_bytes(hindi)?)?);
    let heading = TextStyle::new(&b.font(Font::from_collection(bold, 1)?)?, 16.0);
    let small = TextStyle::new(&regular, 9.0);
    let header = PageRegion::new().with_end("{section}");
    let footer = PageRegion::new().with_center("Page {page} of {pages}");
    let pages = PageTemplate::new(PageSize::A5).with_furniture(Some(header), Some(footer), &small);
    let mut flow = Flow::new(pages.with_first_page(PageTemplate::new(PageSize::A5)))
        .with_title(TITLE)
        .block(FlowBlock::Heading(
            1,
            Paragraph::plain(TITLE, &heading.clone().with_size(24.0)),
        ))
        .block(FlowBlock::Paragraph(Paragraph::plain(
            "A report from the reader's team.",
            &body,
        )))
        .block(FlowBlock::PageBreak);
    let photo = Image::jpeg(photo)?;
    for (i, topic) in TOPICS.iter().enumerate() {
        flow = section(flow, i, topic, &body, &heading, &photo);
    }
    b.flow(&flow)?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// A section: its heading, paragraphs (one in Arabic, one in Hindi), a list and a photo.
fn section(flow: Flow, i: usize, topic: &str, body: &TextStyle, heading: &TextStyle, photo: &Image) -> Flow {
    let sentence = format!(
        "The {} work of the year changed how readers use the app every day. ",
        topic.to_lowercase()
    );
    let mut flow = flow.block(FlowBlock::Heading(1, Paragraph::plain(topic, heading)));
    for n in 0..4 {
        let style = ParagraphStyle::new().with_space(0.0, 8.0);
        flow = flow.block(FlowBlock::Paragraph(
            Paragraph::new(style).text(sentence.repeat(n + 4), body),
        ));
    }
    let foreign = match i {
        1 => Some((format!("{ARABIC} ").repeat(8).trim_end().to_owned(), "ar")),
        3 => Some((format!("{HINDI} ").repeat(12).trim_end().to_owned(), "hi")),
        _ => None,
    };
    if let Some((text, lang)) = foreign {
        let style = body.clone().with_language(lang);
        flow = flow.block(FlowBlock::Paragraph(Paragraph::plain(text, &style)));
    }
    let list = ["faster", "smaller", "easier to share"]
        .iter()
        .fold(List::new(ListKind::Bullet, body), |list, word| {
            list.item(ListItem::new(Paragraph::plain(format!("Files are {word}."), body)))
        });
    let alt = format!("A photo of the team working on {}", topic.to_lowercase());
    flow.block(FlowBlock::List(list))
        .block(FlowBlock::Figure(Figure::new(photo.clone(), alt).with_width(200.0)))
}
