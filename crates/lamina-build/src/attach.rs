//! Attached files (ISO 32000-2 §7.11.4 embedded file streams, §7.11.3 file specifications,
//! §14.13 associated files; ISO 19005-3 §6.8): each attachment is an `/EmbeddedFile` stream
//! with its MIME type and `/Params`, a file specification with `/F`, `/UF` and
//! `/AFRelationship`, listed in the catalog's `/Names /EmbeddedFiles` tree and its `/AF` array
//! (research 04 §6.2).

use std::collections::BTreeMap;

use lamina_cos::{CosMut, Dict, Name, ObjRef, Object, PdfString, Stream};
use lamina_model::PdfDate;
use lamina_syntax::text::encode_text_string;

use crate::error::Result;

/// How an attached file relates to the document (`/AFRelationship`, ISO 32000-2 Table 43).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AfRelationship {
    /// The original the document was made from (a word-processor file).
    Source,
    /// Data the document's content was derived from (a chart's CSV).
    Data,
    /// Another representation of the same content (an e-invoice's XML).
    Alternative,
    /// A supplement to the content.
    Supplement,
    /// None of these (the default).
    #[default]
    Unspecified,
}

impl AfRelationship {
    /// The name written as `/AFRelationship`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Data => "Data",
            Self::Alternative => "Alternative",
            Self::Supplement => "Supplement",
            Self::Unspecified => "Unspecified",
        }
    }
}

/// A file attached to the document ([`Builder::attach`](crate::Builder::attach)).
///
/// ```
/// use lamina_build::{AfRelationship, Attachment};
///
/// let xml = Attachment::new("factur-x.xml", b"<Invoice/>".to_vec(), "text/xml")
///     .with_relationship(AfRelationship::Alternative)
///     .with_description("The invoice as Factur-X XML");
/// assert_eq!(xml.name(), "factur-x.xml");
/// ```
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
    #[must_use]
    pub const fn with_relationship(mut self, relationship: AfRelationship) -> Self {
        self.relationship = relationship;
        self
    }

    /// `/Desc`: a description viewers show next to the file.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// `/Params /ModDate`: when the file last changed (default: the document's creation
    /// date, [`BuildOptions::with_creation_date`](crate::BuildOptions::with_creation_date)).
    #[must_use]
    pub const fn with_mod_date(mut self, date: PdfDate) -> Self {
        self.mod_date = Some(date);
        self
    }

    /// The file name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The file's bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The MIME type.
    #[must_use]
    pub fn mime(&self) -> &str {
        &self.mime
    }

    /// The relationship.
    #[must_use]
    pub const fn relationship(&self) -> AfRelationship {
        self.relationship
    }
}

fn text(s: &str) -> PdfString {
    PdfString::literal(encode_text_string(s))
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
        let mut params = Dict::new();
        params.insert("Size", i64::try_from(file.bytes.len()).unwrap_or(i64::MAX));
        if let Some(date) = file.mod_date.as_ref().or(created) {
            params.insert("ModDate", PdfString::literal(date.to_string()));
        }
        let mut stream = Dict::new();
        stream.insert("Type", Name::from("EmbeddedFile"));
        if !file.mime.is_empty() {
            stream.insert("Subtype", Name::new(file.mime.clone()));
        }
        stream.insert("Params", params);
        let data = cos.create(Object::Stream(Stream::flate(stream, &file.bytes)))?;
        let mut ef = Dict::new();
        ef.insert("F", data);
        ef.insert("UF", data);
        let mut spec = Dict::new();
        spec.insert("Type", Name::from("Filespec"));
        spec.insert("F", text(name));
        spec.insert("UF", text(name));
        spec.insert("EF", ef);
        if let Some(desc) = &file.description {
            spec.insert("Desc", text(desc));
        }
        spec.insert("AFRelationship", Name::from(file.relationship.as_str()));
        let spec = cos.create(Object::Dict(spec))?;
        names.push(Object::String(text(name)));
        names.push(Object::Ref(spec));
        specs.push(spec);
    }
    let mut tree = Dict::new();
    tree.insert("Names", Object::Array(names));
    Ok((tree, specs))
}
