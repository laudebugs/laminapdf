//! An archival letter, written as PDF/A-2b and as PDF/A-2u that is also PDF/UA-1.
//!
//! Run it with `cargo run -p laminapdf --example pdf_a_archive -- [out-dir]`: it writes
//! `pdf_a_archive-2b.pdf` and `pdf_a_archive-2u.pdf`.

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    ArtifactKind, BuildError, BuildOptions, Builder, Color, FillRule, FontHandle, Image, Metadata, PageSize, PointPt,
    Profile, Pt, RectPt, SaveOptions, Surface, TagKind, TextBoxOptions,
};

const MARGIN: f64 = 56.0;
const BAND_HEIGHT: f64 = 70.0;
const TITLE_AT: PointPt = PointPt::from_points(MARGIN, 110.0);
/// The top of the letter's text and of the photo beside it, and the height of both.
const BLOCK_TOP: f64 = 130.0;
const BLOCK_HEIGHT: f64 = 120.0;
const LETTER_WIDTH: f64 = 300.0;
const PHOTO_LEFT: f64 = 380.0;
const PHOTO_WIDTH: f64 = 160.0;
const FOOTER_AT: PointPt = PointPt::from_points(MARGIN, 300.0);
const TITLE: Pt = Pt::new(22.0);
const BODY: Pt = Pt::new(11.0);
const BAND_OPACITY: f64 = 0.35;
const PHOTO_ALT: &str = "A photograph of the sprockets, packed in a box";
const LETTER: &str = "Thank you for your order of 12 sprockets. This letter confirms the delivery date and the \
                      price we agreed. Keep it: PDF/A files look the same in any viewer, now and in the future.";

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("pdf_a_archive-2b.pdf"), build(Level::A2b)?)?;
    std::fs::write(out.join("pdf_a_archive-2u.pdf"), build(Level::A2u)?)?;
    Ok(())
}

/// Which archive to write.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Level {
    /// PDF/A-2b: the letter looks the same everywhere.
    A2b,
    /// PDF/A-2u, whose text maps to Unicode, and PDF/UA-1.
    A2u,
}

/// The letter's bytes.
pub(crate) fn build(level: Level) -> Result<Vec<u8>, Box<dyn Error>> {
    let options = match level {
        Level::A2b => BuildOptions::new().with_profile(Profile::PdfA2b),
        Level::A2u => BuildOptions::new()
            .with_profile(Profile::PdfA2u)
            .with_profile(Profile::PdfUa1),
    };
    let mut b = Builder::new(options.with_tagged(true));
    b.metadata(
        Metadata::default()
            .with_title("Order confirmation 42")
            .with_author("Acme Widgets Ltd"),
    );
    b.lang("en-GB");
    let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
    let photo = Image::jpeg(include_bytes!("../../../fixtures/images/photo-rgb.jpg").to_vec())?;
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &font, &photo)?;
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

fn draw(s: &mut Surface, font: &FontHandle, photo: &Image) -> Result<(), BuildError> {
    // A translucent band across the top: decoration, and transparency, which PDF/A-2 allows.
    let width = PageSize::A4.width.get();
    s.artifact(ArtifactKind::Layout, |s| {
        s.save()
            .opacity(BAND_OPACITY, 1.0)
            .fill_color(Color::rgb(0.12, 0.47, 0.78));
        s.path(|p| _ = p.rect(RectPt::from_points(0.0, 0.0, width, BAND_HEIGHT)))
            .fill(FillRule::NonZero);
        s.restore();
        Ok(())
    })?;
    s.tag(TagKind::H1, |s| {
        s.text("Order confirmation 42", font, TITLE, TITLE_AT).map(drop)
    })?;
    let body = RectPt::from_points(MARGIN, BLOCK_TOP, LETTER_WIDTH, BLOCK_HEIGHT);
    s.tag(TagKind::P, |s| {
        s.text_box(LETTER, font, BODY, body, &TextBoxOptions::new()).map(drop)
    })?;
    let beside = RectPt::from_points(PHOTO_LEFT, BLOCK_TOP, PHOTO_WIDTH, BLOCK_HEIGHT);
    s.figure(photo, beside, Some(PHOTO_ALT));
    let footer = "Acme Widgets Ltd, 1 Example Street, London";
    s.tag(TagKind::P, |s| s.text(footer, font, BODY, FOOTER_AT).map(drop))
}
