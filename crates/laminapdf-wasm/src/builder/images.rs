//! Images to a PDF, one page each (`laminapdf::images_to_pdf`).

use laminapdf::{ImagePages, ImagesOptions, PageSize, Pt};
use serde::Deserialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use super::image::WasmImage;
use crate::build_dto::BuildOptionsInit;
use crate::convert::from_js_or_default;
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// The images of [`images_to_pdf`], in page order. Images are JavaScript-owned objects, which a
/// record can't carry, so they are added one by one.
#[wasm_bindgen]
#[derive(Default)]
pub struct WasmImageList {
    images: Vec<laminapdf::Image>,
}

#[wasm_bindgen]
impl WasmImageList {
    /// An empty list.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `image` as the next page. The image stays usable: the list holds its own copy.
    pub fn push(&mut self, image: &WasmImage) {
        self.images.push(image.image.clone());
    }
}

/// How [`images_to_pdf`] sizes its pages.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImagesOptionsInit {
    /// Each page this size, `[width, height]` in points, turned the image's way, the image fitted
    /// inside the margins and centred. Without it, each page is its image's size at the image's
    /// resolution.
    #[tsify(optional)]
    pub fit: Option<[f64; 2]>,
    /// The space around each image, in points (0 by default).
    #[tsify(optional)]
    pub margin: Option<f64>,
    /// The resolution of an image whose file gives none, in dots per inch (72 by default).
    #[tsify(optional)]
    pub default_dpi: Option<f64>,
    /// The new document's version, file ID, producer, creation date and compression.
    #[tsify(optional)]
    pub build: Option<BuildOptionsInit>,
}

/// A new document with one page per image of `images`, in order: a JPEG embedded as it is, a PNG
/// with its transparency, each drawn the way up its EXIF orientation says.
///
/// # Errors
/// `invalid-geometry` for no images, a negative margin or resolution, or a page too small for its
/// margins; `unknown-option`, `invalid-argument` for bad options.
#[wasm_bindgen(js_name = imagesToPdf)]
pub fn images_to_pdf(
    images: &WasmImageList,
    #[wasm_bindgen(unchecked_param_type = "ImagesOptionsInit | undefined")] options: &JsValue,
) -> JsResult<WasmDocument> {
    let init: ImagesOptionsInit = from_js_or_default(options, "options")?;
    let mut options = ImagesOptions::new();
    if let Some([width, height]) = init.fit {
        options = options.with_pages(ImagePages::Fit(PageSize::new(Pt::new(width), Pt::new(height))));
    }
    if let Some(margin) = init.margin {
        options = options.with_margin(Pt::new(margin));
    }
    if let Some(dpi) = init.default_dpi {
        options = options.with_default_dpi(dpi);
    }
    if let Some(build) = &init.build {
        options = options.with_build(build.to_options()?);
    }
    let doc = laminapdf::images_to_pdf(&images.images, &options).map_err(|e| lamina_error(&e))?;
    Ok(WasmDocument::new(doc))
}
