//! An archival letter (PDF/A, todo B8b), written twice: as PDF/A-2b, and as PDF/A-2u that is
//! also PDF/UA-1. Both embed their font, use only RGB and gray colour (covered by the sRGB
//! output intent the profile writes), place a JPEG photo and a translucent band (transparency,
//! which PDF/A-2 allows), and carry the identification in their XMP packet.
//!
//! ```sh
//! cargo run -p lamina --example pdf_a_archive -- [out-dir]
//! # writes <out-dir>/pdf_a_archive-2b.pdf and <out-dir>/pdf_a_archive-2u.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use lamina::{
    ArtifactKind, BuildOptions, Builder, CancelToken, Color, FillRule, Image, Metadata, PageSize, PointPt, Profile, Pt,
    RectPt, SaveOptions, Tag, TagKind, TextBoxOptions,
};

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
    /// PDF/A-2b.
    A2b,
    /// PDF/A-2u and PDF/UA-1.
    A2u,
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

const BODY: &str = "Thank you for your order of 12 sprockets. This letter confirms the delivery date \
and the price we agreed. We keep a copy in our archive, and you can keep this one: PDF/A files look \
the same in any viewer, now and in the future.";

/// The document's bytes.
pub(crate) fn build(level: Level) -> Result<Vec<u8>, Box<dyn Error>> {
    let at = PointPt::from_points;
    let options = BuildOptions::default()
        .with_id(*b"lamina-ex-pdfa-2")
        .with_producer("lamina examples")
        .with_creation_date("D:20261031120000Z".parse()?);
    let options = match level {
        Level::A2b => options.with_profile(Profile::PdfA2b).with_tagged(true),
        Level::A2u => options.with_profile(Profile::PdfA2u).with_profile(Profile::PdfUa1),
    };
    let mut b = Builder::new(options);
    b.metadata(
        Metadata::default()
            .with_title("Order confirmation 42")
            .with_author("Acme Widgets Ltd"),
    )
    .lang("en-GB");
    let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
    let photo = Image::jpeg(include_bytes!("../../../fixtures/images/photo-rgb.jpg").to_vec())?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    s.artifact(ArtifactKind::Layout, |s| {
        s.save()
            .opacity(0.35, 1.0)
            .fill_color(Color::rgb(0.12, 0.47, 0.78))
            .path(|p| {
                p.rect(rect(0.0, 0.0, 595.28, 70.0));
            })
            .fill(FillRule::NonZero)
            .restore();
        Ok(())
    })?;
    let _ = s
        .text("Order confirmation 42", &font, Pt::new(22.0), at(56.0, 110.0))?
        .tag(TagKind::H1);
    s.tag(TagKind::P, |s| {
        s.text_box(
            BODY,
            &font,
            Pt::new(11.0),
            rect(56.0, 130.0, 300.0, 120.0),
            &TextBoxOptions::default(),
        )
        .map(|_| ())
    })?;
    s.figure(
        &photo,
        rect(380.0, 130.0, 160.0, 120.0),
        Some("A photograph of the sprockets, packed in a box"),
    );
    s.tag(Tag::new(TagKind::P), |s| {
        s.text(
            "Acme Widgets Ltd, 1 Example Street, London",
            &font,
            Pt::new(9.0),
            at(56.0, 300.0),
        )
        .map(|_| ())
    })?;
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new(), &CancelToken::new())?.bytes)
}
