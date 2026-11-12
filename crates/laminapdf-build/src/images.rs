//! [`images_to_pdf`]: a new PDF with one page per image (the editor plan's operation H.02).

use laminapdf_model::{Matrix, Pt, RectPt};

use crate::builder::{BuildOptions, Builder};
use crate::error::{Error, Result};
use crate::page::PageSize;
use crate::surface::image::Image;

/// How [`images_to_pdf`] sizes each page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ImagePages {
    /// Each page is its image's size at the image's resolution (JFIF density, PNG `pHYs`, or
    /// EXIF), or at [`ImagesOptions::with_default_dpi`] when the file gives none, plus the
    /// margins.
    ImageSize,
    /// Each page is this paper size, turned to landscape for an image wider than it is tall, and
    /// the image is scaled to fit inside the margins, centred.
    Fit(PageSize),
}

/// Options for [`images_to_pdf`].
///
/// ```
/// use laminapdf_build::{ImagePages, ImagesOptions, PageSize, Pt};
///
/// let options = ImagesOptions::new().with_pages(ImagePages::Fit(PageSize::A4)).with_margin(Pt::from_mm(10.0));
/// assert_eq!(options.pages(), ImagePages::Fit(PageSize::A4));
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
pub struct ImagesOptions {
    pages: ImagePages,
    margin: Pt,
    default_dpi: f64,
    build: BuildOptions,
}

impl Default for ImagesOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ImagesOptions {
    /// Pages the size of their images, no margin, 72 dpi for an image without a resolution (one
    /// pixel per point), and [`BuildOptions::default`].
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pages: ImagePages::ImageSize,
            margin: Pt::new(0.0),
            default_dpi: 72.0,
            build: BuildOptions::default(),
        }
    }

    /// How each page is sized.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_pages(mut self, pages: ImagePages) -> Self {
        self.pages = pages;
        self
    }

    /// The space around each image (default none).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_margin(mut self, margin: Pt) -> Self {
        self.margin = margin;
        self
    }

    /// The resolution of an image whose file gives none, for [`ImagePages::ImageSize`] (default
    /// 72).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_default_dpi(mut self, dpi: f64) -> Self {
        self.default_dpi = dpi;
        self
    }

    /// The options of the document built (version, metadata, tagging).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_build(mut self, build: BuildOptions) -> Self {
        self.build = build;
        self
    }

    /// How each page is sized.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn pages(&self) -> ImagePages {
        self.pages
    }
}

/// A new PDF with one page per image, in order (E3 plan §2.3). A JPEG is embedded byte for
/// byte (`DCTDecode`), a PNG decoded and Flate-compressed with its alpha as an `/SMask`. Each
/// image is drawn the way up its EXIF `Orientation` says ([`Image::orientation`]), so a photo
/// taken sideways reads upright, and pages are sized as [`ImagesOptions::with_pages`] says.
///
/// ```
/// use laminapdf_build::{Image, ImagesOptions, images_to_pdf};
///
/// let red = Image::rgba(2, 1, &[255, 0, 0, 255, 255, 0, 0, 255])?;
/// let doc = images_to_pdf(&[red], &ImagesOptions::new())?;
/// assert_eq!(doc.page_count()?, 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
/// [`Error::InvalidGeometry`] for no images, a negative or non-finite margin, a default
/// resolution that isn't positive, or a margin that leaves no room on a [`ImagePages::Fit`]
/// page; what [`Builder::finish`] returns.
///
/// Added in 0.2.0.
pub fn images_to_pdf(images: &[Image], options: &ImagesOptions) -> Result<laminapdf_ops::Document> {
    let invalid = |what| Err(Error::InvalidGeometry { what });
    if images.is_empty() {
        return invalid("images_to_pdf needs at least one image");
    }
    let margin = options.margin.get();
    if !margin.is_finite() || margin < 0.0 {
        return invalid("the margin must be a finite number of points, zero or more");
    }
    if !options.default_dpi.is_finite() || options.default_dpi <= 0.0 {
        return invalid("the default resolution must be a positive number of dots per inch");
    }
    let mut builder = Builder::new(options.build.clone());
    for image in images {
        let placed = place(image, options)?;
        let mut page = builder.page(placed.page);
        page.surface().save().transform(placed.matrix).image(
            image,
            RectPt::new(
                Pt::new(0.0),
                Pt::new(0.0),
                Pt::new(f64::from(image.width())),
                Pt::new(f64::from(image.height())),
            ),
        );
        page.surface().restore();
        page.finish()?;
    }
    builder.finish()
}

/// Where one image goes: its page, and the matrix that takes the stored image, in pixels with
/// y down, to its place on that page.
struct Placed {
    page: PageSize,
    matrix: Matrix,
}

fn place(image: &Image, options: &ImagesOptions) -> Result<Placed> {
    let orientation = image.orientation();
    let (width, height) = (f64::from(image.width()), f64::from(image.height()));
    // The image as shown: its sides swap for a quarter turn, and so do its resolutions.
    let (shown_w, shown_h) = if orientation.swaps_sides() {
        (height, width)
    } else {
        (width, height)
    };
    let margin = options.margin.get();
    let (page, scale_x, scale_y) = match options.pages {
        ImagePages::ImageSize => {
            let dpm = image.data.shown.dots_per_metre;
            let dpi = |pick: fn((u32, u32)) -> u32| dpm.map_or(options.default_dpi, |d| f64::from(pick(d)) * 0.0254);
            let (dpi_x, dpi_y) = (dpi(|d| d.0), dpi(|d| d.1));
            let (dpi_x, dpi_y) = if orientation.swaps_sides() {
                (dpi_y, dpi_x)
            } else {
                (dpi_x, dpi_y)
            };
            let (sx, sy) = (72.0 / dpi_x, 72.0 / dpi_y);
            let size = PageSize::new(
                Pt::new(shown_w.mul_add(sx, 2.0 * margin)),
                Pt::new(shown_h.mul_add(sy, 2.0 * margin)),
            );
            (size, sx, sy)
        }
        ImagePages::Fit(paper) => {
            let paper = if (shown_w > shown_h) == (paper.width.get() > paper.height.get()) {
                paper
            } else {
                paper.landscape()
            };
            let (room_w, room_h) = (
                2.0f64.mul_add(-margin, paper.width.get()),
                2.0f64.mul_add(-margin, paper.height.get()),
            );
            if room_w <= 0.0 || room_h <= 0.0 {
                return Err(Error::InvalidGeometry {
                    what: "the margins leave no room for the image on the page",
                });
            }
            let scale = (room_w / shown_w).min(room_h / shown_h);
            (paper, scale, scale)
        }
    };
    // Centred inside the margins (exactly the margin for ImageSize pages).
    let offset_x = (page.width.get() - shown_w * scale_x) / 2.0;
    let offset_y = (page.height.get() - shown_h * scale_y) / 2.0;
    // Turned upright first, then scaled and moved into place.
    let turn = Matrix::new(orientation.matrix(width, height));
    let fit = Matrix::new([scale_x, 0.0, 0.0, scale_y, offset_x, offset_y]);
    Ok(Placed {
        page,
        matrix: turn.then(&fit),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel() -> Image {
        Image::rgba(4, 2, &[0; 32]).unwrap()
    }

    #[test]
    fn an_image_without_a_resolution_is_one_point_per_pixel_plus_the_margins() {
        let options = ImagesOptions::new().with_margin(Pt::new(10.0));
        let placed = place(&pixel(), &options).unwrap();
        assert_eq!((placed.page.width.get(), placed.page.height.get()), (24.0, 22.0));
        assert_eq!(placed.matrix.to_array(), [1.0, 0.0, 0.0, 1.0, 10.0, 10.0]);
    }

    #[test]
    fn a_fitted_image_is_scaled_into_the_margins_on_a_page_turned_its_way() {
        let options = ImagesOptions::new()
            .with_pages(ImagePages::Fit(PageSize::LETTER))
            .with_margin(Pt::new(36.0));
        let placed = place(&pixel(), &options).unwrap();
        // 4 × 2 is landscape, so Letter turns: 792 × 612, 720 × 540 inside the margins.
        assert_eq!((placed.page.width.get(), placed.page.height.get()), (792.0, 612.0));
        let [a, _, _, d, e, f] = placed.matrix.to_array();
        assert_eq!((a, d), (180.0, 180.0));
        assert_eq!((e, f), (36.0, 126.0));
    }

    #[test]
    fn bad_options_and_no_images_are_invalid_geometry() {
        let cases = [
            images_to_pdf(&[], &ImagesOptions::new()),
            images_to_pdf(&[pixel()], &ImagesOptions::new().with_margin(Pt::new(-1.0))),
            images_to_pdf(&[pixel()], &ImagesOptions::new().with_default_dpi(0.0)),
            images_to_pdf(
                &[pixel()],
                &ImagesOptions::new()
                    .with_pages(ImagePages::Fit(PageSize::A6))
                    .with_margin(Pt::new(400.0)),
            ),
        ];
        for result in cases {
            assert_eq!(result.unwrap_err().code(), "invalid-geometry");
        }
    }
}
