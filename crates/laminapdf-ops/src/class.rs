//! What kind of change an op makes, for the save planner (ADR 0006 §1, ADR 0007).

/// The kind of change an op makes. The save planner checks it against `DocMDP` / `FieldMDP`
/// permissions and the document's `SaveConstraint` ([ADR 0007]).
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
///
/// Added in 0.2.0.
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

/// The usage right an annotation op needs: the name a UR3 signature's `/Annots` array must
/// list for the op to stay within the rights it grants (ISO 32000-2 §12.8.2.3, Table 260).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum AnnotRight {
    /// `/Create`: adding an annotation (a reply or a review state is a new one too).
    Create,
    /// `/Modify`: changing one.
    Modify,
    /// `/Delete`: removing one.
    Delete,
}
