//! wasm-bindgen bindings of `lamina-render`'s hayro adapter ([ADR 0004] §5), wrapped by
//! `@lamina/render-hayro`, which gives it the same `RenderBackend` interface as
//! `@lamina/render-pdfium`.
//!
//! The surface is deliberately flat: numbers and byte arrays, no records, so the glue stays
//! tiny. Requests are `(page, scale, clip x/y/width/height, rotation, flags, background)`,
//! with a clip width of 0 meaning the whole page; flags are bits (1 annotations, 2 form
//! widgets, 4 print, 8 grayscale); the background is `0xRRGGBBAA` or `None` for transparent.
//! Errors are thrown as `Error` objects carrying the ADR 0026 `code` and `kind` (and `ranges`
//! for `data-pending`). A render is synchronous: the wrapper checks its `AbortSignal` before
//! and after.
//!
//! [ADR 0004]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0004-render-backend.md

use std::sync::Arc;

use js_sys::{Array, Reflect};
use lamina_render::{
    ByteSource, CancelToken, HayroBackend, PixelRect, RenderBackend, RenderDocument, RenderError, RenderFlags,
    RenderLimits, RenderOpenOptions, RenderRequest, Rgba8, ViewRotation,
};
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

/// A document open in hayro.
#[wasm_bindgen]
pub struct HayroDocument {
    doc: Box<dyn RenderDocument>,
}

#[wasm_bindgen]
impl HayroDocument {
    /// Opens `bytes` (moved into the module's memory; hayro borrows them, no second copy).
    ///
    /// # Errors
    /// The render errors of opening, thrown as `Error` objects with `code` and `kind`.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "wasm-bindgen passes JS strings as owned values"
    )]
    pub fn open(
        bytes: Vec<u8>,
        password: Option<String>,
        max_render_pixels: Option<f64>,
        max_render_side: Option<u32>,
    ) -> Result<HayroDocument, JsValue> {
        let mut limits = RenderLimits::default();
        if let Some(pixels) = max_render_pixels.filter(|p| p.is_finite() && *p >= 1.0) {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "checked finite and positive; saturates at u64::MAX"
            )]
            let pixels = pixels.min(u64::MAX as f64) as u64;
            limits.max_render_pixels = pixels;
        }
        if let Some(side) = max_render_side {
            limits.max_render_side = side;
        }
        let mut options = RenderOpenOptions::new().with_limits(limits);
        if let Some(password) = password.as_deref() {
            options = options.with_password(password);
        }
        let source: Arc<dyn ByteSource> = Arc::new(bytes);
        let doc = HayroBackend::new().open(source, &options).map_err(|e| throw(&e))?;
        Ok(Self { doc })
    }

    /// The number of pages.
    #[wasm_bindgen(js_name = pageCount)]
    pub fn page_count(&self) -> u32 {
        self.doc.page_count()
    }

    /// Page `index`'s `[width, height, rotate]` (displayed points, `/Rotate` in degrees).
    ///
    /// # Errors
    /// `page-index-out-of-range`.
    pub fn page(&self, index: u32) -> Result<Vec<f32>, JsValue> {
        let page = self.doc.page(index).map_err(|e| throw(&e))?;
        Ok(vec![page.width, page.height, f32::from(page.rotate)])
    }

    /// The request's geometry: `[pageWidth, pageHeight, x, y, width, height]` in pixels.
    ///
    /// # Errors
    /// `invalid-argument`, `render-too-large`, `page-index-out-of-range`.
    #[expect(clippy::too_many_arguments, reason = "a flat binding surface: numbers only")]
    pub fn plan(
        &self,
        page: u32,
        scale: f32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rotation: u16,
        flags: u32,
        background: Option<u32>,
    ) -> Result<Vec<u32>, JsValue> {
        let req = request(page, scale, x, y, width, height, rotation, flags, background)?;
        let plan = self.doc.plan(&req).map_err(|e| throw(&e))?;
        Ok(vec![
            plan.page_width,
            plan.page_height,
            plan.tile.x,
            plan.tile.y,
            plan.tile.width,
            plan.tile.height,
        ])
    }

    /// Renders the request into `out` (exactly `width × height × 4` bytes, straight RGBA).
    ///
    /// # Errors
    /// As [`HayroDocument::plan`], plus `render-failed` and `invalid-argument` for a wrong size.
    #[expect(clippy::too_many_arguments, reason = "a flat binding surface: numbers only")]
    pub fn render(
        &self,
        page: u32,
        scale: f32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rotation: u16,
        flags: u32,
        background: Option<u32>,
        out: &mut [u8],
    ) -> Result<(), JsValue> {
        let req = request(page, scale, x, y, width, height, rotation, flags, background)?;
        self.doc
            .render(&req, out, &CancelToken::new())
            .map(|_| ())
            .map_err(|e| throw(&e))
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::many_single_char_names,
    reason = "mirrors the flat binding surface; x/y are the tile's corner"
)]
fn request(
    page: u32,
    scale: f32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    rotation: u16,
    flags: u32,
    background: Option<u32>,
) -> Result<RenderRequest, JsValue> {
    let rotation = match rotation {
        0 => ViewRotation::None,
        90 => ViewRotation::Clockwise90,
        180 => ViewRotation::Half,
        270 => ViewRotation::Clockwise270,
        _ => {
            return Err(throw(&RenderError::InvalidRequest {
                what: "rotation must be 0, 90, 180 or 270",
            }));
        }
    };
    let mut req = RenderRequest::page(page, scale)
        .with_rotation(rotation)
        .with_flags(RenderFlags {
            annotations: flags & 1 != 0,
            form_widgets: flags & 2 != 0,
            print: flags & 4 != 0,
            grayscale: flags & 8 != 0,
        })
        .with_background(background.map(|c| {
            let [r, g, b, a] = c.to_be_bytes();
            Rgba8 { r, g, b, a }
        }));
    if width > 0 {
        req = req.with_clip(PixelRect { x, y, width, height });
    }
    Ok(req)
}

/// The error as a JS `Error` with the ADR 0026 `code`, `kind` and, for `data-pending`, `ranges`
/// (`[offset, length]` pairs).
fn throw(e: &RenderError) -> JsValue {
    let error = js_sys::Error::new(&e.to_string());
    let set = |key: &str, value: &JsValue| {
        // Setting a property on a fresh Error object can't fail.
        let _ = Reflect::set(&error, &JsValue::from_str(key), value);
    };
    set("code", &JsValue::from_str(e.code()));
    set("kind", &JsValue::from_str(e.kind().as_str()));
    if let RenderError::DataPending { ranges } = e {
        let list = Array::new();
        for &(offset, len) in ranges {
            #[expect(clippy::cast_precision_loss, reason = "offsets stay below 2^53 (JS numbers)")]
            list.push(&Array::of2(
                &JsValue::from_f64(offset as f64),
                &JsValue::from_f64(len as f64),
            ));
        }
        set("ranges", &list);
    }
    error.into()
}
