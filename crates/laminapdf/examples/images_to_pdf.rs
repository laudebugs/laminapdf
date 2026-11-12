//! Photos into a PDF, one A4 page each, fitted inside the margins.
//!
//! Each JPEG is embedded as it is, without decoding or compressing it again. A page turns to
//! landscape for a wide photo, and a photo that its camera marked as taken sideways is drawn
//! upright.
//!
//! ```sh
//! cargo run -p laminapdf --example images_to_pdf -- out-dir photo.jpg [more.jpg …]   # writes out-dir/images_to_pdf.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{BuildOptions, Image, ImagePages, ImagesOptions, PageSize, Pt, SaveOptions, images_to_pdf};

/// The space around each photo, in millimetres.
const MARGIN_MM: f64 = 15.0;
const USAGE: &str = "usage: images_to_pdf <out-dir> <photo.jpg> [more.jpg …]";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or(USAGE)?);
    let photos = args.map(std::fs::read).collect::<Result<Vec<_>, _>>()?;
    if photos.is_empty() {
        return Err(USAGE.into());
    }
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("images_to_pdf.pdf"), build(BuildOptions::new(), photos)?)?;
    Ok(())
}

/// The document's bytes, one page per photo (JPEGs). `options` can fix the file ID, the producer
/// and the creation date.
pub(crate) fn build(options: BuildOptions, photos: Vec<Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    let images = photos.into_iter().map(Image::jpeg).collect::<Result<Vec<_>, _>>()?;
    let options = ImagesOptions::new()
        .with_pages(ImagePages::Fit(PageSize::A4))
        .with_margin(Pt::from_mm(MARGIN_MM))
        .with_build(options);
    let mut doc = images_to_pdf(&images, &options)?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
