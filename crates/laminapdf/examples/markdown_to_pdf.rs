//! A report written in Markdown, turned into tagged pages that flow: headings, paragraphs with
//! emphasis, code and links, a photo, lists and a task list, a block quote, a table that runs
//! across pages, a code block, footnotes, and paragraphs in Arabic and Hindi.
//!
//! The front matter gives the title and language, a contents page lists the headings, and the
//! output is checked against PDF/UA-1, so every font is embedded. Run it with `cargo run -p
//! laminapdf --example markdown_to_pdf -- report.md regular.ttf bold.ttc arabic.ttf
//! devanagari.ttf photo.jpg`.

use std::error::Error;

use laminapdf::flow::{PageRegion, PageTemplate, TextStyle};
use laminapdf::markdown::{MarkdownFonts, MarkdownOptions};
use laminapdf::{BuildOptions, Builder, Font, Image, PageSize, Profile, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let [markdown, regular, bold, arabic, hindi, photo] = paths.as_slice() else {
        return Err("arguments: <report.md> <regular.ttf> <bold.ttc> <arabic.ttf> <devanagari.ttf> <photo.jpg>".into());
    };
    let read = |path: &String| std::fs::read(path);
    let fonts = [read(regular)?, read(bold)?, read(arabic)?, read(hindi)?];
    let markdown = std::fs::read_to_string(markdown)?;
    let pdf = build(BuildOptions::new(), &markdown, fonts, read(photo)?)?;
    std::fs::create_dir_all("target/examples-out")?;
    Ok(std::fs::write("target/examples-out/markdown_to_pdf.pdf", pdf)?)
}

/// The report's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(
    options: BuildOptions,
    markdown: &str,
    fonts: [Vec<u8>; 4],
    photo: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let [regular, bold, arabic, hindi] = fonts;
    let mut b = Builder::new(options.with_profile(Profile::PdfUa1));
    let regular = b.font(Font::from_bytes(regular)?)?;
    let fonts = MarkdownFonts::new(&regular)
        .with_bold(&b.font(Font::from_collection(bold, 1)?)?)
        .with_fallback(&b.font(Font::from_bytes(arabic)?)?)
        .with_fallback(&b.font(Font::from_bytes(hindi)?)?);
    // A5 pages, so the table runs across two of them, with the page number in the footer.
    let footer = PageRegion::new().with_center("Page {page} of {pages}");
    let pages = PageTemplate::new(PageSize::A5).with_furniture(None, Some(footer), &TextStyle::new(&regular, 8.0));
    let options = MarkdownOptions::new()
        .with_fonts(fonts)
        .with_template(pages)
        .with_contents(2)
        .with_image("photo.jpg", Image::jpeg(photo)?);
    let report = b.markdown(markdown, &options)?;
    if let Some(warning) = report.warnings.first() {
        return Err(format!("the Markdown has something the PDF leaves out: {}", warning.code()).into());
    }
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}
