//! Form data (E8 plan §2.1): answers into a form from JSON, XFDF or FDF, out of it in the same
//! formats, and back to the form's defaults. Each import, fill and reset is one transaction of
//! [`SetFieldValue`](crate::SetFieldValue) ops, one undo step, with the form's calculations run
//! once at the end.

use laminapdf_interchange as ix;
use laminapdf_model::FieldId;

use crate::document::{Document, Edited};
use crate::error::Result;

pub use ix::{FORM_DATA_SCHEMA, FormDataFormat};

/// How to fill a form.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FillOptions {
    /// Whether the form's calculations run once after the last value (true unless turned off).
    pub calculate: bool,
}

impl Default for FillOptions {
    fn default() -> Self {
        Self { calculate: true }
    }
}

impl FillOptions {
    /// The same options with calculations on or off.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_calculate(mut self, calculate: bool) -> Self {
        self.calculate = calculate;
        self
    }
}

/// What filling, importing or resetting a form did.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FormDataReport {
    /// The transaction: one undo step for every value set and every total calculated.
    pub edited: Edited,
    /// The names in the data that no field of the form has.
    pub unknown: Vec<String>,
    /// The fields a signature's lock covers, by name, left as they are.
    pub locked: Vec<String>,
    /// The fields whose value was refused, by name, each with its code.
    pub refused: Vec<RefusedField>,
    /// Whether the form has an XFA part, which keeps its old answers (`xfa-not-updated`).
    /// Adobe's apps show the XFA answers until the XFA part is removed.
    pub xfa_not_updated: bool,
}

/// A field a fill left as it was, and why.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RefusedField {
    /// The field's fully qualified name.
    pub name: String,
    /// Why: `field-is-signature`, `field-value-invalid` (a value of the wrong kind, too long,
    /// or not among a list's options) or `annotation-glyph-missing` (a character the field's
    /// font can't draw).
    pub code: &'static str,
}

impl Document {
    /// Fills the form from Lamina PDF's form data JSON: `{ "fields": { "<fully qualified name>":
    /// value } }`, where a value is a string, `true` or `false` for a check box with one
    /// on-state, an array of strings for a list box, or `null` for empty ([`FORM_DATA_SCHEMA`]).
    /// It is the object [`Document::export_form_data`] writes, so an export fills a form as it
    /// was. A button takes its on-state's name or export value, a list its export values.
    ///
    /// The values go in as one transaction, and the form's calculations run once at the end
    /// unless [`FillOptions::calculate`] is off. A name no field has, a field a signature's lock
    /// covers and a value its field refuses are each listed in the report and left out; the
    /// rest are filled.
    ///
    /// ```
    /// use laminapdf::{Document, FieldValue, FillOptions, OpenOptions};
    ///
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/forms/fields.pdf"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let report = doc.fill_form(r#"{ "fields": { "name": "Grace Hopper", "agree": false } }"#, FillOptions::default())?;
    /// assert!(report.unknown.is_empty() && report.refused.is_empty());
    /// let name = doc.form()?.fields.into_iter().find(|f| f.name == "name").and_then(|f| f.value);
    /// assert_eq!(name, Some(FieldValue::Text("Grace Hopper".into())));
    /// doc.undo()?; // every value, in one step
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `interchange-unknown-format` for text that isn't a JSON object; `interchange-malformed`
    /// (naming the line) for one that isn't form data;
    /// `interchange-too-large`; `appearance-unavailable` when a text or list value needs an
    /// appearance and the document can't draw one; reading errors. Nothing changes on error.
    ///
    /// Added in 0.7.0.
    pub fn fill_form(&mut self, json: &str, options: FillOptions) -> Result<FormDataReport> {
        if ix::form_data_format(json.as_bytes()) != Some(FormDataFormat::Json) {
            return Err(ix::InterchangeError::UnknownFormat.into());
        }
        self.import_form_data(json.as_bytes(), options)
    }

    /// Fills the form from a form data file: XFDF (ISO 19444-1), FDF (ISO 32000-2 §12.7.8) or
    /// Lamina PDF's JSON, told apart by their first bytes, matching fields by fully qualified
    /// name. As [`Document::fill_form`] otherwise; an XFDF or FDF file's comments are left for
    /// [`Document::import_annotations`].
    ///
    /// # Errors
    /// `interchange-unknown-format`, `interchange-malformed`, `interchange-dtd` and
    /// `interchange-too-large` for a file that can't be read; as [`Document::fill_form`].
    ///
    /// Added in 0.7.0.
    pub fn import_form_data(&mut self, bytes: &[u8], options: FillOptions) -> Result<FormDataReport> {
        let fields = ix::read_form_data(bytes, self.ops().cos().limits())?;
        let form = self.ops().form()?;
        let plan = ix::plan_form_data(&form, &fields);
        let filled = self.ops_mut().fill(plan.values, options.calculate)?;
        Ok(self.report(&form, plan.unknown, &plan.refused, filled))
    }

    /// The form's values as a form data file in `format`, which [`Document::import_form_data`]
    /// reads back, and which other PDF apps read as XFDF or FDF. Every field but push buttons
    /// and signature fields is written, an empty one too. XFDF and FDF carry `file` as the PDF's
    /// name when it is given, and its `/ID`, as those formats do. Nothing in the document changes.
    ///
    /// ```
    /// use laminapdf::{Document, FormDataFormat, OpenOptions};
    ///
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/forms/fields.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let json = String::from_utf8(doc.export_form_data(FormDataFormat::Json, None)?)?;
    /// assert!(json.contains(r#""name": "Ada Lovelace""#));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Reading errors.
    ///
    /// Added in 0.7.0.
    pub fn export_form_data(&self, format: FormDataFormat, file: Option<&str>) -> Result<Vec<u8>> {
        let doc = self.ops();
        Ok(ix::form_data_of(&doc.form()?, format, file, ix::file_ids(doc)))
    }

    /// Gives `fields` (every field but push buttons and signature fields when `None`) their
    /// default values (`/DV`), or empties those without one, as a reset-form action does
    /// (ISO 32000-2 §12.7.6.3). One transaction, with calculations run once at the end; a
    /// field a signature's lock covers is skipped and listed.
    ///
    /// # Errors
    /// `unknown-field` for an ID the form doesn't have; `field-is-signature` for a signature
    /// field; as [`Document::fill_form`]. Nothing changes on error.
    ///
    /// Added in 0.7.0.
    pub fn reset_fields(&mut self, fields: Option<&[FieldId]>) -> Result<FormDataReport> {
        let form = self.ops().form()?;
        let values = ix::reset_values(&form, fields)?;
        let filled = self.ops_mut().fill(values, true)?;
        Ok(self.report(&form, Vec::new(), &[], filled))
    }

    /// The report of `filled` on `form`, after the names the data had that no field has and
    /// the values refused before anything was applied.
    fn report(
        &mut self,
        form: &laminapdf_model::Form,
        unknown: Vec<String>,
        refused_first: &[(FieldId, laminapdf_ops::Error)],
        filled: laminapdf_ops::Filled,
    ) -> FormDataReport {
        let refused = refused_first
            .iter()
            .chain(&filled.refused)
            .map(|(id, e)| RefusedField {
                name: name_of(form, id),
                code: e.code(),
            })
            .collect();
        let edited = self.edited(filled.applied);
        FormDataReport {
            xfa_not_updated: form.has_xfa && edited.changed,
            edited,
            unknown,
            locked: filled.locked.iter().map(|id| name_of(form, id)).collect(),
            refused,
        }
    }
}

/// `id`'s fully qualified name, or the ID itself for a field the form no longer lists.
fn name_of(form: &laminapdf_model::Form, id: &FieldId) -> String {
    form.field(id).map_or_else(|| id.to_string(), |f| f.name.clone())
}
