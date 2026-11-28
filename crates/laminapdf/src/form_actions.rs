//! A form's actions and the sends they ask for (E8 plan §2.3): what a push button does when it is
//! pressed, read as typed data, and the request a `SubmitForm` action would make, built and not
//! made. Lamina PDF runs no action and sends nothing; the reader asks first.

use laminapdf_interchange as ix;
use laminapdf_model::{FieldAction, FieldId, SubmitForm};

use crate::document::Document;
use crate::error::{Error, Result};

pub use ix::{SubmitFormat, SubmitMethod, SubmitRequest, SubmitTarget, SubmittedField};

impl Document {
    /// What `field` does when a person presses it or moves the pointer over it: its `/A` and
    /// `/AA` actions (not the scripts of its [behaviour](Document::field_behaviour)) through
    /// `/Next`, as typed data. Nothing runs.
    ///
    /// ```
    /// use laminapdf::{Document, FormAction, OpenOptions};
    ///
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/forms/actions.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let form = doc.form()?;
    /// let button = form.fields.iter().find(|f| f.name == "web").expect("a button");
    /// let actions = doc.form_actions(&button.id.parse()?)?;
    /// assert_eq!(actions[0].action, FormAction::Uri("https://example.com/help".into()));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `unknown-field`; reading errors.
    ///
    /// Added in 0.7.0.
    pub fn form_actions(&self, field: &FieldId) -> Result<Vec<FieldAction>> {
        let doc = self.ops();
        let form = doc.form()?;
        let field = form
            .field(field)
            .ok_or_else(|| Error::from(laminapdf_ops::Error::UnknownField(field.clone())))?;
        Ok(doc.form_actions(field)?)
    }

    /// The request `action` would make on the form as it is now: the address, the format, the
    /// body, the fields in it and the required fields still empty. Nothing is sent and nothing
    /// changes. The body carries field values only: never the document, a password field, a
    /// field flagged `NoExport` or, in FDF and XFDF, the file's name.
    ///
    /// # Errors
    /// `submit-pdf-refused` for an action that would send the whole PDF; `submit-url-refused`
    /// for an address that isn't `https:` (a `mailto:` address is a request that sends no data);
    /// reading errors.
    ///
    /// Added in 0.7.0.
    pub fn submit_request(&self, action: &SubmitForm) -> Result<SubmitRequest> {
        Ok(ix::submit_request(&self.ops().form()?, action)?)
    }
}
