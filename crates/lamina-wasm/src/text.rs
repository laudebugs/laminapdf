//! Text extraction across the boundary (B9c): `WasmPage.text(options)` as a struct of arrays.
//! Runs cross as JSON records (small: one per font and line piece); every glyph's quad crosses
//! in one `Float32Array` and every glyph's text in one string with UTF-16 offsets, so a page of
//! thousands of glyphs costs three copies, not thousands of objects.

use js_sys::{Float32Array, Object, Reflect, Uint32Array};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::error::JsResult;

/// Options for `WasmPage.text`.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextOptionsInit {
    /// Leave out text drawn invisibly (rendering modes 3 and 7: OCR layers, hidden text).
    #[tsify(optional)]
    pub skip_invisible: Option<bool>,
}

impl TextOptionsInit {
    pub(crate) fn to_options(&self) -> lamina::extract::TextOptions {
        lamina::extract::TextOptions::new().with_skip_invisible(self.skip_invisible.unwrap_or(false))
    }
}

/// One text run (`lamina::extract::TextRun`): glyphs of one font, size, direction and marked
/// content on one baseline, in logical order.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct TextRunJs {
    /// The run's text in logical order (document text).
    pub text: String,
    /// The font's `/BaseFont` (document text).
    pub font: String,
    /// Font size in user-space units, the text matrix applied.
    pub size: f64,
    /// Vertical writing.
    pub vertical: bool,
    /// The marked-content ID the run is in, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub mcid: Option<f64>,
    /// The text comes from an `/ActualText` span.
    pub actual_text: bool,
    /// Drawn invisibly (`Tr` 3 or 7).
    pub invisible: bool,
    /// The bounds of the run's glyph quads, in page space (ADR 0027).
    pub bounds: [f64; 4],
    /// The run's glyphs: `[start, end)` into the page's glyph arrays.
    pub glyphs: [u32; 2],
}

fn index(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

/// `{ lines, runs, quads, glyphText, glyphOffsets, unmapped }` for a page's text.
pub(crate) fn page_text_js(text: &lamina::extract::PageText) -> JsResult<JsValue> {
    let runs: Vec<TextRunJs> = text
        .runs
        .iter()
        .map(|r| TextRunJs {
            text: r.text.clone(),
            font: r.font_name.to_string(),
            size: r.font_size,
            vertical: r.vertical,
            #[expect(clippy::cast_precision_loss, reason = "MCIDs are small integers")]
            mcid: r.mcid.map(|m| m as f64),
            actual_text: r.actual_text,
            invisible: r.invisible,
            bounds: [
                r.bounds.x0.get(),
                r.bounds.y0.get(),
                r.bounds.width().get(),
                r.bounds.height().get(),
            ],
            glyphs: [index(r.glyphs.start), index(r.glyphs.end)],
        })
        .collect();
    #[expect(clippy::cast_possible_truncation, reason = "Float32 is enough for points on a page")]
    let quads: Vec<f32> = text
        .glyphs
        .iter()
        .flat_map(|g| g.quad.iter().flat_map(|p| [p.x.get() as f32, p.y.get() as f32]))
        .collect();
    let mut glyph_text = String::new();
    let mut offsets: Vec<u32> = Vec::with_capacity(text.glyphs.len() + 1);
    let mut utf16 = 0_usize;
    offsets.push(0);
    for g in &text.glyphs {
        glyph_text.push_str(&g.text);
        utf16 += g.text.encode_utf16().count();
        offsets.push(index(utf16));
    }
    let out = Object::new();
    Reflect::set(&out, &"lines".into(), &crate::convert::to_js(&text.lines()))?;
    Reflect::set(&out, &"runs".into(), &crate::convert::to_js(&runs))?;
    Reflect::set(&out, &"quads".into(), &Float32Array::from(quads.as_slice()))?;
    Reflect::set(&out, &"glyphText".into(), &glyph_text.into())?;
    Reflect::set(&out, &"glyphOffsets".into(), &Uint32Array::from(offsets.as_slice()))?;
    Reflect::set(&out, &"unmapped".into(), &JsValue::from(index(text.unmapped)))?;
    Ok(out.into())
}

#[wasm_bindgen(typescript_custom_section)]
const PAGE_TEXT_TS: &str = r#"
/** A page's text as `WasmPage.text` returns it (B9c). */
export interface PageTextJs {
  /** The text by line, in reading order (runs on one baseline, joined; trimmed). */
  lines: string[];
  /** The runs, in content order. */
  runs: TextRunJs[];
  /** Each glyph's quad in page space: 8 numbers per glyph (x0, y0 … x3, y3). */
  quads: Float32Array;
  /** Every glyph's text, concatenated. */
  glyphText: string;
  /** UTF-16 offsets into glyphText: glyph i is glyphText.slice(glyphOffsets[i], glyphOffsets[i + 1]). */
  glyphOffsets: Uint32Array;
  /** Glyphs whose Unicode is unknown (U+FFFD). */
  unmapped: number;
}
"#;
