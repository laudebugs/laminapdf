//! The text module's part of the builder (ADR 0016 §4): fonts, `text` and `textBox`.

use laminapdf::{Alignment, Direction, Feature, Overflow, PointPt, Pt, Standard14, TextBoxOptions, TextRenderMode};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use super::document::WasmBuilder;
use super::page::{WasmPageBuilder, live};
use crate::build_dto::{ColorInit, LinkTargetInit, rect};
use crate::convert::{from_js_or_default, to_js};
use crate::error::{JsResult, invalid_argument, lamina_error};

/// A rectangle as returned to JavaScript.
#[derive(Debug, Serialize, Tsify)]
pub(super) struct RectJs {
    /// Left.
    pub x: f64,
    /// Top.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

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
