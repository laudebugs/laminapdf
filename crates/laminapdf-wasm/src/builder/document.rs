//! The document builder: options, pages, navigation, metadata and attachments.

use std::cell::RefCell;
use std::rc::Rc;

use laminapdf::{Builder, PageSize, Pt, Rotation, Surface};
use wasm_bindgen::prelude::*;

use super::page::WasmPageBuilder;
use crate::build_dto::{
    AttachmentInit, BuildOptionsInit, FitInit, MetadataInit, OutlineEntryInit, PageLabelInit, ViewerPreferencesInit,
};
use crate::convert::{from_js, from_js_or_default, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, builder_finished, lamina_error};

/// The builder, shared with the pages being drawn; `None` once finished.
pub(super) type SharedBuilder = Rc<RefCell<Option<Builder>>>;

/// Builds a new document: pages, navigation and metadata, then [`WasmBuilder::finish`]
/// gives a [`WasmDocument`] (unsaved; its save is a full rewrite).
#[wasm_bindgen]
pub struct WasmBuilder {
    pub(super) builder: SharedBuilder,
}

impl WasmBuilder {
    /// Runs `f` on the builder, unless it has finished (`builder-finished`).
    pub(super) fn with<T>(&self, f: impl FnOnce(&mut Builder) -> JsResult<T>) -> JsResult<T> {
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
