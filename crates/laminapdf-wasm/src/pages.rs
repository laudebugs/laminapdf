//! Pages across documents: source documents for `insertPagesFrom` and `replacePages`, and new
//! documents made from pages (every module).

use laminapdf::{PageId, SourceId};
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, invalid_argument, lamina_error};

#[wasm_bindgen]
impl WasmDocument {
    /// Adds another PDF as a source document and returns its ID (64 hex digits, the SHA-256 of
    /// `bytes`); an encrypted one opens with `password`, which no op ever holds.
    ///
    /// # Errors
    /// `wrong-password`, `password-required`, and the errors of a file that doesn't open.
    #[wasm_bindgen(js_name = addSource)]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "wasm-bindgen takes an optional string only by value"
    )]
    pub fn add_source(&mut self, bytes: Vec<u8>, password: Option<String>) -> JsResult<String> {
        let password = password.as_deref();
        let id = self
            .shared()
            .borrow_mut()
            .add_source(bytes, password)
            .map_err(|e| lamina_error(&e))?;
        Ok(id.to_string())
    }

    /// The page IDs of the source document `source`, in order.
    ///
    /// # Errors
    /// `invalid-argument` for an ID that isn't 64 hex digits, `unknown-source`; reading errors.
    #[wasm_bindgen(js_name = sourcePages, unchecked_return_type = "string[]")]
    pub fn source_pages(&self, source: &str) -> JsResult<JsValue> {
        let id = parse_source(source)?;
        let pages = self.shared().borrow().source_pages(&id).map_err(|e| lamina_error(&e))?;
        Ok(to_js(&pages.iter().map(ToString::to_string).collect::<Vec<_>>()))
    }

    /// A new document of `pages` of this one, in the order given.
    ///
    /// # Errors
    /// `invalid-argument` for a value that isn't a list of page IDs, `unknown-page`; reading
    /// errors.
    #[wasm_bindgen(js_name = extractPages)]
    pub fn extract_pages(
        &self,
        #[wasm_bindgen(unchecked_param_type = "string[]")] pages: &JsValue,
    ) -> JsResult<WasmDocument> {
        let ids: Vec<String> = from_js(pages, "pages")?;
        let ids = ids
            .iter()
            .map(|id| {
                id.parse::<PageId>()
                    .map_err(|_| invalid_argument("pages", format!("not a page ID: {id}"), Vec::new()))
            })
            .collect::<JsResult<Vec<_>>>()?;
        let doc = self.shared().borrow().extract_pages(&ids).map_err(|e| lamina_error(&e))?;
        Ok(Self::new(doc))
    }
}

fn parse_source(source: &str) -> JsResult<SourceId> {
    source
        .parse()
        .map_err(|_| invalid_argument("source", "not a source ID: 64 hex digits".to_owned(), Vec::new()))
}
