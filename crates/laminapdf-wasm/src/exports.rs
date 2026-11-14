//! A document's images, links and outline as files across the boundary:
//! `WasmDocument.extractImages(options)`, `linksJson()`, `linksCsv()`, `outlineJson()` and
//! `outlineCsv()` (the extract and text modules, so the base module stays in its size budget).
//! An image's bytes cross as a `Uint8Array`, not through JSON.

use js_sys::{Array, Reflect, Uint8Array};
use laminapdf::extract::{ExtractedImage, ImageFileFormat, ImageKind, ImageOptions, SkippedImage};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js_or_default, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// Which pages `extractImages` reads.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImageOptionsInit {
    /// The pages (0-based) in order; every page when absent.
    #[serde(default)]
    #[tsify(optional)]
    pub pages: Option<Vec<usize>>,
}

/// One extracted image, its bytes in `data` (a `Uint8Array` set beside these keys).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractedImageJs {
    /// The first page (0-based) that draws it.
    pub page: usize,
    /// Its object number; absent for an inline image.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub object: Option<u32>,
    pub kind: ImageKindJs,
    pub width: u32,
    pub height: u32,
    pub bits_per_component: u8,
    /// The colour space's family as `pdfimages -list` names it.
    pub color_space: String,
    /// Its filters, by their full names.
    pub filters: Vec<String>,
    pub format: ImageFormatJs,
    /// The file extension: `jpg`, `jp2` or `png`.
    pub extension: &'static str,
}

/// What an extracted image is: an image, a stencil (an image mask), or a JPEG's or JPEG 2000's
/// soft mask or stencil mask, which that file can't hold.
#[derive(Debug, Clone, Copy, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ImageKindJs {
    Image,
    Stencil,
    SoftMask,
    Mask,
}

/// The file an image is written as.
#[derive(Debug, Clone, Copy, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ImageFormatJs {
    Jpeg,
    Jpeg2000,
    Png,
}

/// An image that couldn't be extracted, and why.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkippedImageJs {
    pub page: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub object: Option<u32>,
    /// Why, in English; never document content.
    pub reason: String,
}

impl From<&ExtractedImage> for ExtractedImageJs {
    fn from(image: &ExtractedImage) -> Self {
        Self {
            page: image.page,
            object: image.object.map(|r| r.num),
            kind: match image.kind {
                ImageKind::Stencil => ImageKindJs::Stencil,
                ImageKind::SoftMask => ImageKindJs::SoftMask,
                ImageKind::Mask => ImageKindJs::Mask,
                _ => ImageKindJs::Image,
            },
            width: image.width,
            height: image.height,
            bits_per_component: image.bits_per_component,
            color_space: image.color_space.clone(),
            filters: image.filters.clone(),
            format: match image.format {
                ImageFileFormat::Jpeg => ImageFormatJs::Jpeg,
                ImageFileFormat::Jpeg2000 => ImageFormatJs::Jpeg2000,
                _ => ImageFormatJs::Png,
            },
            extension: image.format.extension(),
        }
    }
}

impl From<&SkippedImage> for SkippedImageJs {
    fn from(skipped: &SkippedImage) -> Self {
        Self {
            page: skipped.page,
            object: skipped.object.map(|r| r.num),
            reason: skipped.reason.clone(),
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The images the pages draw, as files (`{ images, skipped }`): JPEG and JPEG 2000 as
    /// stored, the rest as PNG, each with its bytes in `data` (the extract and text modules).
    ///
    /// # Errors
    /// `invalid-argument` for options that aren't any; `page-index-out-of-range`,
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(
        js_name = extractImages,
        unchecked_return_type = "{ images: (ExtractedImageJs & { data: Uint8Array<ArrayBuffer> })[]; skipped: SkippedImageJs[] }"
    )]
    pub fn extract_images(
        &self,
        #[wasm_bindgen(unchecked_param_type = "ImageOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<JsValue> {
        let init: ImageOptionsInit = from_js_or_default(options, "options")?;
        let mut options = ImageOptions::default();
        if let Some(pages) = init.pages {
            options = options.with_pages(pages);
        }
        let extracted = self
            .shared()
            .borrow()
            .extract_images(&options)
            .map_err(|e| lamina_error(&e))?;
        let images = Array::new();
        for image in &extracted.images {
            let object = to_js(&ExtractedImageJs::from(image));
            Reflect::set(&object, &"data".into(), &Uint8Array::from(image.data.as_slice()))?;
            images.push(&object);
        }
        let out = js_sys::Object::new();
        Reflect::set(&out, &"images".into(), &images)?;
        let skipped: Vec<SkippedImageJs> = extracted.skipped.iter().map(SkippedImageJs::from).collect();
        Reflect::set(&out, &"skipped".into(), &to_js(&skipped))?;
        Ok(out.into())
    }

    /// Every page's links as `laminapdf.links/1` JSON (the extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = linksJson)]
    pub fn links_json(&self) -> JsResult<String> {
        self.shared().borrow().links_json().map_err(|e| lamina_error(&e))
    }

    /// Every page's links as CSV, a row a link (the extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = linksCsv)]
    pub fn links_csv(&self) -> JsResult<String> {
        self.shared().borrow().links_csv().map_err(|e| lamina_error(&e))
    }

    /// The outline as `laminapdf.outline/1` JSON (the extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = outlineJson)]
    pub fn outline_json(&self) -> JsResult<String> {
        self.shared().borrow().outline_json().map_err(|e| lamina_error(&e))
    }

    /// The outline as CSV, a row an item with its level (the extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = outlineCsv)]
    pub fn outline_csv(&self) -> JsResult<String> {
        self.shared().borrow().outline_csv().map_err(|e| lamina_error(&e))
    }
}
