//! [`WasmDocument`] and the handles that point into it: [`WasmPage`] (ADR 0028 F2),
//! [`WasmSaveJob`] (F10) and [`WasmWarningCursor`] (F8).
//!
//! The document is shared as `Rc<RefCell<lamina::Document>>` (ADR 0028 §2): wasm32 has no
//! threads, and no call re-enters JavaScript while it holds a borrow, so the `RefCell` never
//! sees two borrows at once.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use js_sys::{Object, Reflect, Uint8Array};
use lamina::{Budget, PageId, SaveJob, Step};
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, from_js_or_default, to_js};
use crate::dto::{
    DocumentInfoJs, EditedJs, OpenOptionsInit, PageInfoJs, ProgressJs, SaveOptionsInit, SaveReportJs, UndoOutcomeJs,
    phase_name,
};
use crate::error::{JsResult, invalid_argument, lamina_error};
use crate::source::WasmSparseSource;

/// The shared document behind every handle.
pub(crate) type Shared = Rc<RefCell<lamina::Document>>;

/// An open PDF: read it, edit it with ops (with undo), save it. The binding of
/// `lamina::Document`, method for method (F12).
#[wasm_bindgen]
pub struct WasmDocument {
    doc: Shared,
}

impl WasmDocument {
    pub(crate) fn new(doc: lamina::Document) -> Self {
        Self {
            doc: Rc::new(RefCell::new(doc)),
        }
    }
}

/// `{ bytes, report }`: a save's output (a `Uint8Array`, F7) and its report.
fn saved(saved: &lamina::Saved) -> JsResult<JsValue> {
    let out = Object::new();
    Reflect::set(&out, &"bytes".into(), &Uint8Array::from(saved.bytes.as_slice()))?;
    Reflect::set(&out, &"report".into(), &to_js(&SaveReportJs::from(&saved.report)))?;
    Ok(out.into())
}

#[wasm_bindgen]
impl WasmDocument {
    /// Opens a document over a sparse source whose bytes arrive later. Until the source holds
    /// what a call needs, the call throws `data-pending` with `data.ranges` (`[offset, length]`
    /// pairs): supply them and call again. Lazy reads on the document can do the same.
    ///
    /// # Errors
    /// As `openBytes`, and `data-pending`.
    #[wasm_bindgen(js_name = openSource)]
    pub fn open_source(
        source: &WasmSparseSource,
        #[wasm_bindgen(unchecked_param_type = "OpenOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<Self> {
        let options: OpenOptionsInit = from_js_or_default(options, "options")?;
        let shared: Arc<dyn lamina::ByteSource> = Arc::clone(&source.inner) as Arc<dyn lamina::ByteSource>;
        let doc = lamina::Document::open_source(shared, options.to_options()?).map_err(|e| lamina_error(&e))?;
        Ok(Self::new(doc))
    }

    /// Opens a document from its bytes.
    ///
    /// # Errors
    /// The §1 object: `not-pdf`, `password-required`, `wrong-password`, `limit-exceeded`, …;
    /// `invalid-argument` / `unknown-option` for bad options.
    #[wasm_bindgen(js_name = openBytes)]
    pub fn open_bytes(
        bytes: Vec<u8>,
        #[wasm_bindgen(unchecked_param_type = "OpenOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<Self> {
        let options: OpenOptionsInit = from_js_or_default(options, "options")?;
        let doc = lamina::Document::open_bytes(bytes, options.to_options()?).map_err(|e| lamina_error(&e))?;
        Ok(Self::new(doc))
    }

    /// The number of pages.
    ///
    /// # Errors
    /// Reading errors.
    #[wasm_bindgen(js_name = pageCount)]
    pub fn page_count(&self) -> JsResult<u32> {
        self.doc.borrow().page_count().map_err(|e| lamina_error(&e))
    }

    /// Page `index`'s facts.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors.
    #[wasm_bindgen(js_name = pageInfo, unchecked_return_type = "PageInfoJs")]
    pub fn page_info(&self, index: u32) -> JsResult<JsValue> {
        let info = self.doc.borrow().page_info(index).map_err(|e| lamina_error(&e))?;
        Ok(to_js(&PageInfoJs::from(info)))
    }

    /// Every page's facts, in order (one page-tree walk).
    ///
    /// # Errors
    /// Reading errors.
    #[wasm_bindgen(js_name = pageInfos, unchecked_return_type = "PageInfoJs[]")]
    pub fn page_infos(&self) -> JsResult<JsValue> {
        let infos = self.doc.borrow().page_infos().map_err(|e| lamina_error(&e))?;
        let infos: Vec<PageInfoJs> = infos.into_iter().map(PageInfoJs::from).collect();
        Ok(to_js(&infos))
    }

    /// A handle on page `index` that follows the page by its ID (F2).
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors.
    pub fn page(&self, index: u32) -> JsResult<WasmPage> {
        let info = self.doc.borrow().page_info(index).map_err(|e| lamina_error(&e))?;
        Ok(WasmPage {
            doc: Rc::clone(&self.doc),
            id: info.id,
        })
    }

    /// A handle on the page with `id` (`p12.0`).
    ///
    /// # Errors
    /// `invalid-argument` for a malformed ID; `unknown-page`.
    #[wasm_bindgen(js_name = pageById)]
    pub fn page_by_id(&self, id: &str) -> JsResult<WasmPage> {
        let id: PageId = id
            .parse()
            .map_err(|_| invalid_argument("id", "not a page ID such as p12.0".to_owned(), Vec::new()))?;
        let info = self.doc.borrow().page_by_id(&id).map_err(|e| lamina_error(&e))?;
        Ok(WasmPage {
            doc: Rc::clone(&self.doc),
            id: info.id,
        })
    }

    /// The Info dictionary's text entries.
    ///
    /// # Errors
    /// Reading errors.
    #[wasm_bindgen(unchecked_return_type = "DocumentInfoJs")]
    pub fn info(&self) -> JsResult<JsValue> {
        let info = self.doc.borrow().info().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&DocumentInfoJs::from(info)))
    }

    /// A cursor over the warnings so far (build warnings first), each as the §1 object.
    pub fn warnings(&self) -> WasmWarningCursor {
        WasmWarningCursor {
            cursor: self.doc.borrow().warnings(),
        }
    }

    /// Whether there are changes no save has written yet.
    #[wasm_bindgen(js_name = hasUnsavedChanges)]
    pub fn has_unsaved_changes(&self) -> bool {
        self.doc.borrow().has_unsaved_changes()
    }

    /// Applies `ops` (the op JSON of ADR 0006 §6) in order, as one transaction.
    ///
    /// # Errors
    /// `invalid-argument` / `unknown-option` for a value that isn't an op; the first op's
    /// error (nothing is applied).
    #[wasm_bindgen(unchecked_return_type = "EditedJs")]
    pub fn apply(&mut self, #[wasm_bindgen(unchecked_param_type = "Op[]")] ops: &JsValue) -> JsResult<JsValue> {
        let ops: Vec<lamina::Op> = from_js(ops, "ops")?;
        let edited = self.doc.borrow_mut().apply(ops).map_err(|e| lamina_error(&e))?;
        Ok(to_js(&EditedJs::from(edited)))
    }

    /// Applies an op log (`{ "v": 1, "ops": [ … ] }`) as one transaction.
    ///
    /// # Errors
    /// `invalid-op-log`, `unsupported-version`; as [`WasmDocument::apply`].
    #[wasm_bindgen(js_name = applyLog, unchecked_return_type = "EditedJs")]
    pub fn apply_log(&mut self, json: &str) -> JsResult<JsValue> {
        let edited = self.doc.borrow_mut().apply_log(json).map_err(|e| lamina_error(&e))?;
        Ok(to_js(&EditedJs::from(edited)))
    }

    /// Undoes the newest transaction; `undefined` when there is nothing to undo.
    ///
    /// # Errors
    /// `nested-transaction`, `saved-not-rebased`; reading errors.
    #[wasm_bindgen(unchecked_return_type = "UndoOutcomeJs | undefined")]
    pub fn undo(&mut self) -> JsResult<JsValue> {
        let outcome = self.doc.borrow_mut().undo().map_err(|e| lamina_error(&e))?;
        Ok(outcome.map_or(JsValue::UNDEFINED, |o| to_js(&UndoOutcomeJs::from(&o))))
    }

    /// Redoes the newest undone transaction; `undefined` when there is nothing to redo.
    ///
    /// # Errors
    /// As [`WasmDocument::undo`].
    #[wasm_bindgen(unchecked_return_type = "UndoOutcomeJs | undefined")]
    pub fn redo(&mut self) -> JsResult<JsValue> {
        let outcome = self.doc.borrow_mut().redo().map_err(|e| lamina_error(&e))?;
        Ok(outcome.map_or(JsValue::UNDEFINED, |o| to_js(&UndoOutcomeJs::from(&o))))
    }

    /// Saves and rebases onto the output, running to completion: `{ bytes, report }`.
    ///
    /// # Errors
    /// The save's errors (`would-invalidate-signatures`, `requires-rewrite`, …).
    #[wasm_bindgen(unchecked_return_type = "{ bytes: Uint8Array; report: SaveReportJs }")]
    pub fn save(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "SaveOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<JsValue> {
        let options: SaveOptionsInit = from_js_or_default(options, "options")?;
        let out = self
            .doc
            .borrow_mut()
            .save(&options.to_options(), &lamina::CancelToken::new())
            .map_err(|e| lamina_error(&e))?;
        saved(&out)
    }

    /// A save as a job, stepped with a budget of work units (ADR 0028 §3, §4).
    ///
    /// # Errors
    /// `invalid-argument` / `unknown-option` for bad options.
    #[wasm_bindgen(js_name = saveJob)]
    pub fn save_job(
        &self,
        #[wasm_bindgen(unchecked_param_type = "SaveOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<WasmSaveJob> {
        let options: SaveOptionsInit = from_js_or_default(options, "options")?;
        let job = self.doc.borrow().save_job(&options.to_options());
        Ok(WasmSaveJob {
            doc: Rc::clone(&self.doc),
            job,
        })
    }

    /// Rebases onto `bytes`, the output of a save whose rebase failed (`not-rebased`).
    ///
    /// # Errors
    /// `no-pending-save`; reading errors in `bytes`.
    pub fn rebase(&mut self, bytes: Vec<u8>) -> JsResult<()> {
        self.doc.borrow_mut().rebase(bytes).map_err(|e| lamina_error(&e))
    }
}

/// A page of a [`WasmDocument`], held by its ID (F2): moving pages doesn't retarget it.
#[wasm_bindgen]
pub struct WasmPage {
    doc: Shared,
    id: PageId,
}

#[cfg(feature = "extract")]
#[wasm_bindgen]
impl WasmPage {
    /// The page's text: lines, runs, glyph quads and glyph text (B9c; the extract and text
    /// modules).
    ///
    /// # Errors
    /// `unknown-page` when the page is gone; `data-pending` and the reading errors;
    /// `unknown-option` / `invalid-argument` for bad options.
    #[wasm_bindgen(unchecked_return_type = "PageTextJs")]
    pub fn text(
        &self,
        #[wasm_bindgen(unchecked_param_type = "TextOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<JsValue> {
        let options: crate::text::TextOptionsInit = from_js_or_default(options, "options")?;
        let doc = self.doc.borrow();
        let info = doc.page_by_id(&self.id).map_err(|e| lamina_error(&e))?;
        let text = doc
            .page_text(info.index, &options.to_options())
            .map_err(|e| lamina_error(&e))?;
        crate::text::page_text_js(&text)
    }
}

#[wasm_bindgen]
impl WasmPage {
    /// The page's stable ID (`p12.0`).
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.to_string()
    }

    /// The page's current 0-based index.
    ///
    /// # Errors
    /// `unknown-page` when the page is gone; reading errors.
    pub fn index(&self) -> JsResult<u32> {
        let info = self.doc.borrow().page_by_id(&self.id).map_err(|e| lamina_error(&e))?;
        Ok(info.index)
    }

    /// The page's facts now.
    ///
    /// # Errors
    /// As [`WasmPage::index`].
    #[wasm_bindgen(unchecked_return_type = "PageInfoJs")]
    pub fn info(&self) -> JsResult<JsValue> {
        let info = self.doc.borrow().page_by_id(&self.id).map_err(|e| lamina_error(&e))?;
        Ok(to_js(&PageInfoJs::from(info)))
    }
}

/// A save in progress (F10). Each `step` checks that the document is unchanged since the
/// job was created (`document-changed`).
#[wasm_bindgen]
pub struct WasmSaveJob {
    doc: Shared,
    job: SaveJob,
}

#[wasm_bindgen]
impl WasmSaveJob {
    /// Runs at most `budget` work units: `{ state: "progress", done, total?, phase }` or
    /// `{ state: "done", bytes, report }`.
    ///
    /// # Errors
    /// `cancelled`, `document-changed`, `job-finished`, or the save's own errors.
    #[wasm_bindgen(
        unchecked_return_type = "({ state: \"progress\" } & ProgressJs) | { state: \"done\"; bytes: Uint8Array; report: SaveReportJs }"
    )]
    pub fn step(&mut self, budget: u32) -> JsResult<JsValue> {
        let step = self
            .job
            .step(&mut self.doc.borrow_mut(), Budget::units(budget))
            .map_err(|e| lamina_error(&e))?;
        let (state, out) = match step {
            Step::Done(out) => ("done", saved(&out)?),
            Step::Progress { done, total, phase } => (
                "progress",
                to_js(&ProgressJs {
                    done,
                    total,
                    phase: phase_name(phase),
                }),
            ),
            // A step kind added to the non-exhaustive enum: report no progress.
            _ => (
                "progress",
                to_js(&ProgressJs {
                    done: 0,
                    total: None,
                    phase: "other",
                }),
            ),
        };
        Reflect::set(&out, &"state".into(), &state.into())?;
        Ok(out)
    }

    /// Stops the job: its next step fails with `cancelled`.
    pub fn cancel(&self) {
        self.job.cancel_token().cancel();
    }
}

/// The warnings of a document, read in batches (F8).
#[wasm_bindgen]
pub struct WasmWarningCursor {
    cursor: lamina::WarningCursor,
}

#[wasm_bindgen]
impl WasmWarningCursor {
    /// Up to `max` more warnings, each as the ADR 0026 §1 object; empty at the end.
    #[wasm_bindgen(js_name = nextBatch, unchecked_return_type = "Diagnostic[]")]
    pub fn next_batch(&mut self, max: u32) -> JsValue {
        let batch: Vec<lamina::Diagnostic> = self
            .cursor
            .next_batch(max)
            .iter()
            .map(lamina::Warning::diagnostic)
            .collect();
        to_js(&batch)
    }

    /// How many warnings are left.
    pub fn remaining(&self) -> u32 {
        self.cursor.remaining()
    }
}
