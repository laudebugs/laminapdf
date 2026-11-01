//! The binding form of the builder (B5c left it to this crate, ADR 0028 F1–F3):
//! [`WasmBuilder`] owns a `laminapdf::Builder`; [`WasmPageBuilder`] is an *owned* page that
//! draws on its own `Surface` (paths as segment records instead of closures) and is written
//! into the builder by [`WasmPageBuilder::finish`]. The surface is the one
//! `laminapdf-build` uses, moved into the builder's page at the end, so the output is
//! byte-identical to the Rust API's.
//!
//! The text methods (fonts, `text`, `textBox`) exist only in the text module (feature
//! `text`, ADR 0016 §4).

use std::cell::RefCell;
use std::rc::Rc;

use laminapdf::{Builder, Image, PageSize, Pt, Rotation, Surface};
#[cfg(feature = "text")]
use serde::Serialize;
#[cfg(feature = "text")]
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::build_dto::{
    AttachmentInit, BLEND_MODES, BuildOptionsInit, FILL_RULES, FitInit, LINE_CAPS, LINE_JOINS, LinkTargetInit,
    MetadataInit, OutlineEntryInit, PageLabelInit, Path, ViewerPreferencesInit, artifact_kind, color, matrix, pick,
    rect, stroke_style, tag,
};
use crate::convert::{from_js, from_js_or_default, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, builder_finished, invalid_argument, lamina_error};

/// The builder, shared with the pages being drawn; `None` once finished.
type SharedBuilder = Rc<RefCell<Option<Builder>>>;

/// Builds a new document: pages, navigation and metadata, then [`WasmBuilder::finish`]
/// gives a [`WasmDocument`] (unsaved; its save is a full rewrite).
#[wasm_bindgen]
pub struct WasmBuilder {
    builder: SharedBuilder,
}

impl WasmBuilder {
    /// Runs `f` on the builder, unless it has finished (`builder-finished`).
    fn with<T>(&self, f: impl FnOnce(&mut Builder) -> JsResult<T>) -> JsResult<T> {
        let mut builder = self.builder.borrow_mut();
        f(builder.as_mut().ok_or_else(builder_finished)?)
    }
}

#[wasm_bindgen]
impl WasmBuilder {
    /// A builder for a new, empty document.
    ///
    /// # Errors
    /// `invalid-argument` / `unknown-option` for bad options.
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "BuildOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<Self> {
        let options: BuildOptionsInit = from_js_or_default(options, "options")?;
        Ok(Self {
            builder: Rc::new(RefCell::new(Some(Builder::new(options.to_options()?)))),
        })
    }

    /// A page of `width` × `height` points, written by [`WasmPageBuilder::finish`].
    pub fn page(&self, width: f64, height: f64) -> WasmPageBuilder {
        WasmPageBuilder {
            builder: Rc::clone(&self.builder),
            size: PageSize::new(Pt::new(width), Pt::new(height)),
            rotate: Rotation::None,
            user_unit: 1.0,
            boxes: [None; 4],
            stroke: laminapdf::StrokeStyle::default(),
            surface: Some(Surface::default()),
        }
    }

    /// Defines the named destination `name`: page `index` shown as `fit` (default: the page).
    ///
    /// # Errors
    /// `builder-finished`; `invalid-argument` for a bad `fit`.
    pub fn destination(
        &self,
        name: &str,
        index: u32,
        #[wasm_bindgen(unchecked_param_type = "FitInit | undefined")] fit: &JsValue,
    ) -> JsResult<()> {
        let fit = if fit.is_undefined() {
            laminapdf::Fit::Page
        } else {
            from_js::<FitInit>(fit, "fit")?.to_fit()
        };
        self.with(|b| {
            b.destination(name, index, fit);
            Ok(())
        })
    }

    /// Adds outline items (with their children); returns their IDs in pre-order.
    ///
    /// # Errors
    /// `builder-finished`; `invalid-argument`; `limit-exceeded`.
    #[wasm_bindgen(unchecked_return_type = "string[]")]
    pub fn outline(
        &self,
        #[wasm_bindgen(unchecked_param_type = "OutlineEntryInit[]")] entries: &JsValue,
    ) -> JsResult<JsValue> {
        let entries: Vec<OutlineEntryInit> = from_js(entries, "entries")?;
        self.with(|b| {
            let ids = b
                .outline(entries.iter().map(OutlineEntryInit::to_entry).collect())
                .map_err(|e| lamina_error(&e))?;
            let ids: Vec<String> = ids.iter().map(ToString::to_string).collect();
            Ok(to_js(&ids))
        })
    }

    /// The page labels, ranges by first page (checked at [`WasmBuilder::finish`]).
    ///
    /// # Errors
    /// `builder-finished`; `invalid-argument`.
    #[wasm_bindgen(js_name = pageLabels)]
    pub fn page_labels(
        &self,
        #[wasm_bindgen(unchecked_param_type = "PageLabelInit[]")] ranges: &JsValue,
    ) -> JsResult<()> {
        let ranges: Vec<PageLabelInit> = from_js(ranges, "ranges")?;
        self.with(|b| {
            b.page_labels(ranges.iter().map(PageLabelInit::to_label).collect());
            Ok(())
        })
    }

    /// The metadata, written as the Info dictionary and an XMP packet.
    ///
    /// # Errors
    /// `builder-finished`; `invalid-argument` (a date that isn't a PDF date).
    pub fn metadata(&self, #[wasm_bindgen(unchecked_param_type = "MetadataInit")] metadata: &JsValue) -> JsResult<()> {
        let metadata = from_js::<MetadataInit>(metadata, "metadata")?.to_metadata()?;
        self.with(|b| {
            b.metadata(metadata);
            Ok(())
        })
    }

    /// The natural language (`/Lang`), a BCP 47 tag.
    ///
    /// # Errors
    /// `builder-finished`.
    pub fn lang(&self, lang: &str) -> JsResult<()> {
        self.with(|b| {
            b.lang(lang);
            Ok(())
        })
    }

    /// Attaches a file (`/EmbeddedFiles`, the catalog's `/AF`): `name`, its bytes and its MIME
    /// type. The same name again replaces the file.
    ///
    /// # Errors
    /// `builder-finished`; `invalid-argument` (a date that isn't a PDF date); `unknown-option`.
    pub fn attach(
        &self,
        name: &str,
        bytes: Vec<u8>,
        mime: &str,
        #[wasm_bindgen(unchecked_param_type = "AttachmentInit | undefined")] options: &JsValue,
    ) -> JsResult<()> {
        let file = from_js_or_default::<AttachmentInit>(options, "options")?.to_attachment(name, bytes, mime)?;
        self.with(|b| {
            b.attach(file);
            Ok(())
        })
    }

    /// How viewers present the document when it opens.
    ///
    /// # Errors
    /// `builder-finished`; `invalid-argument`.
    #[wasm_bindgen(js_name = viewerPreferences)]
    pub fn viewer_preferences(
        &self,
        #[wasm_bindgen(unchecked_param_type = "ViewerPreferencesInit")] prefs: &JsValue,
    ) -> JsResult<()> {
        let prefs = from_js::<ViewerPreferencesInit>(prefs, "prefs")?.to_prefs();
        self.with(|b| {
            b.viewer_preferences(prefs);
            Ok(())
        })
    }

    /// Writes the fonts, links, page tree, outline and metadata, and returns the document.
    /// The builder is finished afterwards, and so are its unfinished pages.
    ///
    /// # Errors
    /// `builder-finished`; `page-index-out-of-range`, `unknown-destination`,
    /// `invalid-page-labels`, `invalid-geometry`, `subset-failed`.
    pub fn finish(&self) -> JsResult<WasmDocument> {
        let builder = self.builder.borrow_mut().take().ok_or_else(builder_finished)?;
        let doc = builder.finish().map_err(|e| lamina_error(&e))?;
        Ok(WasmDocument::new(doc))
    }
}

/// An image to draw (decoded or passed through once, shared by every use).
#[wasm_bindgen]
pub struct WasmImage {
    image: Image,
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

/// One page being drawn, in page space (points from the top-left, y down; ADR 0027).
/// Invalid geometry is reported by [`WasmPageBuilder::finish`] (`invalid-geometry`).
#[wasm_bindgen]
pub struct WasmPageBuilder {
    builder: SharedBuilder,
    size: PageSize,
    rotate: Rotation,
    user_unit: f64,
    /// Crop, bleed, trim and art boxes.
    boxes: [Option<laminapdf::RectPt>; 4],
    /// The style strokes use ([`WasmPageBuilder::stroke_style`]).
    stroke: laminapdf::StrokeStyle,
    /// What is drawn; `None` once [`WasmPageBuilder::finish`] moved it into the builder.
    surface: Option<Surface>,
}

/// The page's surface, unless the page has finished.
fn live(surface: &mut Option<Surface>) -> JsResult<&mut Surface> {
    surface.as_mut().ok_or_else(builder_finished)
}

#[wasm_bindgen]
impl WasmPageBuilder {
    /// `/Rotate`: 0, 90, 180 or 270 degrees.
    ///
    /// # Errors
    /// `invalid-argument` for another angle.
    pub fn rotate(&mut self, degrees: i32) -> JsResult<()> {
        self.rotate = Rotation::from_degrees(i64::from(degrees)).ok_or_else(|| {
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
        let m: [f64; 6] = m
            .try_into()
            .map_err(|_| invalid_argument("matrix", "a matrix has 6 numbers".to_owned(), Vec::new()))?;
        live(&mut self.surface)?.transform(matrix(m));
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

    /// Opens a structure element until the matching `endTag`: `kind` indexes the standard
    /// types (`Document`, `Part`, `Sect`, `H1`–`H6`, `P`, `L`, `LI`, `Lbl`, `LBody`, `Table`,
    /// `TR`, `TH`, `TD`, `Figure`, `Caption`, `Link`, `Span`, `Quote`, `Note`, `Code`), the role
    /// of `custom` when given; `/Alt`, `/ActualText`, `/Lang`, `/E`; `scope` 0 (none), 1 (row),
    /// 2 (column), 3 (both). `@laminapdf/sdk` turns names into these indices and checks them.
    ///
    /// # Errors
    /// `invalid-argument` for an index out of range.
    #[wasm_bindgen(js_name = beginTag)]
    #[expect(clippy::too_many_arguments, reason = "flat attributes (no record to convert)")]
    pub fn begin_tag(
        &mut self,
        kind: u8,
        custom: Option<String>,
        alt: Option<String>,
        #[wasm_bindgen(js_name = "actualText")] actual_text: Option<String>,
        lang: Option<String>,
        expansion: Option<String>,
        scope: u8,
    ) -> JsResult<()> {
        let tag = tag(kind, custom, [alt, actual_text, lang, expansion], scope)?;
        live(&mut self.surface)?.begin_tag(tag);
        Ok(())
    }

    /// Opens an artifact until the matching `endTag`: `kind` 0 (layout), 1 (page),
    /// 2 (pagination) or 3 (background).
    ///
    /// # Errors
    /// `invalid-argument` for another kind.
    #[wasm_bindgen(js_name = beginArtifact)]
    pub fn begin_artifact(&mut self, kind: u8) -> JsResult<()> {
        let kind = artifact_kind(kind)?;
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

/// A rectangle as returned to JavaScript.
#[cfg(feature = "text")]
#[derive(Debug, Serialize, Tsify)]
pub(crate) struct RectJs {
    /// Left.
    pub x: f64,
    /// Top.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

#[cfg(feature = "text")]
impl From<laminapdf::RectPt> for RectJs {
    fn from(r: laminapdf::RectPt) -> Self {
        Self {
            x: r.x0.get(),
            y: r.y0.get(),
            width: r.width().get(),
            height: r.height().get(),
        }
    }
}

#[cfg(feature = "text")]
pub use text::WasmFont;

/// The text module's part of the builder (ADR 0016 §4).
#[cfg(feature = "text")]
mod text {
    use laminapdf::{Alignment, Direction, Feature, Overflow, PointPt, Pt, Standard14, TextBoxOptions, TextRenderMode};
    use serde::{Deserialize, Serialize};
    use tsify::Tsify;
    use wasm_bindgen::prelude::*;

    use super::{RectJs, WasmBuilder, WasmPageBuilder, live};
    use crate::build_dto::{ColorInit, LinkTargetInit, rect};
    use crate::convert::{from_js_or_default, to_js};
    use crate::error::{JsResult, invalid_argument, lamina_error};

    /// A font registered with a [`WasmBuilder`].
    #[wasm_bindgen]
    pub struct WasmFont {
        font: laminapdf::FontHandle,
    }

    #[wasm_bindgen]
    impl WasmFont {
        /// The family name.
        #[wasm_bindgen(getter)]
        pub fn family(&self) -> String {
            self.font.info().family.clone()
        }

        /// The PostScript name.
        #[wasm_bindgen(getter, js_name = postscriptName)]
        pub fn postscript_name(&self) -> String {
            self.font.info().postscript_name.clone()
        }

        /// Whether it is one of the standard 14 fonts (not embedded).
        #[wasm_bindgen(getter, js_name = isStandard)]
        pub fn is_standard(&self) -> bool {
            self.font.is_standard()
        }
    }

    #[wasm_bindgen]
    impl WasmBuilder {
        /// Loads a font from its bytes (TrueType or OpenType/CFF) and registers it; it is
        /// subset to the glyphs drawn.
        ///
        /// # Errors
        /// `builder-finished`; `invalid-font`, `font-not-embeddable`.
        #[wasm_bindgen(js_name = fontBytes)]
        pub fn font_bytes(&self, bytes: Vec<u8>) -> JsResult<WasmFont> {
            self.with(|b| {
                let font = b.font_bytes(bytes).map_err(|e| lamina_error(&e))?;
                Ok(WasmFont { font })
            })
        }

        /// One of the standard 14 fonts, by its PostScript name (`Helvetica`,
        /// `Times-Roman`, …): not embedded, Latin only, no shaping.
        ///
        /// # Errors
        /// `builder-finished`; `invalid-argument` for another name.
        #[wasm_bindgen(js_name = standardFont)]
        pub fn standard_font(&self, name: &str) -> JsResult<WasmFont> {
            let font = Standard14::from_base_font(name).ok_or_else(|| {
                let known: Vec<String> = Standard14::ALL.iter().map(|f| f.base_font().to_owned()).collect();
                invalid_argument(
                    "name",
                    "not a standard 14 font name".to_owned(),
                    crate::error::did_you_mean(name, &known),
                )
            })?;
            self.with(|b| {
                let font = b.standard_font(font).map_err(|e| lamina_error(&e))?;
                Ok(WasmFont { font })
            })
        }
    }

    /// Text rendering (`Tr`).
    #[derive(Debug, Clone, Copy, Deserialize, Tsify)]
    #[serde(rename_all = "camelCase")]
    pub(super) enum RenderModeInit {
        /// Fill (the default).
        Fill,
        /// Stroke.
        Stroke,
        /// Fill, then stroke.
        FillStroke,
        /// Invisible (for an OCR layer).
        Invisible,
    }

    /// Paragraph direction.
    #[derive(Debug, Clone, Copy, Deserialize, Tsify)]
    #[serde(rename_all = "camelCase")]
    pub enum DirectionInit {
        /// Left to right.
        Ltr,
        /// Right to left.
        Rtl,
    }

    impl From<DirectionInit> for Direction {
        fn from(d: DirectionInit) -> Self {
            match d {
                DirectionInit::Ltr => Self::LeftToRight,
                DirectionInit::Rtl => Self::RightToLeft,
            }
        }
    }

    /// A link over drawn text.
    #[derive(Debug, Deserialize, Tsify)]
    #[serde(deny_unknown_fields)]
    pub(super) struct TextLinkInit {
        /// Where it goes.
        pub target: LinkTargetInit,
        /// Alternate text.
        #[tsify(optional)]
        pub contents: Option<String>,
    }

    /// How a run of text is drawn.
    #[derive(Debug, Default, Deserialize, Tsify)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub(super) struct TextOptionsInit {
        /// The fill colour.
        #[tsify(optional)]
        pub fill: Option<ColorInit>,
        /// The rendering mode.
        #[tsify(optional)]
        pub render_mode: Option<RenderModeInit>,
        /// A link over the run.
        #[tsify(optional)]
        pub link: Option<TextLinkInit>,
        /// OpenType features (`liga=0`, `smcp`).
        #[serde(default)]
        #[tsify(optional)]
        pub features: Vec<String>,
        /// The language (BCP 47).
        #[tsify(optional)]
        pub language: Option<String>,
        /// The paragraph direction (default: from the first strong character).
        #[tsify(optional)]
        pub direction: Option<DirectionInit>,
    }

    /// What a run measures.
    #[derive(Debug, Serialize, Tsify)]
    pub(super) struct TextRunJs {
        /// The advance width of its widest line.
        pub width: f64,
        /// Its box, page space.
        pub bounds: RectJs,
        /// Its lines (a run breaks only at line breaks in the text).
        pub lines: u32,
    }

    /// Alignment of a text box's lines.
    #[derive(Debug, Clone, Copy, Deserialize, Tsify)]
    #[serde(rename_all = "camelCase")]
    pub(super) enum AlignInit {
        /// The start of the paragraph's direction (the default).
        Start,
        /// The end.
        End,
        /// Left.
        Left,
        /// Right.
        Right,
        /// Centred.
        Center,
    }

    /// What happens to text that doesn't fit a text box.
    #[derive(Debug, Clone, Copy, Deserialize, Tsify)]
    #[serde(rename_all = "camelCase")]
    pub(super) enum OverflowInit {
        /// Drawn up to the last line that fits (the default).
        Clip,
        /// `text-overflow`.
        Error,
    }

    /// Options for `textBox`.
    #[derive(Debug, Default, Deserialize, Tsify)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub(super) struct TextBoxOptionsInit {
        /// Line alignment.
        #[tsify(optional)]
        pub align: Option<AlignInit>,
        /// Line height as a factor of the size (default 1.2).
        #[tsify(optional)]
        pub line_height: Option<f64>,
        /// Overflow handling.
        #[tsify(optional)]
        pub overflow: Option<OverflowInit>,
        /// Paragraph direction.
        #[tsify(optional)]
        pub direction: Option<DirectionInit>,
        /// The language (BCP 47).
        #[tsify(optional)]
        pub language: Option<String>,
        /// OpenType features.
        #[serde(default)]
        #[tsify(optional)]
        pub features: Vec<String>,
    }

    /// One drawn line of a text box. Offsets are UTF-16 code units into the text (JS
    /// string indices).
    #[derive(Debug, Serialize, Tsify)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct LineBoxJs {
        /// Where the line's text starts.
        pub start: u32,
        /// Where it ends (trailing spaces and the line break included).
        pub end: u32,
        /// The end of what is drawn (without trailing spaces and the break).
        pub content_end: u32,
        /// The line's box.
        pub rect: RectJs,
        /// Its baseline (y).
        pub baseline: f64,
    }

    /// How a text box laid its text out.
    #[derive(Debug, Serialize, Tsify)]
    pub(super) struct TextLayoutJs {
        /// The lines drawn.
        pub lines: Vec<LineBoxJs>,
        /// Where the text that didn't fit starts (UTF-16 offset), if any.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[tsify(optional)]
        pub overflow: Option<u32>,
    }

    /// UTF-8 byte offsets into `text` as UTF-16 offsets (JS string indices), in one pass over
    /// the text: offsets are looked up in increasing order. An offset inside a character
    /// counts the characters before it.
    fn utf16(text: &str, bytes: &[usize]) -> Vec<u32> {
        let mut order: Vec<usize> = (0..bytes.len()).collect();
        order.sort_by_key(|&i| bytes.get(i).copied().unwrap_or(0));
        let mut out = vec![0u32; bytes.len()];
        let mut chars = text.char_indices().peekable();
        let mut units = 0usize;
        for i in order {
            let target = bytes.get(i).copied().unwrap_or(0);
            while let Some(&(at, c)) = chars.peek() {
                if at >= target {
                    break;
                }
                units += c.len_utf16();
                chars.next();
            }
            if let Some(slot) = out.get_mut(i) {
                *slot = u32::try_from(units).unwrap_or(u32::MAX);
            }
        }
        out
    }

    fn features(names: &[String]) -> laminapdf::Result<Vec<Feature>> {
        names
            .iter()
            .map(|f| f.parse::<Feature>().map_err(laminapdf::Error::from))
            .collect()
    }

    #[wasm_bindgen]
    impl WasmPageBuilder {
        /// Draws `text` in `font` at `size` points with the baseline at `y` (ADR 0027 §5):
        /// shaped, bidi-resolved, one line per line break.
        ///
        /// # Errors
        /// `missing-glyph`, `font-not-shapable`, `invalid-feature`, `invalid-geometry`;
        /// `invalid-argument` for bad options.
        #[wasm_bindgen(unchecked_return_type = "TextRunJs")]
        pub fn text(
            &mut self,
            text: &str,
            font: &WasmFont,
            size: f64,
            x: f64,
            y: f64,
            #[wasm_bindgen(unchecked_param_type = "TextOptionsInit | undefined")] options: &JsValue,
        ) -> JsResult<JsValue> {
            // Every option is checked before anything is drawn.
            let options: TextOptionsInit = from_js_or_default(options, "options")?;
            let fill = options.fill.as_ref().map(ColorInit::to_color).transpose()?;
            let features = features(&options.features).map_err(|e| lamina_error(&e))?;
            let surface = live(&mut self.surface)?;
            let run = surface
                .text(text, &font.font, Pt::new(size), PointPt::from_points(x, y))
                .map_err(|e| lamina_error(&e.into()))?;
            let styled = (|| -> laminapdf::Result<TextRunJs> {
                let mut run = run;
                if !features.is_empty() {
                    run = run.features(&features)?;
                }
                if let Some(language) = &options.language {
                    run = run.language(language)?;
                }
                if let Some(direction) = options.direction {
                    run = run.direction(direction.into())?;
                }
                if let Some(color) = fill {
                    run = run.fill(color);
                }
                if let Some(mode) = options.render_mode {
                    run = run.render_mode(match mode {
                        RenderModeInit::Fill => TextRenderMode::Fill,
                        RenderModeInit::Stroke => TextRenderMode::Stroke,
                        RenderModeInit::FillStroke => TextRenderMode::FillStroke,
                        RenderModeInit::Invisible => TextRenderMode::Invisible,
                    });
                }
                if let Some(link) = &options.link {
                    run = run.link(link.target.to_target(), link.contents.clone());
                }
                Ok(TextRunJs {
                    width: run.width().get(),
                    bounds: run.bounds().into(),
                    lines: u32::try_from(run.lines()).unwrap_or(u32::MAX),
                })
            })();
            let run = styled.map_err(|e| {
                // Laying the run out again failed: draw nothing.
                surface.discard_last_text();
                lamina_error(&e)
            })?;
            Ok(to_js(&run))
        }

        /// Lays `text` out in `rect` (lines broken at Unicode line-break opportunities) and
        /// draws it.
        ///
        /// # Errors
        /// As [`WasmPageBuilder::text`]; `text-overflow` with `overflow: "error"`.
        #[wasm_bindgen(js_name = textBox, unchecked_return_type = "TextLayoutJs")]
        #[expect(clippy::too_many_arguments, reason = "the rectangle is flat (no record to convert)")]
        pub fn text_box(
            &mut self,
            text: &str,
            font: &WasmFont,
            size: f64,
            x: f64,
            y: f64,
            width: f64,
            height: f64,
            #[wasm_bindgen(unchecked_param_type = "TextBoxOptionsInit | undefined")] options: &JsValue,
        ) -> JsResult<JsValue> {
            let r = rect(x, y, width, height);
            let init: TextBoxOptionsInit = from_js_or_default(options, "options")?;
            let mut options = TextBoxOptions::default();
            if let Some(align) = init.align {
                options = options.with_align(match align {
                    AlignInit::Start => Alignment::Start,
                    AlignInit::End => Alignment::End,
                    AlignInit::Left => Alignment::Left,
                    AlignInit::Right => Alignment::Right,
                    AlignInit::Center => Alignment::Center,
                });
            }
            if let Some(factor) = init.line_height {
                options = options.with_line_height(factor);
            }
            if let Some(overflow) = init.overflow {
                options = options.with_overflow(match overflow {
                    OverflowInit::Clip => Overflow::Clip,
                    OverflowInit::Error => Overflow::Error,
                });
            }
            if let Some(direction) = init.direction {
                options = options.with_direction(direction.into());
            }
            if let Some(language) = init.language {
                options = options.with_language(language);
            }
            for feature in features(&init.features).map_err(|e| lamina_error(&e))? {
                options = options.with_feature(feature);
            }
            let layout = live(&mut self.surface)?
                .text_box(text, &font.font, Pt::new(size), r, &options)
                .map_err(|e| lamina_error(&e.into()))?;
            let mut offsets: Vec<usize> = layout
                .lines
                .iter()
                .flat_map(|l| [l.range.start, l.range.end, l.content.end])
                .collect();
            offsets.extend(layout.overflow);
            let units = utf16(text, &offsets);
            let unit = |i: usize| units.get(i).copied().unwrap_or(0);
            let lines = layout
                .lines
                .iter()
                .enumerate()
                .map(|(i, l)| LineBoxJs {
                    start: unit(3 * i),
                    end: unit(3 * i + 1),
                    content_end: unit(3 * i + 2),
                    rect: l.rect.into(),
                    baseline: l.baseline.get(),
                })
                .collect();
            Ok(to_js(&TextLayoutJs {
                lines,
                overflow: layout.overflow.map(|_| unit(3 * layout.lines.len())),
            }))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::utf16;

        #[test]
        fn utf16_converts_unsorted_offsets_in_one_pass() {
            // "aé𝄞b": a (1 byte, 1 unit), é (2, 1), 𝄞 (4, 2), b.
            assert_eq!(utf16("aé𝄞b", &[7, 0, 3, 1, 8]), [4, 0, 2, 1, 5]);
        }
    }
}
