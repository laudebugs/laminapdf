//! What kind of change an op makes, for the save planner (ADR 0006 §1, ADR 0007).

/// The kind of change an op makes. The save planner checks it against `DocMDP` / `FieldMDP`
/// permissions and the document's `SaveConstraint` ([ADR 0007]).
///
/// [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum ModificationClass {
    /// Document metadata (Info, XMP).
    Metadata,
    /// Filling form fields.
    FormFill,
    /// Adding, changing or removing annotations.
    Annotation,
    /// Signing.
    Signature,
    /// Page order, rotation, insertion, deletion.
    PageStructure,
    /// Page content.
    Content,
    /// Document-wide structure (encryption, outline, catalog entries).
    Document,
}
