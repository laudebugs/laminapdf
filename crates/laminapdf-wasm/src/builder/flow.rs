//! `WasmBuilder.flow` (ADR 0032 §7): a `laminapdf.document/1` document laid out as tagged pages
//! that flow, filled from data when it is a template, in the text module. The document and the
//! data cross as JSON, so integers stay integers; the fonts and images it names are WebAssembly
//! objects, which a record can't hold, so a [`WasmFlowResources`] collects them by name.

use laminapdf::flow::document::{Resources, fill_or_read};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use super::document::WasmBuilder;
use super::image::WasmImage;
use super::text::WasmFont;
use crate::convert::{from_js, from_js_or_default, to_js};
use crate::error::{JsResult, lamina_error};

/// The fonts and images a document names, by the names it gives them.
#[wasm_bindgen]
#[derive(Default)]
pub struct WasmFlowResources {
    fonts: Vec<(String, laminapdf::FontHandle)>,
    images: Vec<(String, laminapdf::Image)>,
}

#[wasm_bindgen]
impl WasmFlowResources {
    /// No fonts and no images.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// `font` for the styles that name `name`; the same name again replaces it.
    pub fn font(&mut self, name: &str, font: &WasmFont) {
        self.fonts.retain(|(n, _)| n != name);
        self.fonts.push((name.to_owned(), font.handle().clone()));
    }

    /// `image` for the figures that name `name`; the same name again replaces it.
    pub fn image(&mut self, name: &str, image: &WasmImage) {
        self.images.retain(|(n, _)| n != name);
        self.images.push((name.to_owned(), image.image.clone()));
    }
}

/// How `flow` reads its document.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FlowOptionsInit {
    /// The values a template's `$data`, `$each` and `$item` placeholders read; without it, the
    /// document is read as it is.
    #[tsify(optional, type = "unknown")]
    pub data: Option<serde_json::Value>,
}

/// What `flow` added.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlowReportJs {
    /// The pages it added, in order, by ID.
    pub pages: Vec<String>,
}

#[wasm_bindgen]
impl WasmBuilder {
    /// Lays `document` out as tagged pages that flow, after any pages already added, filled from
    /// `options.data` first when it is given. The document's profiles are the builder's to set
    /// (`profiles` in its options); its warnings are the finished document's.
    ///
    /// # Errors
    /// `builder-finished`; `unknown-option`, `invalid-argument`; `invalid-document` with the JSON
    /// Pointer at fault and the closest keys in `didYouMean`; `template-data-missing`,
    /// `template-data-type`, `template-too-large`; `unknown-font` and `unknown-image` for a name
    /// `resources` lacks; `missing-glyph`, `flow-content-too-large`.
    #[wasm_bindgen(unchecked_return_type = "FlowReportJs")]
    pub fn flow(
        &self,
        #[wasm_bindgen(unchecked_param_type = "unknown")] document: &JsValue,
        resources: &WasmFlowResources,
        #[wasm_bindgen(unchecked_param_type = "FlowOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<JsValue> {
        let init: FlowOptionsInit = from_js_or_default(options, "options")?;
        let json: serde_json::Value = from_js(document, "document")?;
        let document = fill_or_read(&json, init.data.as_ref()).map_err(|e| lamina_error(&laminapdf::Error::from(e)))?;
        let named = resources
            .fonts
            .iter()
            .fold(Resources::new(), |r, (name, font)| r.with_font(name.clone(), font));
        let named = resources
            .images
            .iter()
            .fold(named, |r, (name, image)| r.with_image(name.clone(), image.clone()));
        self.with(|b| {
            let report = b.document(&document, &named).map_err(|e| lamina_error(&e))?;
            Ok(to_js(&FlowReportJs {
                pages: report.pages.iter().map(ToString::to_string).collect(),
            }))
        })
    }
}
