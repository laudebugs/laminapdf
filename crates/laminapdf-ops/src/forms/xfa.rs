//! [`RemoveXfa`] (E8 plan §2.4): the form's XFA part taken out, so every viewer shows the
//! `AcroForm` fields.

use laminapdf_model::set_catalog_entry;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::op::Effect;
use crate::pages::edit_form;

/// Removes the interactive form's `/XFA` entry (ISO 32000-2 §12.7.9) and the catalog's
/// `/NeedsRendering` (§7.7.2), so every viewer shows the `AcroForm` fields, which hold the answers
/// the pages show. Adobe's apps otherwise show the XFA part's own values: for a static form the
/// fields' answers wouldn't show there, and a dynamic form has no fields to show. Some static
/// forms keep their values only in the XFA `datasets` packet; their fields look empty
/// afterwards. The packets stay in the file's earlier revisions until a full rewrite
/// ([`Applied::still_in_earlier_revisions`](crate::Applied::still_in_earlier_revisions)), and
/// undo puts them back. It fails with `invalid-op` when the form has no `/XFA`.
///
/// Its class is `Document`: a certified file (any `DocMDP` level) and a file with usage rights
/// don't allow it, and the planner reports it before the op applies. The library never applies
/// it by itself; [`Document::xfa`](laminapdf_model::Document::xfa) says whether it is needed.
///
/// ```
/// use laminapdf_ops::{Op, RemoveXfa};
///
/// let op = Op::from(RemoveXfa::new());
/// assert_eq!(serde_json::to_string(&op)?, r#"{"op":"removeXfa","args":{}}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct RemoveXfa {}

impl RemoveXfa {
    /// The op that removes the form's XFA part.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    pub(crate) fn apply_to(doc: &mut laminapdf_model::Document) -> Result<Effect> {
        if !doc.form()?.has_xfa {
            return Err(Error::InvalidOp("removeXfa needs a form with an XFA part"));
        }
        edit_form(doc, |form| {
            form.remove("XFA");
        })?;
        if doc.catalog()?.contains_key("NeedsRendering") {
            set_catalog_entry(&mut doc.cos_mut().edit(), "NeedsRendering", None)?;
        }
        Ok(Effect::default())
    }
}
