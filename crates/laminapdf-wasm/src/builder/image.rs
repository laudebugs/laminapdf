//! Images, decoded or passed through once and shared by every use.

use laminapdf::Image;
use wasm_bindgen::prelude::*;

use crate::error::{JsResult, lamina_error};

/// An image to draw (decoded or passed through once, shared by every use).
#[wasm_bindgen]
pub struct WasmImage {
    pub(super) image: Image,
}

#[wasm_bindgen]
impl WasmImage {
    /// A JPEG, passed through (`DCTDecode`).
    ///
    /// # Errors
    /// `invalid-image`, `unsupported-image`.
    pub fn jpeg(bytes: Vec<u8>) -> JsResult<Self> {
        let image = Image::jpeg(bytes).map_err(|e| lamina_error(&e.into()))?;
        Ok(Self { image })
    }

    /// A PNG (alpha as an `/SMask`).
    ///
    /// # Errors
    /// `invalid-image`, `unsupported-image`, `image-too-large`.
    pub fn png(bytes: &[u8]) -> JsResult<Self> {
        let image = Image::png(bytes).map_err(|e| lamina_error(&e.into()))?;
        Ok(Self { image })
    }

    /// Raw 8-bit RGBA pixels, row by row.
    ///
    /// # Errors
    /// `invalid-image` when the length isn't `width × height × 4`.
    pub fn rgba(width: u32, height: u32, rgba: &[u8]) -> JsResult<Self> {
        let image = Image::rgba(width, height, rgba).map_err(|e| lamina_error(&e.into()))?;
        Ok(Self { image })
    }

    /// Width in pixels.
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.image.width()
    }

    /// Height in pixels.
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.image.height()
    }
}
