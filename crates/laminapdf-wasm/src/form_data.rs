//! Form data across the boundary (the extract and text modules, as the form view is):
//! `WasmDocument.fillForm(json, options)` fills from Lamina PDF's form data JSON,
//! `importFormData(bytes, options)` from JSON, XFDF or FDF, `exportFormData(format, file)`
//! writes them, `resetFields(fields)` restores defaults, and `flattenForm(fields, intent)` draws
//! fields into their pages. Each is one edit.

use laminapdf::{FillOptions, FlattenIntent, FormDataFormat, FormDataReport};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, from_js_or_default, to_js};
use crate::document::WasmDocument;
use crate::dto::EditedJs;
use crate::error::{JsResult, lamina_error};
use crate::form_behaviour::field_id;

/// A form data file's format.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum FormDataFormatInit {
    /// Lamina PDF's JSON, the object `fillForm` takes.
    Json,
    /// XFDF (ISO 19444-1).
    Xfdf,
    /// FDF (ISO 32000-2 §12.7.8).
    Fdf,
}

/// How to fill a form.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FillOptionsInit {
    /// Whether the form's calculations run once after the last value; true when left out.
    #[tsify(optional)]
    pub calculate: Option<bool>,
}

impl From<FillOptionsInit> for FillOptions {
    fn from(init: FillOptionsInit) -> Self {
        Self::default().with_calculate(init.calculate.unwrap_or(true))
    }
}

/// A field a fill left as it was, and why.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RefusedFieldJs {
    /// The field's fully qualified name.
    pub name: String,
    /// Why: `field-is-signature`, `field-value-invalid` or `annotation-glyph-missing`.
    pub code: &'static str,
}

/// What filling, importing or resetting a form did.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FormDataReportJs {
    /// The edit, as `apply` reports one.
    pub edited: EditedJs,
    /// The names in the data that no field has.
    pub unknown: Vec<String>,
    /// The fields a signature's lock covers, by name, left as they are.
    pub locked: Vec<String>,
    /// The fields whose value was refused.
    pub refused: Vec<RefusedFieldJs>,
    /// Whether the form has an XFA part, whose answers weren't changed.
    pub xfa_not_updated: bool,
}

impl From<FormDataReport> for FormDataReportJs {
    fn from(r: FormDataReport) -> Self {
        Self {
            edited: r.edited.into(),
            unknown: r.unknown,
            locked: r.locked,
            refused: r
                .refused
                .into_iter()
                .map(|f| RefusedFieldJs {
                    name: f.name,
                    code: f.code,
                })
                .collect(),
            xfa_not_updated: r.xfa_not_updated,
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// Fills the form from form data JSON text (`{ "fields": { … } }`) as one edit.
    ///
    /// # Errors
    /// `invalid-argument` for options of the wrong shape; `interchange-malformed`;
    /// `appearance-unavailable`; reading errors.
    #[wasm_bindgen(js_name = fillForm, unchecked_return_type = "FormDataReportJs")]
    pub fn fill_form(
        &self,
        json: &str,
        #[wasm_bindgen(unchecked_param_type = "FillOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<JsValue> {
        let options: FillOptions = from_js_or_default::<FillOptionsInit>(options, "options")?.into();
        let report = self
            .shared()
            .borrow_mut()
            .fill_form(json, options)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&FormDataReportJs::from(report)))
    }

    /// Fills the form from a JSON, XFDF or FDF file as one edit.
    ///
    /// # Errors
    /// As `fillForm`; `interchange-unknown-format`, `interchange-dtd`, `interchange-too-large`.
    #[wasm_bindgen(js_name = importFormData, unchecked_return_type = "FormDataReportJs")]
    pub fn import_form_data(
        &self,
        bytes: &[u8],
        #[wasm_bindgen(unchecked_param_type = "FillOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<JsValue> {
        let options: FillOptions = from_js_or_default::<FillOptionsInit>(options, "options")?.into();
        let report = self
            .shared()
            .borrow_mut()
            .import_form_data(bytes, options)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&FormDataReportJs::from(report)))
    }

    /// The form's values as a JSON, XFDF or FDF file; nothing changes. XFDF and FDF name the
    /// PDF `file`.
    ///
    /// # Errors
    /// `invalid-argument` for a format it doesn't know; reading errors.
    #[wasm_bindgen(js_name = exportFormData)]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "wasm-bindgen takes an optional string only by value"
    )]
    pub fn export_form_data(
        &self,
        #[wasm_bindgen(unchecked_param_type = "FormDataFormatInit")] format: &JsValue,
        file: Option<String>,
    ) -> JsResult<Vec<u8>> {
        let format = match from_js::<FormDataFormatInit>(format, "format")? {
            FormDataFormatInit::Json => FormDataFormat::Json,
            FormDataFormatInit::Xfdf => FormDataFormat::Xfdf,
            FormDataFormatInit::Fdf => FormDataFormat::Fdf,
        };
        self.shared()
            .borrow()
            .export_form_data(format, file.as_deref())
            .map_err(|e| lamina_error(&e))
    }

    /// Gives `fields` (IDs, `f12.0`; every field but push buttons and signature fields when
    /// left out) their default values, as one edit.
    ///
    /// # Errors
    /// `invalid-argument` for an ID that doesn't parse; `unknown-field`; `field-is-signature`.
    #[wasm_bindgen(js_name = resetFields, unchecked_return_type = "FormDataReportJs")]
    pub fn reset_fields(&self, fields: Option<Vec<String>>) -> JsResult<JsValue> {
        let ids = fields
            .map(|ids| ids.iter().map(|id| field_id(id)).collect::<JsResult<Vec<_>>>())
            .transpose()?;
        let report = self
            .shared()
            .borrow_mut()
            .reset_fields(ids.as_deref())
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&FormDataReportJs::from(report)))
    }

    /// Draws `fields` (IDs, `f12.0`; every field but signature fields and those a lock covers
    /// when left out) into their pages for `intent` (`"view"` or `"print"`) and removes them, as
    /// one edit.
    ///
    /// # Errors
    /// `invalid-argument` for an ID that doesn't parse or an intent of the wrong shape;
    /// `unknown-field`; `field-is-signature`; `appearance-unavailable`.
    #[wasm_bindgen(js_name = flattenForm, unchecked_return_type = "EditedJs")]
    pub fn flatten_form(
        &self,
        fields: Option<Vec<String>>,
        #[wasm_bindgen(unchecked_param_type = "FlattenIntent")] intent: &JsValue,
    ) -> JsResult<JsValue> {
        let ids = fields
            .map(|ids| ids.iter().map(|id| field_id(id)).collect::<JsResult<Vec<_>>>())
            .transpose()?;
        let intent: FlattenIntent = from_js(intent, "intent")?;
        let edited = self
            .shared()
            .borrow_mut()
            .flatten_form(ids.as_deref(), intent)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&EditedJs::from(edited)))
    }
}
