//! Attached files (ISO 32000-2 §7.11.4 embedded file streams, §7.11.3 file specifications,
//! §14.13 associated files; ISO 19005-3 §6.8): each attachment is an `/EmbeddedFile` stream
//! with its MIME type and `/Params` (size, modification date, MD5 checksum), a file
//! specification with `/F`, `/UF` and `/AFRelationship`, listed in the catalog's
//! `/Names /EmbeddedFiles` tree and its `/AF` array (research 04 §6.2). `laminapdf-model`
//! writes the stream and the specification, as it does for the attachment ops.

use std::collections::BTreeMap;

use laminapdf_cos::{CosMut, Dict, ObjRef, Object};
pub use laminapdf_model::AfRelationship;
use laminapdf_model::{EmbeddedFileSpec, PdfDate, write_embedded_file};

use crate::error::Result;
use crate::text_string;

/// A file attached to the document ([`Builder::attach`](crate::Builder::attach)).
///
/// ```
/// use laminapdf_build::{AfRelationship, Attachment};
///
/// let xml = Attachment::new("factur-x.xml", b"<Invoice/>".to_vec(), "text/xml")
///     .with_relationship(AfRelationship::Alternative)
///     .with_description("The invoice as Factur-X XML");
/// assert_eq!(xml.name(), "factur-x.xml");
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    name: String,
    bytes: Vec<u8>,
    mime: String,
    relationship: AfRelationship,
    description: Option<String>,
    mod_date: Option<PdfDate>,
}

impl Attachment {
    /// The file `name` (its key in the attachments list and its `/F` and `/UF`), its bytes and
    /// its MIME type (`/Subtype`, such as `text/xml`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(name: impl Into<String>, bytes: Vec<u8>, mime: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bytes,
            mime: mime.into(),
            relationship: AfRelationship::default(),
            description: None,
            mod_date: None,
        }
    }

    /// How the file relates to the document (default [`AfRelationship::Unspecified`]).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_relationship(mut self, relationship: AfRelationship) -> Self {
        self.relationship = relationship;
        self
    }

    /// `/Desc`: a description viewers show next to the file.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// `/Params /ModDate`: when the file last changed (default: the document's creation
    /// date, [`BuildOptions::with_creation_date`](crate::BuildOptions::with_creation_date)).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_mod_date(mut self, date: PdfDate) -> Self {
        self.mod_date = Some(date);
        self
    }

    /// The file name.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The file's bytes.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The MIME type.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn mime(&self) -> &str {
        &self.mime
    }

    /// The relationship.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn relationship(&self) -> AfRelationship {
        self.relationship
    }
}

/// Writes every attachment and returns the `/EmbeddedFiles` name tree (one leaf, sorted by
/// name) and the file specifications for the catalog's `/AF`.
pub(crate) fn write(
    cos: &mut CosMut<'_>,
    files: &BTreeMap<String, Attachment>,
    created: Option<&PdfDate>,
) -> Result<(Dict, Vec<ObjRef>)> {
    let mut names = Vec::with_capacity(files.len() * 2);
    let mut specs = Vec::with_capacity(files.len());
    for (name, file) in files {
        let mut spec = EmbeddedFileSpec::new(name, &file.bytes, &file.mime);
        spec.description = file.description.as_deref();
        spec.modified = file.mod_date.as_ref().or(created);
        spec.relationship = file.relationship;
        let spec = write_embedded_file(cos, &spec)?;
        names.push(Object::String(text_string(name)));
        names.push(Object::Ref(spec));
        specs.push(spec);
    }
    let mut tree = Dict::new();
    tree.insert("Names", Object::Array(names));
    Ok((tree, specs))
}
