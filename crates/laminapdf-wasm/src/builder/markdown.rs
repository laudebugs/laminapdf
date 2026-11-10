//! `WasmBuilder.markdown` (ADR 0032 §7): Markdown laid out as a flow, in the text module. The
//! fonts and images are WebAssembly objects, which a record can't hold, so a
//! [`WasmMarkdownOptions`] collects them by role and target; the rest is a record.

use laminapdf::PageSize;
use laminapdf::flow::{Margins, PageRegion, PageTemplate, TextStyle};
use laminapdf::markdown::{MarkdownBuildOptions, MarkdownFonts};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use super::document::WasmBuilder;
use super::image::WasmImage;
use super::text::WasmFont;
use crate::convert::{from_js_or_default, to_js};
use crate::error::{JsResult, invalid_argument, lamina_error};

/// The fonts and images of a Markdown build, by role and by the target the Markdown names.
#[wasm_bindgen]
#[derive(Default)]
pub struct WasmMarkdownOptions {
    fonts: Vec<(String, laminapdf::FontHandle)>,
    images: Vec<(String, laminapdf::Image)>,
}

#[wasm_bindgen]
impl WasmMarkdownOptions {
    /// No fonts (the standard 14 Helvetica family and Courier) and no images.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// `font` for `role`: `regular`, `bold` (headings too), `italic`, `boldItalic`, `mono`, or
    /// `fallback` (each call adds one) for characters the others lack.
    ///
    /// # Errors
    /// `invalid-argument` for another role.
    pub fn font(&mut self, role: &str, font: &WasmFont) -> JsResult<()> {
        const ROLES: [&str; 6] = ["regular", "bold", "italic", "boldItalic", "mono", "fallback"];
        if !ROLES.contains(&role) {
            let known: Vec<String> = ROLES.iter().map(|r| (*r).to_owned()).collect();
            return Err(invalid_argument(
                "role",
                "not a font role".to_owned(),
                laminapdf::did_you_mean(role, &known),
            ));
        }
        self.fonts.retain(|(r, _)| r != role || role == "fallback");
        self.fonts.push((role.to_owned(), font.handle().clone()));
        Ok(())
    }

    /// `image` for the images whose target is `target`, as the Markdown writes it.
    pub fn image(&mut self, target: &str, image: &WasmImage) {
        self.images.retain(|(t, _)| t != target);
        self.images.push((target.to_owned(), image.image.clone()));
    }
}

/// A header or footer: text at the start, centre and end, with `{page}`, `{pages}`, `{section}`
/// and `{title}` filled in.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PageRegionInit {
    /// At the start of the line.
    #[tsify(optional)]
    pub start: Option<String>,
    /// Centred.
    #[tsify(optional)]
    pub center: Option<String>,
    /// At the end of the line.
    #[tsify(optional)]
    pub end: Option<String>,
}

impl PageRegionInit {
    fn to_region(&self) -> PageRegion {
        let mut region = PageRegion::new();
        if let Some(text) = &self.start {
            region = region.with_start(text.clone());
        }
        if let Some(text) = &self.center {
            region = region.with_center(text.clone());
        }
        if let Some(text) = &self.end {
            region = region.with_end(text.clone());
        }
        region
    }
}

/// The pages a Markdown document flows through.
#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PageTemplateInit {
    /// The page's width and height in points.
    pub size: [f64; 2],
    /// Top, right, bottom and left margins in points (72 each by default).
    #[tsify(optional)]
    pub margins: Option<[f64; 4]>,
    /// The header.
    #[tsify(optional)]
    pub header: Option<PageRegionInit>,
    /// The footer.
    #[tsify(optional)]
    pub footer: Option<PageRegionInit>,
    /// The size of the header and footer text, in the regular font (8 points by default).
    #[tsify(optional)]
    pub furniture_size: Option<f64>,
}

/// How `markdown` lays a document out.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MarkdownOptionsInit {
    /// The body text's size in points (10.5 by default); headings scale from it.
    #[tsify(optional)]
    pub size: Option<f64>,
    /// A contents page listing headings of levels 1 to `contents`, at the start.
    #[tsify(optional)]
    pub contents: Option<u8>,
    /// The pages (by default A4, 56-point margins, "Page {page} of {pages}" in the footer).
    #[tsify(optional)]
    pub page: Option<PageTemplateInit>,
}

/// What `markdown` added.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MarkdownReportJs {
    /// The pages it added, in order, by ID.
    pub pages: Vec<String>,
}

#[wasm_bindgen]
impl WasmBuilder {
    /// Lays `markdown` out as tagged pages that flow, after any pages already added; its front
    /// matter's title, language, author and subject become the document's. Its warnings
    /// (`markdown-html-ignored`, `markdown-image-missing`, the flow's) are the finished
    /// document's.
    ///
    /// # Errors
    /// `builder-finished`; `unknown-option`, `invalid-argument`; `missing-glyph`,
    /// `flow-content-too-large`.
    #[wasm_bindgen(unchecked_return_type = "MarkdownReportJs")]
    pub fn markdown(
        &self,
        markdown: &str,
        resources: &WasmMarkdownOptions,
        #[wasm_bindgen(unchecked_param_type = "MarkdownOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<JsValue> {
        let init: MarkdownOptionsInit = from_js_or_default(options, "options")?;
        let regular = resources
            .fonts
            .iter()
            .find(|(r, _)| r == "regular")
            .map(|(_, f)| f.clone());
        let mut options = MarkdownBuildOptions::new();
        if let Some(regular) = &regular {
            let mut fonts = MarkdownFonts::new(regular);
            for (role, font) in &resources.fonts {
                fonts = match role.as_str() {
                    "bold" => fonts.with_bold(font),
                    "italic" => fonts.with_italic(font),
                    "boldItalic" => fonts.with_bold_italic(font),
                    "mono" => fonts.with_mono(font),
                    "fallback" => fonts.with_fallback(font),
                    _ => fonts,
                };
            }
            options = options.with_fonts(fonts);
        } else if !resources.fonts.is_empty() {
            return Err(invalid_argument(
                "resources",
                "the other font roles need a regular font".to_owned(),
                Vec::new(),
            ));
        }
        if let Some(size) = init.size {
            options = options.with_size(size);
        }
        if let Some(levels) = init.contents {
            options = options.with_contents(levels);
        }
        for (target, image) in &resources.images {
            options = options.with_image(target.clone(), image.clone());
        }
        self.with(|b| {
            if let Some(page) = &init.page {
                options = options.clone().with_template(template(b, page, regular.as_ref())?);
            }
            let report = b.markdown(markdown, &options).map_err(|e| lamina_error(&e))?;
            Ok(to_js(&MarkdownReportJs {
                pages: report.pages.iter().map(ToString::to_string).collect(),
            }))
        })
    }
}

/// The page template `page` describes, its furniture in the regular font (Helvetica without one).
fn template(
    builder: &mut laminapdf::Builder,
    page: &PageTemplateInit,
    regular: Option<&laminapdf::FontHandle>,
) -> JsResult<PageTemplate> {
    let [width, height] = page.size;
    let mut template = PageTemplate::new(PageSize::new(laminapdf::Pt::new(width), laminapdf::Pt::new(height)));
    if let Some([top, right, bottom, left]) = page.margins {
        template = template.with_margins(Margins::new(top, right, bottom, left));
    }
    if page.header.is_some() || page.footer.is_some() {
        let font = match regular {
            Some(font) => font.clone(),
            None => builder
                .standard_font(laminapdf::Standard14::Helvetica)
                .map_err(|e| lamina_error(&e))?,
        };
        let style = TextStyle::new(&font, page.furniture_size.unwrap_or(8.0));
        let header = page.header.as_ref().map(PageRegionInit::to_region);
        let footer = page.footer.as_ref().map(PageRegionInit::to_region);
        template = template.with_furniture(header, footer, &style);
    }
    Ok(template)
}
