//! Pages across documents: source documents for `insertPagesFrom`, `replacePages` and
//! `mergeDocuments`, new documents made from pages, split from one or merged from several (every
//! module).

use std::num::NonZeroUsize;

use laminapdf::{MergeInput, MergeOptions, PageId, SplitBy};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, from_js_or_default, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, invalid_argument, lamina_error};

#[wasm_bindgen]
impl WasmDocument {
    /// Adds another PDF as a source document: `{ id, pages }`, its ID (64 hex digits, the
    /// SHA-256 of `bytes`) and its page IDs in order. An encrypted one opens with `password`,
    /// which no op ever holds.
    ///
    /// # Errors
    /// `wrong-password`, `password-required`, and the errors of a file that doesn't open.
    #[wasm_bindgen(js_name = addSource, unchecked_return_type = "{ id: string; pages: string[] }")]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "wasm-bindgen takes an optional string only by value"
    )]
    pub fn add_source(&mut self, bytes: Vec<u8>, password: Option<String>) -> JsResult<JsValue> {
        let mut doc = self.shared().borrow_mut();
        let id = doc
            .add_source(bytes, password.as_deref())
            .map_err(|e| lamina_error(&e))?;
        let pages = doc.source_pages(&id).map_err(|e| lamina_error(&e))?;
        Ok(to_js(&AddedSource {
            id: id.to_string(),
            pages: pages.iter().map(ToString::to_string).collect(),
        }))
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
        let ids = page_ids(&ids, "pages")?;
        let doc = self
            .shared()
            .borrow()
            .extract_pages(&ids)
            .map_err(|e| lamina_error(&e))?;
        Ok(Self::new(doc))
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// This document cut into new ones: `{ count: n }` parts of `n` pages, `{ ranges: [[id, …],
    /// …] }` one part per list of page IDs, or `{ bookmarks: level }` a part from each bookmark
    /// at that depth (1: the top level). Each part carries what its pages reach and the
    /// bookmarks that go to them.
    ///
    /// # Errors
    /// `invalid-argument` / `unknown-option` for a value that isn't one of those, `unknown-page`,
    /// `no-pages-left` for an empty list; reading errors.
    #[wasm_bindgen(unchecked_return_type = "WasmDocument[]")]
    pub fn split(
        &self,
        #[wasm_bindgen(unchecked_param_type = "{ count: number } | { ranges: string[][] } | { bookmarks: number }")]
        by: &JsValue,
    ) -> JsResult<js_sys::Array> {
        let by = match from_js::<SplitByJs>(by, "by")? {
            SplitByJs::Count(n) => SplitBy::Count(n),
            SplitByJs::Bookmarks(level) => SplitBy::Bookmarks { level },
            SplitByJs::Ranges(ranges) => {
                SplitBy::Ranges(ranges.iter().map(|r| page_ids(r, "by")).collect::<JsResult<Vec<_>>>()?)
            }
        };
        let parts = self.shared().borrow().split(&by).map_err(|e| lamina_error(&e))?;
        Ok(parts.into_iter().map(|doc| JsValue::from(Self::new(doc))).collect())
    }

    /// A new document of `inputs` (each `{ bytes, password?, title? }`) one after another,
    /// with their bookmarks, named destinations, form fields, layers and page labels; `options`
    /// as `mergeDocuments`' (`outline`, `fields`).
    ///
    /// # Errors
    /// `invalid-argument` / `unknown-option` for bad values, `wrong-password`,
    /// `password-required`, and the errors of a file that doesn't open.
    #[wasm_bindgen]
    pub fn merge(
        #[wasm_bindgen(unchecked_param_type = "{ bytes: Uint8Array; password?: string; title?: string }[]")]
        inputs: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "MergeOptions | undefined")] options: &JsValue,
    ) -> JsResult<WasmDocument> {
        let options: MergeOptions = from_js_or_default(options, "options")?;
        let inputs = merge_inputs(inputs)?;
        let doc = laminapdf::Document::merge(inputs, options).map_err(|e| lamina_error(&e))?;
        Ok(Self::new(doc))
    }
}

/// `split`'s `by`.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
enum SplitByJs {
    Count(NonZeroUsize),
    Ranges(Vec<Vec<String>>),
    Bookmarks(NonZeroUsize),
}

/// Page IDs parsed from strings, `argument` naming the value in an error.
fn page_ids(ids: &[String], argument: &'static str) -> JsResult<Vec<PageId>> {
    ids.iter()
        .map(|id| {
            id.parse::<PageId>()
                .map_err(|_| invalid_argument(argument, format!("not a page ID: {id}"), Vec::new()))
        })
        .collect()
}

/// `merge`'s inputs, read field by field so the bytes aren't copied through JSON.
fn merge_inputs(inputs: &JsValue) -> JsResult<Vec<MergeInput>> {
    let not_list = || {
        invalid_argument(
            "inputs",
            "not a list of { bytes, password?, title? }".to_owned(),
            Vec::new(),
        )
    };
    let list = inputs.dyn_ref::<js_sys::Array>().ok_or_else(not_list)?;
    list.iter()
        .map(|item| {
            let field = |name: &str| js_sys::Reflect::get(&item, &JsValue::from_str(name)).ok();
            let bytes = field("bytes")
                .and_then(|b| b.dyn_into::<js_sys::Uint8Array>().ok())
                .ok_or_else(not_list)?;
            let mut input = MergeInput::new(bytes.to_vec());
            if let Some(password) = field("password").and_then(|p| p.as_string()) {
                input = input.with_password(password);
            }
            if let Some(title) = field("title").and_then(|t| t.as_string()) {
                input = input.with_title(title);
            }
            Ok(input)
        })
        .collect()
}

/// `addSource`'s answer.
#[derive(serde::Serialize)]
struct AddedSource {
    id: String,
    pages: Vec<String>,
}
