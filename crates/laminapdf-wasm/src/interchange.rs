//! Comments to and from other apps across the boundary (the extract and text modules, as the
//! comment threads are): `WasmDocument.exportAnnotations(options)` writes XFDF, FDF or Markdown,
//! `previewImport(bytes, options)` says what an import would do, and `importAnnotations(bytes,
//! options)` applies it as one edit.

use laminapdf::{
    CommentFormat, ExportComments, ExportOptions, ImportMode, ImportOptions, ImportSummary, ImportWarning,
    MarkdownOptions,
};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, from_js_or_default, to_js};
use crate::document::WasmDocument;
use crate::dto::EditedJs;
use crate::error::{JsResult, lamina_error};

/// The format of an export.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CommentFormatInit {
    /// XFDF (ISO 19444-1).
    Xfdf,
    /// FDF.
    Fdf,
    /// Markdown, for notes apps.
    Markdown,
}

/// Which comments an export writes.
#[derive(Debug, Clone, Copy, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ExportCommentsInit {
    /// Every comment.
    #[default]
    All,
    /// Text markup and notes, with their replies.
    HighlightsAndNotes,
}

/// What Markdown includes; each is on when left out.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MarkdownInit {
    /// A heading for each page.
    #[tsify(optional)]
    pub headings: Option<bool>,
    /// Who wrote each comment, and when.
    #[tsify(optional)]
    pub authors: Option<bool>,
    /// The replies.
    #[tsify(optional)]
    pub replies: Option<bool>,
}

/// How to export comments.
#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExportOptionsInit {
    /// The format.
    pub format: CommentFormatInit,
    /// The pages, 0-based; every page when left out.
    #[tsify(optional)]
    pub pages: Option<Vec<u32>>,
    /// Which comments.
    #[serde(default)]
    #[tsify(optional)]
    pub comments: ExportCommentsInit,
    /// The document's file name, which XFDF and FDF record.
    #[tsify(optional)]
    pub file: Option<String>,
    /// What Markdown includes.
    #[serde(default)]
    #[tsify(optional)]
    pub markdown: MarkdownInit,
}

/// What an import does with the comments already in the document.
#[derive(Debug, Clone, Copy, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ImportModeInit {
    /// Keep them, update the ones the file names by `/NM`, add the rest.
    #[default]
    Merge,
    /// Delete the comments on the pages the file names first.
    Replace,
    /// Add everything as copies.
    Add,
}

/// How to import comments.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportOptionsInit {
    /// What to do with the comments already there; `"merge"` when left out.
    #[serde(default)]
    #[tsify(optional)]
    pub mode: ImportModeInit,
    /// The document's file name, checked against the one the file records.
    #[tsify(optional)]
    pub file: Option<String>,
}

impl From<ImportOptionsInit> for ImportOptions {
    fn from(init: ImportOptionsInit) -> Self {
        let mut options = Self::new(match init.mode {
            ImportModeInit::Merge => ImportMode::Merge,
            ImportModeInit::Replace => ImportMode::Replace,
            ImportModeInit::Add => ImportMode::Add,
        });
        options.file = init.file;
        options
    }
}

/// Something an import says: `interchange-skipped` or `interchange-document-mismatch`.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportWarningJs {
    /// The warning's code.
    pub code: &'static str,
    /// What is skipped: an annotation's subtype (`Sound`) or `field`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Why: `unsupported`, `form-field`, `missing-page`, `missing-parent` or `incomplete`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
    /// Its page, 0-based.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Its line in the exchange file, from 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// For a mismatch, the PDF the file names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// What an import does, or did.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportSummaryJs {
    /// The comments added, replies and statuses included.
    pub added: usize,
    /// The comments found by `/NM` and kept.
    pub matched: usize,
    /// The comments deleted first (`"replace"`).
    pub deleted: usize,
    /// The pages the added comments are on, 0-based.
    pub pages: Vec<u32>,
    /// What to say about it.
    pub warnings: Vec<ImportWarningJs>,
}

impl From<ImportSummary> for ImportSummaryJs {
    fn from(s: ImportSummary) -> Self {
        Self {
            added: s.added,
            matched: s.matched,
            deleted: s.deleted,
            pages: s.pages,
            warnings: s.warnings.into_iter().map(warning).collect(),
        }
    }
}

fn warning(w: ImportWarning) -> ImportWarningJs {
    let code = w.code();
    match w {
        ImportWarning::Skipped(s) => ImportWarningJs {
            code,
            kind: Some(s.kind),
            reason: Some(s.reason.as_str()),
            page: s.page,
            line: s.line,
            file: None,
        },
        ImportWarning::DocumentMismatch { file } => ImportWarningJs {
            code,
            kind: None,
            reason: None,
            page: None,
            line: None,
            file,
        },
        _ => ImportWarningJs {
            code,
            kind: None,
            reason: None,
            page: None,
            line: None,
            file: None,
        },
    }
}

/// An import's summary and the edit it made.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportedJs {
    /// What it did.
    pub summary: ImportSummaryJs,
    /// The edit, as `apply` reports one.
    pub edited: EditedJs,
}

#[wasm_bindgen]
impl WasmDocument {
    /// The document's comments as an XFDF, FDF or Markdown file; nothing changes.
    ///
    /// # Errors
    /// `invalid-argument` for options of the wrong shape; `page-index-out-of-range`; reading
    /// errors.
    #[wasm_bindgen(js_name = exportAnnotations)]
    pub fn export_annotations(
        &self,
        #[wasm_bindgen(unchecked_param_type = "ExportOptionsInit")] options: &JsValue,
    ) -> JsResult<Vec<u8>> {
        let init: ExportOptionsInit = from_js(options, "options")?;
        let mut options = ExportOptions::new(match init.format {
            CommentFormatInit::Xfdf => CommentFormat::Xfdf,
            CommentFormatInit::Fdf => CommentFormat::Fdf,
            CommentFormatInit::Markdown => CommentFormat::Markdown,
        });
        options.pages = init.pages;
        options.comments = match init.comments {
            ExportCommentsInit::All => ExportComments::All,
            ExportCommentsInit::HighlightsAndNotes => ExportComments::HighlightsAndNotes,
        };
        options.file = init.file;
        let mut markdown = MarkdownOptions::default();
        markdown.headings = init.markdown.headings.unwrap_or(true);
        markdown.authors = init.markdown.authors.unwrap_or(true);
        markdown.replies = init.markdown.replies.unwrap_or(true);
        options.markdown = markdown;
        self.shared()
            .borrow()
            .export_annotations(&options)
            .map_err(|e| lamina_error(&e))
    }

    /// What importing an XFDF or FDF file would do; nothing changes.
    ///
    /// # Errors
    /// `invalid-argument` for options of the wrong shape; `interchange-unknown-format`,
    /// `interchange-malformed`, `interchange-dtd`, `interchange-too-large`.
    #[wasm_bindgen(js_name = previewImport, unchecked_return_type = "ImportSummaryJs")]
    pub fn preview_import(
        &self,
        bytes: &[u8],
        #[wasm_bindgen(unchecked_param_type = "ImportOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<JsValue> {
        let options: ImportOptions = from_js_or_default::<ImportOptionsInit>(options, "options")?.into();
        let summary = self
            .shared()
            .borrow()
            .preview_import(bytes, &options)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&ImportSummaryJs::from(summary)))
    }

    /// Imports an XFDF or FDF file's comments as one edit, one undo step.
    ///
    /// # Errors
    /// As `previewImport`; the ops' errors, after which nothing has changed.
    #[wasm_bindgen(js_name = importAnnotations, unchecked_return_type = "ImportedJs")]
    pub fn import_annotations(
        &self,
        bytes: &[u8],
        #[wasm_bindgen(unchecked_param_type = "ImportOptionsInit | undefined")] options: &JsValue,
    ) -> JsResult<JsValue> {
        let options: ImportOptions = from_js_or_default::<ImportOptionsInit>(options, "options")?.into();
        let (summary, edited) = self
            .shared()
            .borrow_mut()
            .import_annotations(bytes, &options)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&ImportedJs {
            summary: summary.into(),
            edited: edited.into(),
        }))
    }
}
