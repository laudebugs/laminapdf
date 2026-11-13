//! wasm-bindgen bindings of `laminapdf-render`'s [`TiffWriter`]: the web reader's TIFF export.
//! The document worker draws each page itself and hands this module the pixels, one page at a
//! time, so the module holds only the TIFF it is writing, never a renderer. It is its own
//! module, loaded only when a TIFF export starts, so the reader's startup size doesn't change.
//!
//! Errors are thrown as the ADR 0026 §1 object, built by `RenderError::diagnostic`, the
//! serialiser every binding shares.

use laminapdf_render::{RenderError, TiffWriter};
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

/// A TIFF file being written, one page per [`TiffFile::add_page`].
#[wasm_bindgen]
pub struct TiffFile {
    writer: Option<TiffWriter>,
}

#[wasm_bindgen]
impl TiffFile {
    /// An empty file.
    ///
    /// # Errors
    /// `export-failed` if the encoder can't write the file's header.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let writer = TiffWriter::new().map_err(|e| throw(&e))?;
        Ok(Self { writer: Some(writer) })
    }

    /// Adds a page of `width` × `height` straight RGBA pixels, row by row, drawn at `dpi`.
    ///
    /// # Errors
    /// `invalid-argument` when `rgba` isn't `width × height × 4` bytes, `dpi` isn't
    /// positive, or the file is finished; `export-failed` if the encoder fails.
    #[wasm_bindgen(js_name = addPage)]
    pub fn add_page(&mut self, width: u32, height: u32, rgba: &[u8], dpi: f64) -> Result<(), JsValue> {
        let writer = self.writer.as_mut().ok_or_else(|| throw(&finished()))?;
        writer.add_page(width, height, rgba, dpi).map_err(|e| throw(&e))
    }

    /// The file's bytes. The file is finished: it takes no more pages.
    ///
    /// # Errors
    /// `invalid-argument` when it was already finished.
    pub fn finish(&mut self) -> Result<Vec<u8>, JsValue> {
        let writer = self.writer.take().ok_or_else(|| throw(&finished()))?;
        Ok(writer.finish())
    }
}

fn finished() -> RenderError {
    RenderError::InvalidRequest {
        what: "the TIFF file is already finished",
    }
}

/// The error as the ADR 0026 §1 object, a plain JS object; a JS `Error` with the message if
/// `JSON.parse` ever failed.
fn throw(e: &RenderError) -> JsValue {
    let diagnostic = e.diagnostic();
    js_sys::JSON::parse(&diagnostic.to_json().to_string())
        .unwrap_or_else(|_| js_sys::Error::new(&diagnostic.message).into())
}
