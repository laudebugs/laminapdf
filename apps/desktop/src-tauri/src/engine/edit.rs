//! A document's edits through its window's engine (E1h): the webview's edit session sends an
//! op log, an undo or a redo, and gets what changed back. The engine draws the pages from the
//! edited file from then on; the history (labels, the saved point) stays in the webview.

use laminapdf_desktop_protocol::{DocId, EditCall, EditReport, FromEngine, Reversal, ToEngine};
use serde::{Deserialize, Serialize};

use super::{Engines, PageMeta, ask, needs_password, unexpected};
use crate::DesktopError;

/// What the webview asks of a document's edits.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum EditRequest {
    /// Apply an op log (`{ "v": 1, "ops": [ … ] }`) as one transaction.
    Apply { log: String },
    /// Undo the newest transaction.
    Undo,
    /// Redo the newest undone one.
    Redo,
}

/// What an edit did, as the webview reads it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditOutcome {
    /// Whether the document changed.
    pub changed: bool,
    /// How an undo or redo went: `applied`, `partial` or `skipped`.
    pub reversal: Option<&'static str>,
    /// The IDs of the pages whose content or attributes changed.
    pub touched: Vec<String>,
    /// The page order an op log reports, when it changed the page list.
    pub page_order: Option<Vec<String>>,
    /// Every page after the edit, in page order.
    pub pages: Vec<PageMeta>,
    /// The bookmarks the op log added, by ID.
    pub created_outline_items: Vec<String>,
    /// The annotations the op log added, by ID.
    pub created_annotations: Vec<String>,
    /// The outline's links were rewritten: read it again.
    pub outline_repaired: bool,
}

impl From<EditReport> for EditOutcome {
    fn from(report: EditReport) -> Self {
        Self {
            changed: report.changed,
            reversal: report.reversal.map(|reversal| match reversal {
                Reversal::Applied => "applied",
                Reversal::Partial => "partial",
                Reversal::Skipped => "skipped",
            }),
            touched: report.touched,
            page_order: report.page_order,
            pages: report
                .pages
                .into_iter()
                .map(|page| PageMeta::new(page.id, &page.size))
                .collect(),
            created_outline_items: report.created_outline_items,
            created_annotations: report.created_annotations,
            outline_repaired: report.outline_repaired,
        }
    }
}

impl Engines {
    /// Applies, undoes or redoes an edit of `doc`, one of `window`'s documents.
    ///
    /// # Errors
    /// `edit-refused` with the op's error code (nothing changed); the engine's other failures.
    pub(crate) fn edit(&self, window: &str, doc: DocId, request: EditRequest) -> Result<EditOutcome, DesktopError> {
        self.check(window, doc)?;
        let call = match request {
            EditRequest::Apply { log } => EditCall::Apply { log },
            EditRequest::Undo => EditCall::Undo,
            EditRequest::Redo => EditCall::Redo,
        };
        match ask(&self.engine(window)?, |req| ToEngine::Edit { req, doc, call })? {
            FromEngine::Edited { report, .. } => Ok(EditOutcome::from(*report)),
            FromEngine::NeedsPassword { .. } => Err(needs_password()),
            other => Err(unexpected(&other)),
        }
    }
}
