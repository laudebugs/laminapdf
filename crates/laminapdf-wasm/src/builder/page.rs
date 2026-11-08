//! One page being drawn: graphics state, paths, images, tags and links.

use laminapdf::{Matrix, PageSize, Rotation, Surface};
use wasm_bindgen::prelude::*;

use super::document::SharedBuilder;
use super::image::WasmImage;
use crate::build_dto::{
    ARTIFACT_KINDS, BLEND_MODES, FILL_RULES, LINE_CAPS, LINE_JOINS, LinkTargetInit, Path, color, pick, rect,
    stroke_style, tag,
};
use crate::convert::from_js;
use crate::error::{JsResult, builder_finished, invalid_argument, lamina_error};

/// One page being drawn, in page space (points from the top-left, y down; ADR 0027).
/// Invalid geometry is reported by [`WasmPageBuilder::finish`] (`invalid-geometry`).
#[wasm_bindgen]
pub struct WasmPageBuilder {
    pub(super) builder: SharedBuilder,
    pub(super) size: PageSize,
    pub(super) rotate: Rotation,
    pub(super) user_unit: f64,
    /// Crop, bleed, trim and art boxes.
    pub(super) boxes: [Option<laminapdf::RectPt>; 4],
    /// The style strokes use ([`WasmPageBuilder::stroke_style`]).
    pub(super) stroke: laminapdf::StrokeStyle,
    /// What is drawn; `None` once [`WasmPageBuilder::finish`] moved it into the builder.
    pub(super) surface: Option<Surface>,
}

/// The page's surface, unless the page has finished.
pub(super) fn live(surface: &mut Option<Surface>) -> JsResult<&mut Surface> {
    surface.as_mut().ok_or_else(builder_finished)
}

#[wasm_bindgen]
impl WasmPageBuilder {
    /// `/Rotate`: 0, 90, 180 or 270 degrees.
    ///
    /// # Errors
    /// `invalid-argument` for another angle.
    pub fn rotate(&mut self, degrees: i32) -> JsResult<()> {
        let canonical = u16::try_from(degrees).ok().and_then(Rotation::from_canonical_degrees);
        self.rotate = canonical.ok_or_else(|| {
            invalid_argument(
                "degrees",
                "the rotation must be 0, 90, 180 or 270".to_owned(),
                Vec::new(),
            )
        })?;
        Ok(())
    }

    /// `/UserUnit`, for pages larger than 14,400 units.
    #[wasm_bindgen(js_name = userUnit)]
    pub fn user_unit(&mut self, unit: f64) {
        self.user_unit = unit;
    }

    /// `/CropBox`, `/BleedBox`, `/TrimBox` or `/ArtBox` (`kind`: `crop`, `bleed`, `trim`,
    /// `art`), in the media box's page space.
    ///
    /// # Errors
    /// `invalid-argument` for another kind.
    #[wasm_bindgen(js_name = setBox)]
    pub fn set_box(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "\"crop\" | \"bleed\" | \"trim\" | \"art\"")] kind: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> JsResult<()> {
        let slot = pick("kind", kind, &[("crop", 0), ("bleed", 1), ("trim", 2), ("art", 3)])?;
        if let Some(b) = self.boxes.get_mut(slot) {
            *b = Some(rect(x, y, width, height));
        }
        Ok(())
    }

    /// Saves the graphics state (`q`).
    ///
    /// # Errors
    /// `builder-finished` after [`WasmPageBuilder::finish`] (so does every drawing method).
    pub fn save(&mut self) -> JsResult<()> {
        live(&mut self.surface)?.save();
        Ok(())
    }

    /// Restores the graphics state (`Q`); one without a matching save is ignored.
    ///
    /// # Errors
    /// `builder-finished`.
    pub fn restore(&mut self) -> JsResult<()> {
        live(&mut self.surface)?.restore();
        Ok(())
    }

    /// Applies `[a, b, c, d, e, f]` (page space) to what is drawn next.
    ///
    /// # Errors
    /// `invalid-argument` unless there are 6 numbers.
    pub fn transform(&mut self, m: &[f64]) -> JsResult<()> {
        let m = Matrix::from_slice(m)
            .ok_or_else(|| invalid_argument("matrix", "a matrix has 6 numbers".to_owned(), Vec::new()))?;
        live(&mut self.surface)?.transform(m);
        Ok(())
    }

    /// The colour fills use: 1 (gray), 3 (RGB) or 4 (CMYK) components, 0 to 1.
    ///
    /// # Errors
    /// `invalid-argument` for another number of components.
    #[wasm_bindgen(js_name = fillColor)]
    pub fn fill_color(&mut self, components: &[f64]) -> JsResult<()> {
        live(&mut self.surface)?.fill_color(color(components)?);
        Ok(())
    }

    /// The colour strokes use (as [`WasmPageBuilder::fill_color`]).
    ///
    /// # Errors
    /// As [`WasmPageBuilder::fill_color`].
    #[wasm_bindgen(js_name = strokeColor)]
    pub fn stroke_color(&mut self, components: &[f64]) -> JsResult<()> {
        live(&mut self.surface)?.stroke_color(color(components)?);
        Ok(())
    }

    /// The fill colour as `#rrggbb` or `#rgb` (`DeviceRGB`).
    ///
    /// # Errors
    /// `invalid-color`.
    #[wasm_bindgen(js_name = fillColorHex)]
    pub fn fill_color_hex(&mut self, hex: &str) -> JsResult<()> {
        let c = laminapdf::Color::hex(hex).map_err(|e| lamina_error(&e.into()))?;
        live(&mut self.surface)?.fill_color(c);
        Ok(())
    }

    /// The stroke colour as `#rrggbb` or `#rgb`.
    ///
    /// # Errors
    /// `invalid-color`.
    #[wasm_bindgen(js_name = strokeColorHex)]
    pub fn stroke_color_hex(&mut self, hex: &str) -> JsResult<()> {
        let c = laminapdf::Color::hex(hex).map_err(|e| lamina_error(&e.into()))?;
        live(&mut self.surface)?.stroke_color(c);
        Ok(())
    }

    /// Fill and stroke opacity, 0 to 1 (an `ExtGState`).
    ///
    /// # Errors
    /// `builder-finished`.
    pub fn opacity(&mut self, fill: f64, stroke: f64) -> JsResult<()> {
        live(&mut self.surface)?.opacity(fill, stroke);
        Ok(())
    }

    /// The blend mode, camelCase (`multiply`, `colorDodge`, …).
    ///
    /// # Errors
    /// `invalid-argument` for an unknown mode.
    #[wasm_bindgen(js_name = blendMode)]
    pub fn blend_mode(&mut self, mode: &str) -> JsResult<()> {
        live(&mut self.surface)?.blend_mode(pick("mode", mode, &BLEND_MODES)?);
        Ok(())
    }

    /// The style later strokes use: `width` (points), `cap` (`butt`, `round`, `square`),
    /// `join` (`miter`, `round`, `bevel`), the miter limit and a dash pattern (empty: solid).
    ///
    /// # Errors
    /// `invalid-argument` for an unknown cap or join.
    #[wasm_bindgen(js_name = strokeStyle)]
    pub fn stroke_style(
        &mut self,
        width: f64,
        cap: &str,
        join: &str,
        #[wasm_bindgen(js_name = "miterLimit")] miter_limit: f64,
        dash: &[f64],
        #[wasm_bindgen(js_name = "dashPhase")] dash_phase: f64,
    ) -> JsResult<()> {
        let (cap, join) = (pick("cap", cap, &LINE_CAPS)?, pick("join", join, &LINE_JOINS)?);
        self.stroke = stroke_style(width, cap, join, miter_limit, dash, dash_phase);
        Ok(())
    }

    /// Fills `path` (the flat path encoding) with `rule` (`nonZero` or `evenOdd`).
    ///
    /// # Errors
    /// `invalid-argument` for a bad path or rule.
    #[wasm_bindgen(js_name = fillPath)]
    pub fn fill_path(&mut self, path: &[f64], rule: &str) -> JsResult<()> {
        let (path, rule) = (Path::decode(path)?, pick("rule", rule, &FILL_RULES)?);
        let _ = live(&mut self.surface)?.path(|p| path.build(p)).fill(rule);
        Ok(())
    }

    /// Strokes `path` with the current stroke style.
    ///
    /// # Errors
    /// `invalid-argument` for a bad path.
    #[wasm_bindgen(js_name = strokePath)]
    pub fn stroke_path(&mut self, path: &[f64]) -> JsResult<()> {
        let path = Path::decode(path)?;
        let _ = live(&mut self.surface)?.path(|p| path.build(p)).stroke(&self.stroke);
        Ok(())
    }

    /// Fills `path` with `rule`, then strokes it with the current stroke style.
    ///
    /// # Errors
    /// As [`WasmPageBuilder::fill_path`].
    #[wasm_bindgen(js_name = fillStrokePath)]
    pub fn fill_stroke_path(&mut self, path: &[f64], rule: &str) -> JsResult<()> {
        let (path, rule) = (Path::decode(path)?, pick("rule", rule, &FILL_RULES)?);
        let _ = live(&mut self.surface)?
            .path(|p| path.build(p))
            .fill_stroke(rule, &self.stroke);
        Ok(())
    }

    /// Clips to `path` (with `rule`) until the next `restore`.
    ///
    /// # Errors
    /// As [`WasmPageBuilder::fill_path`].
    #[wasm_bindgen(js_name = clipPath)]
    pub fn clip_path(&mut self, path: &[f64], rule: &str) -> JsResult<()> {
        let (path, rule) = (Path::decode(path)?, pick("rule", rule, &FILL_RULES)?);
        let _ = live(&mut self.surface)?.path(|p| path.build(p)).clip(rule);
        Ok(())
    }

    /// Draws `image` scaled into the rectangle.
    ///
    /// # Errors
    /// `builder-finished`.
    pub fn image(&mut self, image: &WasmImage, x: f64, y: f64, width: f64, height: f64) -> JsResult<()> {
        live(&mut self.surface)?.image(&image.image, rect(x, y, width, height));
        Ok(())
    }

    /// Opens a structure element until the matching `endTag`: `kind` names a standard type
    /// (`Document`, `Part`, `Sect`, `H1`–`H6`, `P`, `L`, `LI`, `Lbl`, `LBody`, `Table`, `TR`,
    /// `TH`, `TD`, `Figure`, `Caption`, `Link`, `Span`, `Quote`, `Note`, `Code`), the role of
    /// `custom` when given; `/Alt`, `/ActualText`, `/Lang`, `/E`; `scope` is `row`, `column` or
    /// `both` when given.
    ///
    /// # Errors
    /// `invalid-argument` for an unknown `kind` or `scope`.
    #[wasm_bindgen(js_name = beginTag)]
    #[expect(clippy::too_many_arguments, reason = "flat attributes (no record to convert)")]
    pub fn begin_tag(
        &mut self,
        kind: &str,
        custom: Option<String>,
        alt: Option<String>,
        #[wasm_bindgen(js_name = "actualText")] actual_text: Option<String>,
        lang: Option<String>,
        expansion: Option<String>,
        scope: Option<String>,
    ) -> JsResult<()> {
        let tag = tag(kind, custom, [alt, actual_text, lang, expansion], scope)?;
        live(&mut self.surface)?.begin_tag(tag);
        Ok(())
    }

    /// Opens an artifact until the matching `endTag`: `kind` is `layout`, `page`, `pagination`
    /// or `background`, or a pagination artifact that says what it is: `header` (attached to the
    /// top edge), `footer` (the bottom) or `watermark`.
    ///
    /// # Errors
    /// `invalid-argument` for another kind.
    #[wasm_bindgen(js_name = beginArtifact)]
    pub fn begin_artifact(&mut self, kind: &str) -> JsResult<()> {
        let kind = pick("kind", kind, &ARTIFACT_KINDS)?;
        live(&mut self.surface)?.begin_artifact(kind);
        Ok(())
    }

    /// Ends the innermost open element or artifact (none open: nothing happens).
    ///
    /// # Errors
    /// `builder-finished`.
    #[wasm_bindgen(js_name = endTag)]
    pub fn end_tag(&mut self) -> JsResult<()> {
        live(&mut self.surface)?.end_tag();
        Ok(())
    }

    /// A link annotation over the rectangle, with `contents` as its alternate text.
    ///
    /// # Errors
    /// `invalid-argument` / `unknown-option` for a bad target.
    pub fn link(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        #[wasm_bindgen(unchecked_param_type = "LinkTargetInit")] target: &JsValue,
        contents: Option<String>,
    ) -> JsResult<()> {
        let target = from_js::<LinkTargetInit>(target, "target")?.to_target();
        live(&mut self.surface)?.link(rect(x, y, width, height), target, contents);
        Ok(())
    }

    /// Writes the page into its builder: content stream, resources and page dictionary.
    /// Returns its page ID. The page is finished afterwards: every later call on it fails
    /// with `builder-finished`.
    ///
    /// # Errors
    /// `builder-finished` (the page or its builder already finished); `invalid-geometry` for
    /// the first invalid value drawn; `unknown-font` for a font from another builder.
    pub fn finish(&mut self) -> JsResult<String> {
        let surface = self.surface.take().ok_or_else(builder_finished)?;
        let mut shared = self.builder.borrow_mut();
        let builder = shared.as_mut().ok_or_else(builder_finished)?;
        let mut page = builder.page(self.size);
        page.rotate(self.rotate).user_unit(self.user_unit);
        let [crop, bleed, trim, art] = self.boxes;
        if let Some(r) = crop {
            page.crop_box(r);
        }
        if let Some(r) = bleed {
            page.bleed_box(r);
        }
        if let Some(r) = trim {
            page.trim_box(r);
        }
        if let Some(r) = art {
            page.art_box(r);
        }
        *page.surface() = surface;
        let id = page.finish().map_err(|e| lamina_error(&e.into()))?;
        Ok(id.to_string())
    }
}
